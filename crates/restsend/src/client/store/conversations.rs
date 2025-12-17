use super::{is_cache_expired, ClientStore};
use crate::{
    callback::ChatRequestStatus,
    models::{
        conversation::{ConversationUpdateFields, Extra, Tags},
        ChatLog, ChatLogStatus, Content, ContentType, Conversation,
    },
    request::ChatRequest,
    services::{conversation::*, topic::get_topic},
    storage::{QueryOption, QueryResult, Storage, StoreModel, Table, ValueItem},
    utils::{elapsed, now_millis},
};
use crate::{Error, Result};
use log::warn;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Mutex,
};
use std::{collections::HashSet, sync::Arc};

pub(crate) async fn merge_conversation(
    message_storage: Arc<Storage>,
    conversation: Conversation,
) -> Result<Conversation> {
    let t = message_storage.table::<Conversation>().await?;
    let mut conversation = conversation;

    let mut sync_last_readable = false;
    if let Some(old_conversation) = t.get("", &conversation.topic_id).await {
        if old_conversation.last_message_seq != conversation.last_message_seq {
            sync_last_readable = true;
        }
        conversation.merge_local_read_state(&old_conversation);
    }

    if sync_last_readable {
        if let Some(log) =
            get_conversation_last_readable_message(message_storage.clone(), &conversation.topic_id)
                .await
        {
            conversation.last_message = Some(log.content.clone());
            conversation.last_message_at = log.created_at.clone();
            conversation.last_sender_id = log.sender_id;
            conversation.last_message_seq = Some(log.seq);
        }
    }

    conversation.is_partial = false;
    conversation.cached_at = now_millis();
    t.set("", &conversation.topic_id, Some(&conversation))
        .await
        .ok();
    Ok(conversation)
}

async fn get_conversation_last_readable_message(
    message_storage: Arc<Storage>,
    topic_id: &str,
) -> Option<ChatLog> {
    let t = message_storage.readonly_table::<ChatLog>().await.ok()?;
    let last_log = t.last(topic_id).await?;
    if !last_log.content.unreadable {
        return Some(last_log);
    }

    let option = QueryOption {
        keyword: None,
        start_sort_value: Some(last_log.seq),
        limit: 10,
    };

    let result = t.query(topic_id, &option).await?;
    for log in result.items.into_iter() {
        if !log.content.unreadable {
            return Some(log);
        }
    }
    None
}

impl ClientStore {
    pub(super) async fn merge_conversation_from_chat(
        &self,
        req: &ChatRequest,
        req_status: &mut ChatRequestStatus,
        is_countable: bool,
    ) -> Option<Conversation> {
        let t = self.message_storage.table::<Conversation>().await.ok()?;
        let mut conversation = match t.get("", &req.topic_id).await {
            Some(c) => c,
            None => self
                .get_conversation_by(Conversation::new(&req.topic_id), true, false)
                .await
                .unwrap_or_else(|| Conversation::new(&req.topic_id)),
        };

        if let Some(content) = req.content.as_ref() {
            match ContentType::from(content.content_type.clone()) {
                ContentType::None | ContentType::Recall => {}
                ContentType::TopicJoin => {
                    conversation.last_message_at = req.created_at.clone();
                    conversation.is_partial = true; // force fetch conversation
                }
                ContentType::TopicChangeOwner => {
                    conversation.topic_owner_id = Some(req.attendee.clone());
                }
                ContentType::ConversationUpdate => {
                    match serde_json::from_str::<ConversationUpdateFields>(&content.text) {
                        Ok(fields) => {
                            conversation.updated_at = req.created_at.clone();
                            if fields.extra.is_some() {
                                conversation.extra = fields.extra;
                            }
                            if fields.tags.is_some() {
                                conversation.tags = fields.tags;
                            }
                            if fields.remark.is_some() {
                                conversation.remark = fields.remark;
                            }
                            if fields.mark_unread.unwrap_or(false) && conversation.unread == 0 {
                                conversation.unread = 1;
                                req_status.has_read = false;
                            }
                            conversation.sticky = fields.sticky.unwrap_or(conversation.sticky);
                            conversation.mute = fields.mute.unwrap_or(conversation.mute);
                        }
                        Err(_) => {}
                    }
                }
                ContentType::ConversationRemoved => {
                    return None;
                }
                ContentType::TopicUpdate => {
                    match serde_json::from_str::<crate::models::Topic>(&content.text) {
                        Ok(topic) => {
                            conversation.name = topic.name;
                            conversation.icon = topic.icon;
                            conversation.topic_extra = topic.extra;
                        }
                        Err(_) => {}
                    }
                }
                ContentType::UpdateExtra => {
                    //TODO: ugly code, need refactor, need a last_message_chat_id field in Conversation
                    if let Some(lastlog_seq) = conversation.last_message_seq {
                        if let Ok(log_t) = self.message_storage.readonly_table::<ChatLog>().await {
                            if let Some(log_in_store) =
                                log_t.get(&req.topic_id, &content.text).await
                            {
                                if lastlog_seq == log_in_store.seq {
                                    if let Some(last_message_content) =
                                        conversation.last_message.as_mut()
                                    {
                                        last_message_content.extra = content.extra.clone();
                                    }
                                }
                            }
                        }
                    }
                }
                _ => {
                    if req.seq > conversation.last_read_seq
                        && is_countable
                        && !req.chat_id.is_empty()
                        && req_status.unread_countable
                    {
                        conversation.unread += 1;
                    }
                }
            }
        }
        if req.seq >= conversation.last_seq {
            conversation.last_seq = req.seq;

            if is_countable && !req.chat_id.is_empty() {
                conversation.last_sender_id = req.attendee.clone();
                conversation.last_message_at = req.created_at.clone();
                conversation.last_message = req.content.clone();
                conversation.last_message_seq = Some(req.seq);
                conversation.updated_at = req.created_at.clone();
            }
        }

        if req_status.has_read {
            conversation.last_read_at = Some(req.created_at.clone());
            conversation.last_read_seq = conversation.last_seq;
            conversation.unread = 0;
        }

        self.ensure_topic_owner_id(&mut conversation).await;
        conversation.cached_at = now_millis();
        t.set("", &conversation.topic_id, Some(&conversation))
            .await
            .ok();
        Some(conversation)
    }

    pub(crate) async fn merge_conversations(
        &self,
        conversations: Vec<Conversation>,
    ) -> Vec<Conversation> {
        let mut results = vec![];
        let t = match self.message_storage.table::<Conversation>().await {
            Ok(t) => t,
            Err(_) => return vec![],
        };
        let now = now_millis();
        for conversation in conversations {
            let mut conversation = conversation;

            if let Some(old_conversation) = t.get("", &conversation.topic_id).await {
                if let Some(topic_created_at) = conversation.topic_created_at.as_ref() {
                    let old_conversation_created_at = old_conversation
                        .topic_created_at
                        .as_ref()
                        .map(|v| {
                            chrono::DateTime::parse_from_rfc3339(v)
                                .map(|v| v.timestamp_millis())
                                .unwrap_or(0)
                        })
                        .unwrap_or(0);
                    let new_conversation_created_at =
                        chrono::DateTime::parse_from_rfc3339(topic_created_at)
                            .map(|v| v.timestamp_millis())
                            .unwrap_or(0);
                    // clean all logs
                    if new_conversation_created_at != old_conversation_created_at {
                        if let Ok(log_t) = self.message_storage.table::<ChatLog>().await {
                            log_t.clear(&conversation.topic_id).await.ok();
                        }
                    }
                }

                conversation.merge_local_read_state(&old_conversation);
            }

            if let Some(log) = get_conversation_last_readable_message(
                self.message_storage.clone(),
                &conversation.topic_id,
            )
            .await
            {
                conversation.last_message = Some(log.content.clone());
                conversation.last_message_at = log.created_at.clone();
                conversation.last_sender_id = log.sender_id;
                conversation.last_message_seq = Some(log.seq);
            }

            self.ensure_topic_owner_id(&mut conversation).await;
            conversation.is_partial = false;
            conversation.cached_at = now;
            results.push(ValueItem {
                partition: "".to_string(),
                sort_key: conversation.sort_key(),
                key: conversation.topic_id.clone(),
                value: Some(conversation),
            });
        }
        t.batch_update(&results).await.ok();
        results.into_iter().map(|v| v.value.unwrap()).collect()
    }

    pub fn emit_conversation_update(&self, conversation: Conversation) -> Result<Conversation> {
        if let Some(cb) = self.callback.read().unwrap().as_ref() {
            cb.on_conversations_updated(vec![conversation.clone()], None);
        }
        Ok(conversation)
    }

    pub fn emit_topic_read(&self, topic_id: String, message: ChatRequest) {
        if let Some(cb) = self.callback.read().unwrap().as_ref() {
            cb.on_topic_read(topic_id, message);
        }
    }

    pub async fn update_conversation(&self, conversation: Conversation) -> Result<Conversation> {
        let mut conversation =
            merge_conversation(self.message_storage.clone(), conversation).await?;
        if self.ensure_topic_owner_id(&mut conversation).await {
            self.persist_conversation(&conversation).await;
        }
        Ok(conversation)
    }

    pub async fn set_conversation_remark(
        &self,
        topic_id: &str,
        remark: Option<String>,
    ) -> Result<Conversation> {
        {
            let t = self.message_storage.table::<Conversation>().await?;
            if let Some(mut conversation) = t.get("", topic_id).await {
                conversation.remark = remark.clone();
                t.set("", topic_id, Some(&conversation)).await.ok();
            }
        }

        let c = set_conversation_remark(&self.endpoint, &self.token, &topic_id, remark).await?;
        let mut c = merge_conversation(self.message_storage.clone(), c).await?;
        if self.ensure_topic_owner_id(&mut c).await {
            self.persist_conversation(&c).await;
        }
        self.emit_conversation_update(c)
    }

    pub async fn set_conversation_sticky(
        &self,
        topic_id: &str,
        sticky: bool,
    ) -> Result<Conversation> {
        {
            let t = self.message_storage.table::<Conversation>().await?;
            if let Some(mut conversation) = t.get("", topic_id).await {
                conversation.sticky = sticky;
                t.set("", topic_id, Some(&conversation)).await.ok();
            }
        }

        let c = set_conversation_sticky(&self.endpoint, &self.token, &topic_id, sticky).await?;
        let mut c = merge_conversation(self.message_storage.clone(), c).await?;
        if self.ensure_topic_owner_id(&mut c).await {
            self.persist_conversation(&c).await;
        }
        self.emit_conversation_update(c)
    }

    pub async fn set_conversation_mute(&self, topic_id: &str, mute: bool) -> Result<Conversation> {
        {
            let t = self.message_storage.table::<Conversation>().await?;
            if let Some(mut conversation) = t.get("", topic_id).await {
                conversation.mute = mute;
                t.set("", topic_id, Some(&conversation)).await.ok();
            }
        }

        let c = set_conversation_mute(&self.endpoint, &self.token, &topic_id, mute).await?;
        let mut c = merge_conversation(self.message_storage.clone(), c).await?;
        if self.ensure_topic_owner_id(&mut c).await {
            self.persist_conversation(&c).await;
        }
        self.emit_conversation_update(c)
    }

    pub async fn set_conversation_read_local(
        &self,
        topic_id: &str,
        last_read_at: &str,
        last_seq: Option<i64>,
    ) -> Option<Conversation> {
        let t = self.message_storage.table::<Conversation>().await.ok()?;
        match t.get("", topic_id).await {
            Some(mut conversation) => {
                if conversation.is_partial {
                    return None;
                }
                let last_seq = last_seq.unwrap_or(conversation.last_seq);

                if conversation.last_read_seq == last_seq && conversation.unread == 0 {
                    return None;
                }
                conversation.last_read_at = Some(last_read_at.to_string());
                conversation.last_read_seq = last_seq;
                conversation.unread = 0;
                t.set("", topic_id, Some(&conversation)).await.ok();
                Some(conversation)
            }
            _ => None,
        }
    }

    pub async fn set_all_conversations_read_local(&self) -> Option<()> {
        let t = self.message_storage.table::<Conversation>().await.ok()?;
        let items = t
            .filter(
                "",
                Box::new(move |c| if c.unread == 0 { None } else { Some(c) }),
                None,
                None,
            )
            .await?;
        if items.is_empty() {
            return None;
        }

        let last_read_at = chrono::Local::now().to_rfc3339();
        let update_items = items
            .into_iter()
            .map(|mut c| {
                c.last_read_at = Some(last_read_at.clone());
                c.last_read_seq = c.last_seq;
                c.unread = 0;
                ValueItem {
                    partition: "".to_string(),
                    key: c.topic_id.clone(),
                    sort_key: c.sort_key(),
                    value: Some(c),
                }
            })
            .collect();
        t.batch_update(&update_items).await.ok()
    }

    pub async fn set_conversation_tags(
        &self,
        topic_id: &str,
        tags: Option<Tags>,
    ) -> Result<Conversation> {
        {
            let t = self.message_storage.table::<Conversation>().await?;
            if let Some(mut conversation) = t.get("", topic_id).await {
                conversation.tags = tags.clone();
                t.set("", topic_id, Some(&conversation)).await.ok();
            }
        }

        let values = serde_json::json!({
            "tags": tags.unwrap_or_default(),
        });
        let c = update_conversation(&self.endpoint, &self.token, &topic_id, &values).await?;
        let mut c = merge_conversation(self.message_storage.clone(), c).await?;
        if self.ensure_topic_owner_id(&mut c).await {
            self.persist_conversation(&c).await;
        }
        self.emit_conversation_update(c)
    }

    pub async fn mark_conversation_unread(&self, topic_id: &str) -> Result<()> {
        {
            let t = self.message_storage.table::<Conversation>().await?;
            if let Some(mut conversation) = t.get("", topic_id).await {
                if conversation.unread == 0 {
                    conversation.unread = 1;
                    t.set("", topic_id, Some(&conversation)).await.ok();
                    self.emit_conversation_update(conversation).ok();
                }
            }
        }
        mark_conversation_unread(&self.endpoint, &self.token, &topic_id)
            .await
            .ok();
        Ok(())
    }

    pub async fn set_conversation_extra(
        &self,
        topic_id: &str,
        extra: Option<Extra>,
    ) -> Result<Conversation> {
        {
            let t = self.message_storage.table::<Conversation>().await?;
            if let Some(mut conversation) = t.get("", topic_id).await {
                conversation.extra = extra.clone();
                t.set("", topic_id, Some(&conversation)).await.ok();
            }
        }

        let values = serde_json::json!({
            "extra": extra.unwrap_or_default(),
        });

        let c = update_conversation(&self.endpoint, &self.token, &topic_id, &values).await?;
        let mut c = merge_conversation(self.message_storage.clone(), c).await?;
        if self.ensure_topic_owner_id(&mut c).await {
            self.persist_conversation(&c).await;
        }
        self.emit_conversation_update(c)
    }

    pub async fn clear_conversation(&self, topic_id: &str) -> Result<()> {
        self.pop_incoming_logs(topic_id);
        {
            if let Ok(t) = self.message_storage.table::<ChatLog>().await {
                t.clear(topic_id).await.ok();
            }
        }
        {
            let t = self.message_storage.table::<Conversation>().await?;
            t.remove("", topic_id).await
        }
    }

    pub(crate) async fn sync_removed_conversation(&self, topic_id: &str) {
        match self.removed_conversations.try_write() {
            Ok(mut removed_conversations) => {
                removed_conversations.insert(topic_id.to_string(), (now_millis(), 0));
            }
            Err(_) => {}
        }

        if let Ok(t) = self.message_storage.readonly_table::<Conversation>().await {
            if let Some(_) = t.get("", topic_id).await {
                self.clear_conversation(topic_id).await.ok();
                if let Some(cb) = self.callback.read().unwrap().as_ref() {
                    cb.on_conversation_removed(topic_id.to_string());
                }
            }
        }
    }

    pub(crate) async fn remove_conversation(&self, topic_id: &str) {
        {
            self.clear_conversation(topic_id).await.ok();
        }

        match remove_conversation(&self.endpoint, &self.token, &topic_id).await {
            Ok(_) => {
                if let Some(cb) = self.callback.read().unwrap().as_ref() {
                    cb.on_conversation_removed(topic_id.to_string());
                }
            }
            Err(e) => {
                warn!("remove_conversation failed: {:?}", e);
            }
        }
    }

    pub(crate) async fn get_conversation(
        &self,
        topic_id: &str,
        cb_updated: bool,
        ensure_last_version: bool,
    ) -> Option<Conversation> {
        let t = self
            .message_storage
            .readonly_table::<Conversation>()
            .await
            .ok()?;
        let conversation = t
            .get("", topic_id)
            .await
            .unwrap_or_else(|| Conversation::new(topic_id));
        self.get_conversation_by(conversation, ensure_last_version, cb_updated)
            .await
    }

    pub(crate) async fn get_conversation_by(
        &self,
        conversation: Conversation,
        mut ensure_last_version: bool,
        cb_updated: bool,
    ) -> Option<Conversation> {
        if conversation.is_partial
            || is_cache_expired(
                conversation.cached_at,
                self.option
                    .conversation_cache_expire_secs
                    .load(Ordering::Relaxed) as i64,
            )
        {
            ensure_last_version = true;
        }

        struct PendingGuard<'a> {
            pending_conversations: &'a Mutex<HashSet<String>>,
            topic_id: &'a String,
        }
        impl<'a> Drop for PendingGuard<'a> {
            fn drop(&mut self) {
                match self.pending_conversations.lock() {
                    Ok(mut pending_conversations) => {
                        pending_conversations.remove(self.topic_id);
                    }
                    Err(_) => {}
                }
            }
        }

        {
            match self.pending_conversations.lock() {
                Ok(mut pending_conversations) => {
                    if pending_conversations.contains(&conversation.topic_id) {
                        return Some(conversation);
                    }
                    pending_conversations.insert(conversation.topic_id.to_string());
                }
                Err(_) => {}
            }
        }
        let _guard = PendingGuard {
            pending_conversations: &self.pending_conversations,
            topic_id: &conversation.topic_id.clone(),
        };

        if ensure_last_version {
            match get_conversation(&self.endpoint, &self.token, &conversation.topic_id).await {
                Ok(mut new_conversation) => {
                    new_conversation.is_partial = false;
                    new_conversation.cached_at = now_millis();
                    if conversation.last_message_seq > new_conversation.last_message_seq {
                        new_conversation.last_read_at = conversation.last_read_at;
                        new_conversation.last_read_seq = conversation.last_read_seq;
                        new_conversation.unread = conversation.unread;
                    }
                    self.ensure_topic_owner_id(&mut new_conversation).await;
                    if let Ok(t) = self.message_storage.table::<Conversation>().await {
                        t.set("", &new_conversation.topic_id, Some(&new_conversation))
                            .await
                            .ok();
                    }
                    if cb_updated {
                        if let Some(cb) = self.callback.read().unwrap().as_ref() {
                            cb.on_conversations_updated(vec![new_conversation.clone()], None);
                        }
                    }
                    return Some(new_conversation);
                }
                Err(e) => {
                    warn!(
                        "fetch_conversation {} failed: {:?}",
                        conversation.topic_id, e
                    );
                    if e.to_string().contains("404") {
                        self.clear_conversation(&conversation.topic_id).await.ok();
                        if let Some(cb) = self.callback.read().unwrap().as_ref() {
                            cb.on_conversation_removed(conversation.topic_id.clone());
                        }
                        return None;
                    }
                    return Some(conversation);
                }
            }
        }
        Some(conversation)
    }

    pub(super) async fn save_outgoing_chat_log(&self, req: &ChatRequest) -> Result<()> {
        let t = self.message_storage.table::<ChatLog>().await?;

        let mut log = ChatLog::from(req);
        log.status = ChatLogStatus::Sending;
        log.sender_id = self.user_id.clone();
        t.set(&log.topic_id, &log.id, Some(&log)).await.ok();

        Ok(())
    }

    pub(super) async fn update_outoing_chat_log_state(
        &self,
        topic_id: &str,
        chat_id: &str,
        status: ChatLogStatus,
        seq: Option<i64>,
    ) -> Result<()> {
        let t = self.message_storage.table::<ChatLog>().await?;

        if let Some(mut log) = t.get(topic_id, chat_id).await {
            log.status = status;
            seq.map(|v| log.seq = v);
            t.set(topic_id, chat_id, Some(&log)).await?;
        }
        Ok(())
    }

    fn pop_incoming_logs(&self, topic_id: &str) -> Option<Vec<String>> {
        match self.incoming_logs.try_write() {
            Ok(mut logs) => logs.remove(topic_id),
            Err(_) => None,
        }
    }

    fn put_incoming_log(&self, topic_id: &str, log_id: &str) {
        let mut logs = match self.incoming_logs.try_write() {
            Ok(logs) => logs,
            Err(_) => return,
        };
        let items = logs.entry(topic_id.to_string()).or_insert(vec![]);
        if items.len()
            > self
                .option
                .max_incoming_log_cache_count
                .load(Ordering::Relaxed)
        {
            items.remove(0);
        }
        items.push(log_id.to_string());
    }

    pub(super) async fn save_incoming_chat_log(&self, req: &ChatRequest) -> Result<()> {
        if req.chat_id.is_empty() || req.seq <= 0 {
            return Ok(());
        }

        let log_t = self.message_storage.table::<ChatLog>().await?;
        let topic_id = &req.topic_id;
        let chat_id = &req.chat_id;
        let now = now_millis();
        let mut new_status = ChatLogStatus::Received;

        match req.content.as_ref() {
            Some(content) => match ContentType::from(content.content_type.clone()) {
                ContentType::TopicJoin => {
                    if req.attendee == self.user_id {
                        // when user join topic, clear all local logs
                        self.clear_conversation(topic_id).await.ok();
                    }
                }
                ContentType::Recall => {
                    let recall_chat_id = match req.content.as_ref() {
                        Some(content) => &content.text,
                        None => return Err(Error::Other("[recall] invalid content".to_string())),
                    };

                    match log_t.get(&topic_id, recall_chat_id).await {
                        Some(recall_log) => {
                            if recall_log.recall {
                                return Ok(());
                            }
                            let max_recall_secs =
                                self.option.max_recall_secs.load(Ordering::Relaxed) as i64;
                            if max_recall_secs > 0
                                && now - recall_log.cached_at > max_recall_secs * 1000
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
                            log_t
                                .set(&topic_id, &recall_chat_id, Some(&recall_log))
                                .await
                                .ok();
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

                    match log_t.get(&topic_id, update_chat_id).await {
                        Some(mut log) => {
                            log.content.extra = extra.clone();
                            log_t.set(&topic_id, &update_chat_id, Some(&log)).await.ok();
                        }
                        None => {}
                    }
                }
                _ => {}
            },
            None => {}
        }

        if let Some(old_log) = log_t.get(&topic_id, &chat_id).await {
            match old_log.status {
                ChatLogStatus::Sending => new_status = ChatLogStatus::Sent,
                _ => return Ok(()),
            }
        }

        self.put_incoming_log(topic_id, chat_id);

        let mut log = ChatLog::from(req);
        log.cached_at = now;
        log.status = new_status;
        log_t.set(&log.topic_id, &log.id, Some(&log)).await
    }

    pub(crate) async fn save_chat_logs(
        &self,
        table: &Box<dyn Table<ChatLog>>,
        logs: &Vec<ChatLog>,
    ) -> Result<()> {
        //let table = self.message_storage.table::<ChatLog>().await;
        let mut items = vec![];
        for chat_log in logs {
            let item = match ContentType::from(chat_log.content.content_type.to_string()) {
                ContentType::None => Some(chat_log), // remove local log
                ContentType::Recall => {
                    match table.get(&chat_log.topic_id, &chat_log.content.text).await {
                        Some(recall_log) => {
                            if !recall_log.recall {
                                let mut log = recall_log.clone();
                                log.recall = true;
                                log.content = Content::new(ContentType::Recalled);
                                table
                                    .set(&chat_log.topic_id, &chat_log.content.text, Some(&log))
                                    .await
                                    .ok();
                            }
                        }
                        None => {}
                    };
                    Some(chat_log)
                }
                ContentType::UpdateExtra => {
                    match table.get(&chat_log.topic_id, &chat_log.content.text).await {
                        Some(update_log) => {
                            if !update_log.recall {
                                let mut log = update_log.clone();
                                log.content.extra = chat_log.content.extra.clone();
                                table
                                    .set(&chat_log.topic_id, &chat_log.content.text, Some(&log))
                                    .await
                                    .ok();
                            }
                        }
                        None => {}
                    };
                    Some(chat_log)
                }
                _ => Some(chat_log),
            };
            if let Some(item) = item {
                items.push(ValueItem {
                    partition: item.topic_id.clone(),
                    key: item.id.clone(),
                    sort_key: item.sort_key(),
                    value: Some(item.clone()),
                });
            }
        }
        table.batch_update(&items).await
    }

    pub async fn get_chat_logs(
        &self,
        topic_id: &str,
        conversation_start_seq: i64,
        last_seq: Option<i64>,
        limit: u32,
    ) -> Result<(QueryResult<ChatLog>, bool)> {
        let log_t = self.message_storage.readonly_table::<ChatLog>().await?;
        self.get_chat_logs_with_table(&log_t, topic_id, conversation_start_seq, last_seq, limit)
            .await
    }

    pub async fn get_chat_logs_with_table(
        &self,
        table: &Box<dyn Table<ChatLog>>,
        topic_id: &str,
        conversation_start_seq: i64,
        last_seq: Option<i64>,
        limit: u32,
    ) -> Result<(QueryResult<ChatLog>, bool)> {
        let st = now_millis();

        let mut r = QueryResult {
            start_sort_value: 0,
            end_sort_value: 0,
            items: Vec::new(),
            has_more: false,
        };
        let initial_limit = limit;
        let mut limit = limit;
        let mut last_seq = last_seq;

        let mut total_limit = 0;
        let mut query_diff = 0;
        loop {
            let option = QueryOption {
                keyword: None,
                start_sort_value: last_seq,
                limit,
            };
            total_limit += limit;
            let result = match table.query(topic_id, &option).await {
                Some(result) => result,
                None => break,
            };

            if result.items.len() == 0 {
                break;
            }
            let diff = result.start_sort_value - result.end_sort_value;
            if diff > result.items.len() as i64 {
                break;
            }
            query_diff += diff;

            let next_last_seq = result.items.last().map(|v| v.seq);
            let items: Vec<ChatLog> = result
                .items
                .into_iter()
                .filter(
                    |item| match ContentType::from(item.content.content_type.to_string()) {
                        ContentType::None => false,
                        _ => true,
                    },
                )
                .collect();

            r.items.extend(items);
            r.has_more = result.has_more;
            if !result.has_more || r.items.len() as u32 >= limit {
                break;
            }
            limit -= r.items.len() as u32;
            last_seq = next_last_seq.map(|v| v - limit as i64);
        }
        r.start_sort_value = r.items.first().map(|v| v.seq).unwrap_or(0);
        r.end_sort_value = r.items.last().map(|v| v.seq).unwrap_or(0);
        let need_fetch = {
            if r.items.len() == 0 {
                true
            } else if query_diff > total_limit as i64 {
                true
            } else if r.items.len() != initial_limit as usize {
                let total_diff = r.start_sort_value - conversation_start_seq;
                let must_have = if total_diff > initial_limit as i64 {
                    initial_limit as i64
                } else {
                    total_diff
                };
                r.items.len() < must_have as usize
            } else {
                false
            }
        };

        log::info!(
            "get_chat_logs: topic_id: {}, conversation_start_seq:{}, items:{}, last_seq: {:?}, initial_limit: {}, limit: {}, total_limit: {}, query_diff: {}, need_fetch: {} cost: {:?}",
            topic_id,
            conversation_start_seq,
            r.items.len(),
            last_seq,
            initial_limit,
            limit,
            total_limit,
            query_diff,
            need_fetch,
            elapsed(st),
        );

        match self.pop_incoming_logs(topic_id) {
            Some(incoming_logs) => {
                for log_id in incoming_logs {
                    match r.items.iter_mut().find(|v| v.id == log_id) {
                        Some(_) => {}
                        None => {
                            if let Some(item) = table.get(topic_id, &log_id).await {
                                log::info!(
                                    "get_chat_logs: find lost log log_id: {} seq: {}",
                                    log_id,
                                    item.seq
                                );
                                r.items.push(item);
                            }
                        }
                    }
                }
            }
            None => {}
        }

        Ok((r, need_fetch))
    }

    pub async fn get_chat_log(&self, topic_id: &str, chat_id: &str) -> Option<ChatLog> {
        let t = self.message_storage.readonly_table().await.ok()?;
        t.get(topic_id, chat_id).await
    }

    pub async fn remove_messages(&self, topic_id: &str, chat_ids: &[String]) {
        if let Ok(t) = self.message_storage.table::<ChatLog>().await {
            for chat_id in chat_ids {
                t.remove(topic_id, chat_id).await.ok();
            }
        }
    }

    async fn persist_conversation(&self, conversation: &Conversation) {
        if let Ok(t) = self.message_storage.table::<Conversation>().await {
            t.set("", &conversation.topic_id, Some(conversation))
                .await
                .ok();
        }
    }

    fn get_cached_topic_owner(&self, topic_id: &str) -> Option<String> {
        self.topic_owner_cache
            .read()
            .ok()
            .and_then(|cache| cache.get(topic_id).cloned())
            .and_then(|(owner_id, cached_at)| {
                if is_cache_expired(
                    cached_at,
                    self.option
                        .topic_owner_cache_expire_secs
                        .load(Ordering::Relaxed) as i64,
                ) {
                    None
                } else {
                    Some(owner_id)
                }
            })
    }

    fn cache_topic_owner(&self, topic_id: &str, owner_id: &str) {
        if owner_id.is_empty() {
            return;
        }
        if let Ok(mut cache) = self.topic_owner_cache.write() {
            cache.insert(topic_id.to_string(), (owner_id.to_string(), now_millis()));
        }
    }

    async fn ensure_topic_owner_id(&self, conversation: &mut Conversation) -> bool {
        if !conversation.multiple {
            return false;
        }

        if let Some(owner_id) = conversation.topic_owner_id.as_ref() {
            self.cache_topic_owner(&conversation.topic_id, owner_id);
            return false;
        }

        if let Some(owner_id) = self.get_cached_topic_owner(&conversation.topic_id) {
            conversation.topic_owner_id = Some(owner_id);
            return true;
        }

        match get_topic(&self.endpoint, &self.token, &conversation.topic_id).await {
            Ok(topic) => {
                if !topic.owner_id.is_empty() {
                    self.cache_topic_owner(&conversation.topic_id, &topic.owner_id);
                    conversation.topic_owner_id = Some(topic.owner_id.clone());
                    if conversation.topic_created_at.is_none() && !topic.created_at.is_empty() {
                        conversation.topic_created_at = Some(topic.created_at.clone());
                    }
                    return true;
                }
            }
            Err(err) => warn!(
                "ensure_topic_owner_id failed to fetch topic {}: {:?}",
                conversation.topic_id, err
            ),
        }

        false
    }

    pub async fn get_conversations(
        &self,
        updated_at: &str,
        limit: u32,
    ) -> Result<QueryResult<Conversation>> {
        let t = self.message_storage.table::<Conversation>().await?;

        let start_sort_value = chrono::DateTime::parse_from_rfc3339(updated_at)
            .map(|v| v.timestamp_millis())
            .ok();

        let option = QueryOption {
            keyword: None,
            start_sort_value,
            limit,
        };

        let mut result = match t.query("", &option).await {
            Some(result) => result,
            None => QueryResult {
                start_sort_value: 0,
                end_sort_value: 0,
                items: vec![],
                has_more: false,
            },
        };

        let mut updated_topic_owner = Vec::new();
        for conversation in &mut result.items {
            if let Some(log) = get_conversation_last_readable_message(
                self.message_storage.clone(),
                &conversation.topic_id,
            )
            .await
            {
                conversation.last_message = Some(log.content.clone());
                conversation.last_message_at = log.created_at.clone();
                conversation.last_sender_id = log.sender_id;
                conversation.last_message_seq = Some(log.seq);
            }

            if self.ensure_topic_owner_id(conversation).await {
                updated_topic_owner.push(conversation.clone());
            }
        }

        if !updated_topic_owner.is_empty() {
            for conversation in updated_topic_owner.iter() {
                t.set("", &conversation.topic_id, Some(conversation))
                    .await
                    .ok();
            }
        }
        Ok(result)
    }

    pub async fn filter_conversation(
        &self,
        predicate: Box<dyn Fn(Conversation) -> Option<Conversation> + Send>,
        end_sort_value: Option<i64>,
        limit: Option<u32>,
    ) -> Option<Vec<Conversation>> {
        let t = self.message_storage.readonly_table().await.ok()?;
        let mut conversations = t
            .filter("", Box::new(move |c| predicate(c)), end_sort_value, limit)
            .await?;

        let mut updated_topic_owner = Vec::new();
        for conversation in conversations.iter_mut() {
            if self.ensure_topic_owner_id(conversation).await {
                updated_topic_owner.push(conversation.clone());
            }
        }

        if !updated_topic_owner.is_empty() {
            if let Ok(table) = self.message_storage.table::<Conversation>().await {
                for conversation in updated_topic_owner.iter() {
                    table
                        .set("", &conversation.topic_id, Some(conversation))
                        .await
                        .ok();
                }
            }
        }

        Some(conversations)
    }

    pub async fn get_unread_count(&self) -> u32 {
        let t = match self.message_storage.readonly_table::<Conversation>().await {
            Ok(t) => t,
            Err(_) => return 0,
        };
        let count = Arc::new(AtomicUsize::new(0));
        let count_ref = count.clone();
        t.filter(
            "",
            Box::new(move |c| {
                count_ref.fetch_add(c.unread as usize, Ordering::Relaxed);
                None
            }),
            None,
            None,
        )
        .await;
        count.load(Ordering::Relaxed) as u32
    }
}
