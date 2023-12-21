use std::sync::Arc;

use super::{is_cache_expired, ClientStore};
use crate::{
    models::{
        conversation::{Extra, Tags},
        ChatLog, ChatLogStatus, Content, ContentType, Conversation,
    },
    request::ChatRequest,
    services::conversation::*,
    storage::{QueryOption, QueryResult, Storage},
    utils::{now_millis, spwan_task},
    CONVERSATION_CACHE_EXPIRE_SECS, MAX_RECALL_SECS,
};
use crate::{Error, Result};
use log::{debug, warn};

pub(crate) fn update_conversation_with_storage(
    message_storage: Arc<Storage>,
    conversation: Conversation,
) -> Result<Conversation> {
    let t = message_storage.table::<Conversation>("conversations");

    let topic_id = conversation.topic_id.clone();
    let mut conversation = conversation;
    if let Some(old_conversation) = t.get("", &topic_id) {
        warn!(
            " old_conversation.last_seq:{} conversation.last_seq:{}",
            old_conversation.last_seq, conversation.last_seq
        );
        if old_conversation.last_seq <= conversation.last_seq {
            conversation.last_read_seq = old_conversation.last_read_seq;
            conversation.last_sender_id = old_conversation.last_sender_id.clone();
            conversation.last_message_at = old_conversation.last_message_at.clone();
            conversation.last_message = old_conversation.last_message.clone();

            // TODO: update other fields
            conversation.multiple = old_conversation.multiple;
            conversation.mute = old_conversation.mute;
            conversation.sticky = old_conversation.sticky;
        }
    }

    conversation.is_partial = false;
    conversation.cached_at = now_millis();
    conversation.unread = (conversation.last_seq - conversation.last_read_seq).max(0);

    t.set("", &topic_id, Some(conversation.clone()));
    Ok(conversation)
}

impl ClientStore {
    pub fn emit_conversation_update(&self, topic_id: &str) {
        let t = self.message_storage.table::<Conversation>("conversations");
        if let Some(conversation) = t.get("", topic_id) {
            self.emit_conversations_update(vec![conversation]);
        }
    }

    pub fn emit_conversations_update(&self, conversations: Vec<Conversation>) {
        let mut conversations = conversations;
        conversations.iter_mut().for_each(|c| {
            c.unread = (c.last_seq - c.last_read_seq).max(0);
        });

        self.callback
            .lock()
            .unwrap()
            .as_ref()
            .map(|cb| cb.on_conversations_updated(conversations));
    }

    pub async fn set_conversation_sticky(&self, topic_id: &str, sticky: bool) {
        {
            let t = self.message_storage.table::<Conversation>("conversations");
            if let Some(mut conversation) = t.get("", topic_id) {
                conversation.sticky = sticky;
                t.set("", topic_id, Some(conversation));
            }
        }

        match set_conversation_sticky(&self.endpoint, &self.token, &topic_id, sticky).await {
            Ok(_) => self.emit_conversation_update(topic_id),
            Err(e) => {
                warn!("set_conversation_sticky failed: {:?}", e);
            }
        }
    }

    pub async fn set_conversation_mute(&self, topic_id: &str, mute: bool) {
        {
            let t = self.message_storage.table::<Conversation>("conversations");
            if let Some(mut conversation) = t.get("", topic_id) {
                conversation.mute = mute;
                t.set("", topic_id, Some(conversation));
            }
        }

        match set_conversation_mute(&self.endpoint, &self.token, &topic_id, mute).await {
            Ok(_) => self.emit_conversation_update(topic_id),
            Err(e) => {
                warn!("set_conversation_sticky failed: {:?}", e);
            }
        }
    }

    pub fn set_conversation_read_local(&self, topic_id: &str) {
        let t = self.message_storage.table::<Conversation>("conversations");
        if let Some(mut conversation) = t.get("", topic_id) {
            conversation.last_read_seq = conversation.last_seq;
            t.set("", topic_id, Some(conversation));
        }
    }

    pub async fn set_conversation_read(&self, topic_id: &str) {
        self.set_conversation_read_local(topic_id);
        match set_conversation_read(&self.endpoint, &self.token, &topic_id).await {
            Ok(_) => self.emit_conversation_update(topic_id),
            Err(e) => {
                warn!("set_conversation_read failed: {:?}", e);
            }
        }
    }

    pub async fn set_conversation_tags(&self, topic_id: &str, tags: Option<Tags>) {
        {
            let t = self.message_storage.table::<Conversation>("conversations");
            if let Some(mut conversation) = t.get("", topic_id) {
                conversation.tags = tags.clone();
                t.set("", topic_id, Some(conversation));
            }
        }

        let values = serde_json::json!({
            "tags": tags.unwrap_or_default(),
        });

        match update_conversation(&self.endpoint, &self.token, &topic_id, &values).await {
            Ok(_) => self.emit_conversation_update(topic_id),
            Err(e) => {
                warn!("set_conversation_tags failed: {:?}", e);
            }
        }
    }

    pub async fn set_conversation_extra(&self, topic_id: &str, extra: Option<Extra>) {
        {
            let t = self.message_storage.table::<Conversation>("conversations");
            if let Some(mut conversation) = t.get("", topic_id) {
                conversation.extra = extra.clone();
                t.set("", topic_id, Some(conversation));
            }
        }

        let values = serde_json::json!({
            "extra": extra.unwrap_or_default(),
        });

        match update_conversation(&self.endpoint, &self.token, &topic_id, &values).await {
            Ok(_) => self.emit_conversation_update(topic_id),
            Err(e) => {
                warn!("set_conversation_extra failed: {:?}", e);
            }
        }
    }

    pub(crate) async fn remove_conversation(&self, topic_id: &str) {
        {
            let t = self.message_storage.table::<Conversation>("conversations");
            t.remove("", topic_id);
        }

        match remove_conversation(&self.endpoint, &self.token, &topic_id).await {
            Ok(_) => {
                self.callback
                    .lock()
                    .unwrap()
                    .as_ref()
                    .map(|cb| cb.on_conversation_removed(topic_id.to_string()));
            }
            Err(e) => {
                warn!("remove_conversation failed: {:?}", e);
            }
        }
    }

    pub(crate) fn get_conversation(&self, topic_id: &str) -> Option<Conversation> {
        let t = self.message_storage.table::<Conversation>("conversations");
        let mut conversation = t.get("", topic_id).unwrap_or(Conversation::new(topic_id));

        if conversation.is_partial
            || is_cache_expired(conversation.cached_at, CONVERSATION_CACHE_EXPIRE_SECS)
        {
            self.fetch_conversation(topic_id);
        }
        conversation.unread = (conversation.last_seq - conversation.last_read_seq).max(0);
        Some(conversation)
    }

    pub(super) fn fetch_conversation(&self, topic_id: &str) {
        let endpoint = self.endpoint.clone();
        let token = self.token.clone();
        let topic_id = topic_id.to_string();
        let message_storage = self.message_storage.clone();
        let callback = self.callback.clone();

        spwan_task(async move {
            match get_conversation(&endpoint, &token, &topic_id).await {
                Ok(conversation) => {
                    let conversations = vec![update_conversation_with_storage(
                        message_storage,
                        conversation.clone(),
                    )
                    .unwrap_or(conversation)];

                    callback
                        .lock()
                        .unwrap()
                        .as_ref()
                        .map(|cb| cb.on_conversations_updated(conversations));
                }
                Err(e) => {
                    warn!("fetch_conversation failed id:{} err: {:?}", topic_id, e);
                }
            }
        });
    }

    pub(crate) fn update_conversation(&self, conversation: Conversation) -> Result<Conversation> {
        let t = self.message_storage.table::<Conversation>("conversations");

        let topic_id = conversation.topic_id.clone();
        let mut conversation = conversation;
        if let Some(old_conversation) = t.get("", &topic_id) {
            if old_conversation.last_seq <= conversation.last_seq {
                conversation.last_read_seq = old_conversation.last_read_seq;
                conversation.last_sender_id = old_conversation.last_sender_id.clone();
                conversation.last_message_at = old_conversation.last_message_at.clone();
                conversation.last_message = old_conversation.last_message.clone();

                // TODO: update other fields
                conversation.multiple = old_conversation.multiple;
                conversation.mute = old_conversation.mute;
                conversation.sticky = old_conversation.sticky;
            }
        }

        conversation.is_partial = false;
        conversation.cached_at = now_millis();
        conversation.unread = (conversation.last_seq - conversation.last_read_seq).max(0);

        t.set("", &topic_id, Some(conversation.clone()));
        Ok(conversation)
    }

    pub(super) fn update_conversation_from_chat(&self, req: &ChatRequest) -> Result<Conversation> {
        let topic_id = &req.topic_id;
        let t = self.message_storage.table::<Conversation>("conversations");

        let mut conversation = t.get("", &topic_id).unwrap_or(Conversation::from(req));

        if req.seq > 0 && req.seq >= conversation.last_seq {
            conversation.last_seq = req.seq;
            conversation.last_sender_id = req.attendee.clone();
            conversation.last_message_at = req.created_at.clone();
            conversation.last_message = req.content.clone();
            conversation.cached_at = now_millis();
            conversation.unread = (conversation.last_seq - conversation.last_read_seq).max(0);
            conversation.updated_at = req.created_at.clone();
        }

        Ok(conversation)
    }

    pub(super) fn update_conversation_read(&self, topic_id: &str, updated_at: &str) -> Result<()> {
        let t = self.message_storage.table::<Conversation>("conversations");

        if let Some(mut conversation) = t.get("", topic_id) {
            conversation.last_read_seq = conversation.last_seq;
            conversation.updated_at = updated_at.to_string();
            t.set("", topic_id, Some(conversation));
        }
        Ok(())
    }

    pub(super) fn save_outgoing_chat_log(&self, req: &ChatRequest) -> Result<()> {
        let t = self.message_storage.table::<ChatLog>("chat_logs");

        let mut log = ChatLog::from(req);
        log.status = ChatLogStatus::Sending;
        log.sender_id = self.user_id.clone();
        t.set(&log.topic_id, &log.id, Some(log.clone()));

        Ok(())
    }

    pub(super) fn update_outoing_chat_log_state(
        &self,
        topic_id: &str,
        chat_id: &str,
        status: ChatLogStatus,
        seq: Option<i64>,
    ) -> Result<()> {
        let t = self.message_storage.table::<ChatLog>("chat_logs");

        if let Some(log) = t.get(topic_id, chat_id) {
            let mut log = log.clone();
            log.status = status;
            if let Some(seq) = seq {
                log.seq = seq;
            }
            debug!(
                "update_outoing_chat_log_state: topic_id: {} chat_id: {} status: {:?} seq: {:?}",
                topic_id, chat_id, log.status, seq
            );
            t.set(topic_id, chat_id, Some(log));
        }
        Ok(())
    }

    pub(super) fn save_incoming_chat_log(&self, req: &ChatRequest) -> Result<()> {
        let t = self.message_storage.table::<ChatLog>("chat_logs");
        let topic_id = &req.topic_id;
        let chat_id = &req.chat_id;
        let now = now_millis();
        let mut new_status = ChatLogStatus::Received;
        if let Some(old_log) = t.get(&topic_id, &chat_id) {
            if req.r#type == "recall" {
                if MAX_RECALL_SECS > 0 && now - old_log.cached_at > MAX_RECALL_SECS {
                    return Err(Error::Other("[recall] timeout".to_string()));
                }

                match old_log.status {
                    ChatLogStatus::Received => {}
                    _ => return Err(Error::Other("[recall] invalid status".to_string())),
                }

                if req.attendee != old_log.sender_id {
                    return Err(Error::Other("[recall] invalid owner".to_string()));
                }

                let mut log = old_log.clone();
                log.recall = true;
                log.content = Content::new(ContentType::Recall);
                t.set(&topic_id, &chat_id, Some(log));
            }
            match old_log.status {
                ChatLogStatus::Sending => new_status = ChatLogStatus::Sent,
                _ => return Ok(()),
            }
        }

        let mut log = ChatLog::from(req);
        log.status = new_status;
        log.cached_at = now;

        debug!(
            "save_incoming_chat_log: topic_id: {} chat_id: {} seq: {}",
            topic_id, chat_id, req.seq,
        );
        t.set(&log.topic_id, &log.id, Some(log.clone()));
        Ok(())
    }

    pub(crate) fn save_chat_log(&self, chat_log: &ChatLog) -> Result<()> {
        let t = self.message_storage.table("chat_logs");

        if let Some(_) = t.get(&chat_log.topic_id, &chat_log.id) {
            return Ok(());
        }

        let item = match ContentType::from(chat_log.content.r#type.to_string()) {
            ContentType::None => None, // remove local log
            _ => Some(chat_log.clone()),
        };

        t.set(&chat_log.topic_id, &chat_log.id, item);
        Ok(())
    }

    pub fn get_chat_logs(
        &self,
        topic_id: &str,
        seq: i64,
        limit: u32,
    ) -> Result<QueryResult<ChatLog>> {
        let t = self.message_storage.table("chat_logs");

        let start_sort_value = (seq - limit as i64).max(0);
        let option = QueryOption {
            keyword: None,
            start_sort_value,
            limit,
        };
        Ok(t.query(topic_id, &option))
    }

    pub fn get_chat_log(&self, topic_id: &str, chat_id: &str) -> Option<ChatLog> {
        let t = self.message_storage.table("chat_logs");
        t.get(topic_id, chat_id)
    }

    pub fn remove_messages(&self, topic_id: &str, chat_ids: &[String]) {
        let t = self.message_storage.table::<ChatLog>("chat_logs");
        for chat_id in chat_ids {
            t.remove(topic_id, chat_id);
        }
    }

    pub fn get_conversations(
        &self,
        updated_at: &str,
        limit: u32,
    ) -> Result<QueryResult<Conversation>> {
        let t = self.message_storage.table("conversations");

        let start_sort_value = chrono::DateTime::parse_from_rfc3339(updated_at)
            .map(|v| v.timestamp_millis())
            .unwrap_or(0);

        let option = QueryOption {
            keyword: None,
            start_sort_value,
            limit,
        };
        Ok(t.query("", &option))
    }
}
