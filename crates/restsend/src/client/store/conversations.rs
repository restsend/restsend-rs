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
use log::warn;
use std::sync::Arc;

pub(crate) async fn merge_conversation(
    message_storage: Arc<Storage>,
    conversation: Conversation,
) -> Result<Conversation> {
    let t = message_storage.table::<Conversation>("conversations");
    let mut conversation = conversation;

    if let Some(old_conversation) = t.get("", &conversation.topic_id).await {
        conversation.last_read_seq = old_conversation.last_read_seq;
        conversation.last_sender_id = old_conversation.last_sender_id;
        conversation.last_message_at = old_conversation.last_message_at;
        conversation.last_message = old_conversation.last_message;
        conversation.unread = old_conversation.unread;
    }

    conversation.is_partial = false;
    conversation.cached_at = now_millis();
    t.set("", &conversation.topic_id, Some(&conversation)).await;
    Ok(conversation)
}

pub(super) async fn merge_conversation_from_chat(
    message_storage: Arc<Storage>,
    req: &ChatRequest,
) -> Result<Conversation> {
    let t = message_storage.table::<Conversation>("conversations");
    let mut conversation = t
        .get("", &req.topic_id)
        .await
        .unwrap_or(Conversation::from(req));
    if let Some(content) = req.content.as_ref() {
        match ContentType::from(content.r#type.clone()) {
            ContentType::None | ContentType::Recall => {}
            _ => {
                if req.seq > conversation.last_read_seq
                    && !req.unreadable
                    && !req.chat_id.is_empty()
                {
                    conversation.unread += 1;
                }
            }
        }
    }

    if req.seq >= conversation.last_seq {
        conversation.last_seq = req.seq;

        if !req.unreadable && !req.chat_id.is_empty() {
            conversation.last_sender_id = req.attendee.clone();
            conversation.last_message_at = req.created_at.clone();
            conversation.last_message = req.content.clone();
            conversation.updated_at = req.created_at.clone();
        }
    }

    conversation.cached_at = now_millis();
    t.set("", &conversation.topic_id, Some(&conversation)).await;
    Ok(conversation)
}

impl ClientStore {
    pub(crate) async fn merge_conversations(
        &self,
        conversations: Vec<Conversation>,
    ) -> Vec<Conversation> {
        let mut result = vec![];
        for conversation in conversations {
            let conversation =
                match merge_conversation(self.message_storage.clone(), conversation).await {
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

    pub async fn update_conversation(&self, conversation: Conversation) -> Result<Conversation> {
        merge_conversation(self.message_storage.clone(), conversation).await
    }

    pub async fn set_conversation_remark(
        &self,
        topic_id: &str,
        remark: Option<String>,
    ) -> Result<Conversation> {
        {
            let t = self.message_storage.table::<Conversation>("conversations");
            if let Some(mut conversation) = t.get("", topic_id).await {
                conversation.remark = remark.clone();
                t.set("", topic_id, Some(&conversation)).await;
            }
        }

        let c = set_conversation_remark(&self.endpoint, &self.token, &topic_id, remark).await?;
        let c = merge_conversation(self.message_storage.clone(), c).await?;
        self.emit_conversation_update(c)
    }

    pub async fn set_conversation_sticky(
        &self,
        topic_id: &str,
        sticky: bool,
    ) -> Result<Conversation> {
        {
            let t = self.message_storage.table::<Conversation>("conversations");
            if let Some(mut conversation) = t.get("", topic_id).await {
                conversation.sticky = sticky;
                t.set("", topic_id, Some(&conversation)).await;
            }
        }

        let c = set_conversation_sticky(&self.endpoint, &self.token, &topic_id, sticky).await?;
        let c = merge_conversation(self.message_storage.clone(), c).await?;
        self.emit_conversation_update(c)
    }

    pub async fn set_conversation_mute(&self, topic_id: &str, mute: bool) -> Result<Conversation> {
        {
            let t = self.message_storage.table::<Conversation>("conversations");
            if let Some(mut conversation) = t.get("", topic_id).await {
                conversation.mute = mute;
                t.set("", topic_id, Some(&conversation)).await;
            }
        }

        let c = set_conversation_mute(&self.endpoint, &self.token, &topic_id, mute).await?;
        let c = merge_conversation(self.message_storage.clone(), c).await?;
        self.emit_conversation_update(c)
    }

    pub async fn set_conversation_read_local(&self, topic_id: &str) -> Option<Conversation> {
        let t = self.message_storage.table::<Conversation>("conversations");
        match t.get("", topic_id).await {
            Some(mut conversation) => {
                if conversation.is_partial {
                    return None;
                }

                if conversation.last_read_seq == conversation.last_seq && conversation.unread == 0 {
                    return None;
                }

                conversation.last_read_seq = conversation.last_seq;
                conversation.unread = 0;
                t.set("", topic_id, Some(&conversation)).await;
                Some(conversation)
            }
            _ => None,
        }
    }

    pub async fn set_conversation_read(&self, topic_id: &str) {
        match self.set_conversation_read_local(topic_id).await {
            Some(conversation) => {
                match set_conversation_read(&self.endpoint, &self.token, &topic_id).await {
                    Ok(_) => self.emit_conversation_update(conversation).ok(),
                    Err(e) => {
                        warn!("set_conversation_read failed: {:?}", e);
                        None
                    }
                };
            }
            None => {}
        }
    }

    pub async fn set_conversation_tags(
        &self,
        topic_id: &str,
        tags: Option<Tags>,
    ) -> Result<Conversation> {
        {
            let t = self.message_storage.table::<Conversation>("conversations");
            if let Some(mut conversation) = t.get("", topic_id).await {
                conversation.tags = tags.clone();
                t.set("", topic_id, Some(&conversation)).await;
            }
        }

        let values = serde_json::json!({
            "tags": tags.unwrap_or_default(),
        });
        let c = update_conversation(&self.endpoint, &self.token, &topic_id, &values).await?;
        let c = merge_conversation(self.message_storage.clone(), c).await?;
        self.emit_conversation_update(c)
    }

    pub async fn set_conversation_extra(
        &self,
        topic_id: &str,
        extra: Option<Extra>,
    ) -> Result<Conversation> {
        {
            let t = self.message_storage.table::<Conversation>("conversations");
            if let Some(mut conversation) = t.get("", topic_id).await {
                conversation.extra = extra.clone();
                t.set("", topic_id, Some(&conversation)).await;
            }
        }

        let values = serde_json::json!({
            "extra": extra.unwrap_or_default(),
        });

        let c = update_conversation(&self.endpoint, &self.token, &topic_id, &values).await?;
        let c = merge_conversation(self.message_storage.clone(), c).await?;
        self.emit_conversation_update(c)
    }

    pub(crate) async fn remove_conversation(&self, topic_id: &str) {
        {
            let t = self.message_storage.table::<Conversation>("conversations");
            t.remove("", topic_id).await;
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

    pub(crate) async fn get_conversation(&self, topic_id: &str) -> Option<Conversation> {
        let t = self.message_storage.table::<Conversation>("conversations");
        let conversation = t
            .get("", topic_id)
            .await
            .unwrap_or(Conversation::new(topic_id));

        if conversation.is_partial
            || is_cache_expired(conversation.cached_at, CONVERSATION_CACHE_EXPIRE_SECS)
        {
            self.fetch_conversation(topic_id).await;
        }
        Some(conversation)
    }

    pub(super) async fn fetch_conversation(&self, topic_id: &str) {
        let endpoint = self.endpoint.clone();
        let token = self.token.clone();
        let topic_id = topic_id.to_string();
        let message_storage = self.message_storage.clone();
        let callback = self.callback.clone();

        spwan_task(async move {
            match get_conversation(&endpoint, &token, &topic_id).await {
                Ok(c) => {
                    let c = match merge_conversation(message_storage, c).await {
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

    pub(super) async fn update_conversation_read(
        &self,
        topic_id: &str,
        updated_at: &str,
        last_read_seq: Option<i64>,
    ) -> Result<()> {
        let t = self.message_storage.table::<Conversation>("conversations");

        if let Some(mut conversation) = t.get("", topic_id).await {
            conversation.last_read_seq = last_read_seq.unwrap_or(conversation.last_seq);
            conversation.updated_at = updated_at.to_string();
            t.set("", topic_id, Some(&conversation)).await;
        }
        Ok(())
    }

    pub(super) async fn save_outgoing_chat_log(&self, req: &ChatRequest) -> Result<()> {
        let t = self.message_storage.table::<ChatLog>("chat_logs");

        let mut log = ChatLog::from(req);
        log.status = ChatLogStatus::Sending;
        log.sender_id = self.user_id.clone();
        t.set(&log.topic_id, &log.id, Some(&log)).await;

        Ok(())
    }

    pub(super) async fn update_outoing_chat_log_state(
        &self,
        topic_id: &str,
        chat_id: &str,
        status: ChatLogStatus,
        seq: Option<i64>,
    ) -> Result<()> {
        let t = self.message_storage.table::<ChatLog>("chat_logs");

        if let Some(log) = t.get(topic_id, chat_id).await {
            let mut log = log.clone();
            log.status = status;
            if let Some(seq) = seq {
                log.seq = seq;
            }
            t.set(topic_id, chat_id, Some(&log)).await;
        }
        Ok(())
    }

    pub(super) async fn save_incoming_chat_log(&self, req: &ChatRequest) -> Result<()> {
        if req.chat_id.is_empty() || req.seq <= 0 {
            return Ok(());
        }

        let t = self.message_storage.table::<ChatLog>("chat_logs");
        let topic_id = &req.topic_id;
        let chat_id = &req.chat_id;
        let now = now_millis();
        let mut new_status = ChatLogStatus::Received;

        match req.content.as_ref() {
            Some(content) => match ContentType::from(content.r#type.clone()) {
                ContentType::None | ContentType::Recall => {
                    let recall_chat_id = match req.content.as_ref() {
                        Some(content) => &content.text,
                        None => return Err(Error::Other("[recall] invalid content".to_string())),
                    };

                    match t.get(&topic_id, recall_chat_id).await {
                        Some(recall_log) => {
                            if recall_log.recall {
                                return Ok(());
                            }

                            if MAX_RECALL_SECS > 0
                                && now - recall_log.cached_at > MAX_RECALL_SECS * 1000
                            {
                                return Err(Error::Other("[recall] timeout".to_string()));
                            }

                            match recall_log.status {
                                ChatLogStatus::Sent | ChatLogStatus::Received => {}
                                _ => {
                                    return Err(Error::Other("[recall] invalid status".to_string()))
                                }
                            }

                            if req.attendee != recall_log.sender_id {
                                return Err(Error::Other("[recall] invalid owner".to_string()));
                            }

                            let mut recall_log = recall_log.clone();
                            recall_log.recall = true;
                            recall_log.content = Content::new(ContentType::None);
                            t.set(&topic_id, &recall_chat_id, Some(&recall_log)).await;
                        }
                        None => return Ok(()),
                    }
                }
                ContentType::UpdateExtra => {
                    let (extra, update_chat_id) = match req.content.as_ref() {
                        Some(content) => (&content.extra, &content.text),
                        None => {
                            return Err(Error::Other("[update_extra] invalid content".to_string()))
                        }
                    };

                    match t.get(&topic_id, update_chat_id).await {
                        Some(mut log) => {
                            log.content.extra = extra.clone();
                            t.set(&topic_id, &update_chat_id, Some(&log)).await;
                        }
                        None => {}
                    }
                }
                _ => {}
            },
            None => {}
        }

        if let Some(old_log) = t.get(&topic_id, &chat_id).await {
            match old_log.status {
                ChatLogStatus::Sending => new_status = ChatLogStatus::Sent,
                _ => return Ok(()),
            }
        }

        let mut log = ChatLog::from(req);
        log.cached_at = now;
        log.status = new_status;
        t.set(&log.topic_id, &log.id, Some(&log)).await;
        Ok(())
    }

    pub(crate) async fn save_chat_log(&self, chat_log: &ChatLog) -> Result<()> {
        let t = self.message_storage.table::<ChatLog>("chat_logs");
        let item = match ContentType::from(chat_log.content.r#type.to_string()) {
            ContentType::None => Some(chat_log), // remove local log
            ContentType::Recall => {
                match t.get(&chat_log.topic_id, &chat_log.content.text).await {
                    Some(recall_log) => {
                        if !recall_log.recall {
                            let mut log = recall_log.clone();
                            log.recall = true;
                            log.content = Content::new(ContentType::None);
                            t.set(&chat_log.topic_id, &chat_log.content.text, Some(&log))
                                .await;
                        }
                    }
                    None => {}
                };
                Some(chat_log)
            }
            ContentType::UpdateExtra => {
                match t.get(&chat_log.topic_id, &chat_log.content.text).await {
                    Some(update_log) => {
                        if !update_log.recall {
                            let mut log = update_log.clone();
                            log.content.extra = chat_log.content.extra.clone();
                            t.set(&chat_log.topic_id, &chat_log.content.text, Some(&log))
                                .await;
                        }
                    }
                    None => {}
                };
                Some(chat_log)
            }
            _ => Some(chat_log),
        };

        t.set(&chat_log.topic_id, &chat_log.id, item).await;
        Ok(())
    }

    pub async fn get_chat_logs(
        &self,
        topic_id: &str,
        last_seq: Option<i64>,
        limit: u32,
    ) -> Result<QueryResult<ChatLog>> {
        let t = self.message_storage.table::<ChatLog>("chat_logs");

        let mut r = QueryResult {
            start_sort_value: 0,
            end_sort_value: 0,
            items: Vec::new(),
        };

        let mut limit = limit;
        let mut last_seq = last_seq;

        loop {
            let option = QueryOption {
                keyword: None,
                start_sort_value: last_seq,
                limit,
            };

            let result = t.query(topic_id, &option).await;
            if result.items.len() == 0 {
                break;
            }
            let has_more = result.items.len() >= limit as usize;
            let next_last_seq = result.items.last().map(|v| v.seq);

            let items: Vec<ChatLog> = result
                .items
                .into_iter()
                .filter(
                    |item| match ContentType::from(item.content.r#type.to_string()) {
                        ContentType::None => false,
                        _ => true,
                    },
                )
                .collect();

            r.items.extend(items);
            if !has_more || r.items.len() >= limit as usize {
                break;
            }
            limit -= r.items.len() as u32;
            last_seq = next_last_seq.map(|v| v - limit as i64);
        }
        r.start_sort_value = r.items.first().map(|v| v.seq).unwrap_or(0);
        r.end_sort_value = r.items.last().map(|v| v.seq).unwrap_or(0);
        Ok(r)
    }

    pub async fn get_chat_log(&self, topic_id: &str, chat_id: &str) -> Option<ChatLog> {
        let t = self.message_storage.table("chat_logs");
        t.get(topic_id, chat_id).await
    }

    pub async fn remove_messages(&self, topic_id: &str, chat_ids: &[String]) {
        let t = self.message_storage.table::<ChatLog>("chat_logs");
        for chat_id in chat_ids {
            t.remove(topic_id, chat_id).await;
        }
    }

    pub async fn get_conversations(
        &self,
        updated_at: &str,
        limit: u32,
    ) -> Result<QueryResult<Conversation>> {
        let t = self.message_storage.table("conversations");

        let start_sort_value = chrono::DateTime::parse_from_rfc3339(updated_at)
            .map(|v| v.timestamp_millis())
            .ok();

        let option = QueryOption {
            keyword: None,
            start_sort_value,
            limit,
        };
        Ok(t.query("", &option).await)
    }

    pub async fn filter_conversation(
        &self,
        predicate: Box<dyn Fn(Conversation) -> Option<Conversation> + Send>,
    ) -> Vec<Conversation> {
        let t = self.message_storage.table("conversations");
        t.filter("", Box::new(move |c| predicate(c))).await
    }

    pub async fn get_last_conversation_updated_at(&self) -> Option<String> {
        let t = self.message_storage.table::<Conversation>("conversations");
        let conversation = t.last("").await;
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
