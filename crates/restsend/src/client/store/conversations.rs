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
use std::sync::Arc;

pub(crate) fn merge_conversation(
    message_storage: Arc<Storage>,
    conversation: Conversation,
) -> Result<Conversation> {
    let t = message_storage.table::<Conversation>("conversations");
    let mut conversation = conversation;

    if let Some(old_conversation) = t.get("", &conversation.topic_id) {
        conversation.last_read_seq = old_conversation.last_read_seq;
        conversation.last_sender_id = old_conversation.last_sender_id;
        conversation.last_message_at = old_conversation.last_message_at;
        conversation.last_message = old_conversation.last_message;
        conversation.unread = old_conversation.unread;
    }

    conversation.is_partial = false;
    conversation.cached_at = now_millis();
    t.set("", &conversation.topic_id, Some(&conversation));
    Ok(conversation)
}

pub(super) fn merge_conversation_from_chat(
    message_storage: Arc<Storage>,
    req: &ChatRequest,
) -> Result<Conversation> {
    let t = message_storage.table::<Conversation>("conversations");
    let mut conversation = t.get("", &req.topic_id).unwrap_or(Conversation::from(req));
    if let Some(content) = req.content.as_ref() {
        match ContentType::from(content.r#type.clone()) {
            ContentType::None | ContentType::Recall => {}
            _ => {
                if req.seq > conversation.last_read_seq {
                    conversation.unread += 1;
                }
            }
        }
    }

    if req.seq >= conversation.last_seq && !req.unreadable {
        conversation.last_seq = req.seq;
        conversation.last_sender_id = req.attendee.clone();
        conversation.last_message_at = req.created_at.clone();
        conversation.last_message = req.content.clone();
        conversation.updated_at = req.created_at.clone();
    }

    conversation.cached_at = now_millis();
    t.set("", &conversation.topic_id, Some(&conversation));
    Ok(conversation)
}

impl ClientStore {
    pub(crate) fn merge_conversations(
        &self,
        conversations: Vec<Conversation>,
    ) -> Vec<Conversation> {
        let mut result = vec![];
        for conversation in conversations {
            let conversation = match merge_conversation(self.message_storage.clone(), conversation)
            {
                Ok(c) => c,
                Err(e) => {
                    warn!("merge_conversation failed: {:?}", e);
                    continue;
                }
            };
            result.push(conversation);
        }
        result
    }

    pub fn emit_conversation_update(&self, conversation: Conversation) -> Result<Conversation> {
        if let Some(cb) = self.callback.lock().unwrap().as_ref() {
            cb.on_conversations_updated(vec![conversation.clone()]);
        }
        Ok(conversation)
    }

    pub async fn set_conversation_remark(
        &self,
        topic_id: &str,
        remark: Option<String>,
    ) -> Result<Conversation> {
        {
            let t = self.message_storage.table::<Conversation>("conversations");
            if let Some(mut conversation) = t.get("", topic_id) {
                conversation.remark = remark.clone();
                t.set("", topic_id, Some(&conversation));
            }
        }

        set_conversation_remark(&self.endpoint, &self.token, &topic_id, remark)
            .await
            .and_then(|c| merge_conversation(self.message_storage.clone(), c))
            .and_then(|c| self.emit_conversation_update(c))
    }

    pub async fn set_conversation_sticky(
        &self,
        topic_id: &str,
        sticky: bool,
    ) -> Result<Conversation> {
        {
            let t = self.message_storage.table::<Conversation>("conversations");
            if let Some(mut conversation) = t.get("", topic_id) {
                conversation.sticky = sticky;
                t.set("", topic_id, Some(&conversation));
            }
        }

        set_conversation_sticky(&self.endpoint, &self.token, &topic_id, sticky)
            .await
            .and_then(|c| merge_conversation(self.message_storage.clone(), c))
            .and_then(|c| self.emit_conversation_update(c))
    }

    pub async fn set_conversation_mute(&self, topic_id: &str, mute: bool) -> Result<Conversation> {
        {
            let t = self.message_storage.table::<Conversation>("conversations");
            if let Some(mut conversation) = t.get("", topic_id) {
                conversation.mute = mute;
                t.set("", topic_id, Some(&conversation));
            }
        }

        set_conversation_mute(&self.endpoint, &self.token, &topic_id, mute)
            .await
            .and_then(|c| merge_conversation(self.message_storage.clone(), c))
            .and_then(|c| self.emit_conversation_update(c))
    }

    pub fn set_conversation_read_local(&self, topic_id: &str) -> Option<Conversation> {
        let t = self.message_storage.table::<Conversation>("conversations");
        match t.get("", topic_id) {
            Some(mut conversation) => {
                conversation.last_read_seq = conversation.last_seq;
                conversation.unread = 0;
                t.set("", topic_id, Some(&conversation));
                Some(conversation)
            }
            _ => None,
        }
    }

    pub async fn set_conversation_read(&self, topic_id: &str) {
        let conversation = self.set_conversation_read_local(topic_id);
        match set_conversation_read(&self.endpoint, &self.token, &topic_id).await {
            Ok(_) => match conversation {
                Some(conversation) => self.emit_conversation_update(conversation).ok(),
                None => None,
            },
            Err(e) => {
                warn!("set_conversation_read failed: {:?}", e);
                None
            }
        };
    }

    pub async fn set_conversation_tags(
        &self,
        topic_id: &str,
        tags: Option<Tags>,
    ) -> Result<Conversation> {
        {
            let t = self.message_storage.table::<Conversation>("conversations");
            if let Some(mut conversation) = t.get("", topic_id) {
                conversation.tags = tags.clone();
                t.set("", topic_id, Some(&conversation));
            }
        }

        let values = serde_json::json!({
            "tags": tags.unwrap_or_default(),
        });

        update_conversation(&self.endpoint, &self.token, &topic_id, &values)
            .await
            .and_then(|c| merge_conversation(self.message_storage.clone(), c))
            .and_then(|c| self.emit_conversation_update(c))
    }

    pub async fn set_conversation_extra(
        &self,
        topic_id: &str,
        extra: Option<Extra>,
    ) -> Result<Conversation> {
        {
            let t = self.message_storage.table::<Conversation>("conversations");
            if let Some(mut conversation) = t.get("", topic_id) {
                conversation.extra = extra.clone();
                t.set("", topic_id, Some(&conversation));
            }
        }

        let values = serde_json::json!({
            "extra": extra.unwrap_or_default(),
        });

        update_conversation(&self.endpoint, &self.token, &topic_id, &values)
            .await
            .and_then(|c| merge_conversation(self.message_storage.clone(), c))
            .and_then(|c| self.emit_conversation_update(c))
    }

    pub(crate) async fn remove_conversation(&self, topic_id: &str) {
        {
            let t = self.message_storage.table::<Conversation>("conversations");
            t.remove("", topic_id);
        }

        match remove_conversation(&self.endpoint, &self.token, &topic_id).await {
            Ok(_) => {
                if let Some(cb) = self.callback.lock().unwrap().as_ref() {
                    cb.on_conversation_removed(topic_id.to_string());
                }
            }
            Err(e) => {
                warn!("remove_conversation failed: {:?}", e);
            }
        }
    }

    pub(crate) fn get_conversation(&self, topic_id: &str) -> Option<Conversation> {
        let t = self.message_storage.table::<Conversation>("conversations");
        let conversation = t.get("", topic_id).unwrap_or(Conversation::new(topic_id));

        if conversation.is_partial
            || is_cache_expired(conversation.cached_at, CONVERSATION_CACHE_EXPIRE_SECS)
        {
            self.fetch_conversation(topic_id);
        }
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
                Ok(c) => {
                    let c = match merge_conversation(message_storage, c) {
                        Ok(c) => c,
                        Err(e) => {
                            warn!("update_conversation_with_storage failed: {:?}", e);
                            return;
                        }
                    };
                    if let Some(cb) = callback.lock().unwrap().as_ref() {
                        cb.on_conversations_updated(vec![c]);
                    };
                }
                Err(e) => {
                    warn!("get_conversation failed: {:?}", e);
                    return;
                }
            };
        });
    }

    pub(super) fn update_conversation_read(
        &self,
        topic_id: &str,
        updated_at: &str,
        last_read_seq: Option<i64>,
    ) -> Result<()> {
        let t = self.message_storage.table::<Conversation>("conversations");

        if let Some(mut conversation) = t.get("", topic_id) {
            conversation.last_read_seq = last_read_seq.unwrap_or(conversation.last_seq);
            conversation.updated_at = updated_at.to_string();
            t.set("", topic_id, Some(&conversation));
        }
        Ok(())
    }

    pub(super) fn save_outgoing_chat_log(&self, req: &ChatRequest) -> Result<()> {
        let t = self.message_storage.table::<ChatLog>("chat_logs");

        let mut log = ChatLog::from(req);
        log.status = ChatLogStatus::Sending;
        log.sender_id = self.user_id.clone();
        t.set(&log.topic_id, &log.id, Some(&log));

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
            t.set(topic_id, chat_id, Some(&log));
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
                t.set(&topic_id, &chat_id, Some(&log));
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
        t.set(&log.topic_id, &log.id, Some(&log));
        Ok(())
    }

    pub(crate) fn save_chat_log(&self, chat_log: &ChatLog) -> Result<()> {
        let t = self.message_storage.table("chat_logs");

        if let Some(_) = t.get(&chat_log.topic_id, &chat_log.id) {
            return Ok(());
        }

        let item = match ContentType::from(chat_log.content.r#type.to_string()) {
            ContentType::None => None, // remove local log
            _ => Some(chat_log),
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

    pub fn filter_conversation(
        &self,
        predicate: Box<dyn Fn(Conversation) -> Option<Conversation>>,
    ) -> Vec<Conversation> {
        let t = self.message_storage.table("conversations");
        t.filter(
            "",
            Box::new(move |c| {
                let mut c = c;
                c.unread = (c.last_seq - c.last_read_seq).max(0);
                predicate(c)
            }),
        )
    }

    pub fn get_last_conversation_updated_at(&self) -> Option<String> {
        let t = self.message_storage.table::<Conversation>("conversations");
        let conversation = t.last("");
        match conversation {
            Some(conversation) => {
                if let Ok(t) = chrono::DateTime::parse_from_rfc3339(&conversation.updated_at) {
                    if t.timestamp_millis() > 0
                        && now_millis() - t.timestamp_millis()
                            < 1000 * crate::CONVERSATION_CACHE_EXPIRE_SECS
                    {
                        return Some(conversation.updated_at);
                    }
                }
            }
            None => {}
        }
        None
    }
}
