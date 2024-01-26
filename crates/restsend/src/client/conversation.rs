use super::Client;
use crate::callback::{SyncChatLogsCallback, SyncConversationsCallback};
use crate::models::conversation::{Extra, Tags};
use crate::models::{ChatLog, ChatLogStatus, Conversation, GetChatLogsResult};
use crate::services::conversation::create_chat;
use crate::services::conversation::{
    clean_messages, get_chat_logs_desc, get_conversations, remove_messages,
};
use crate::utils::now_millis;
use crate::{Result, MAX_CONVERSATION_LIMIT, MAX_LOGS_LIMIT};
use log::{debug, warn};
use restsend_macros::export_wasm_or_ffi;

#[export_wasm_or_ffi]
impl Client {
    pub async fn create_chat(&self, user_id: String) -> Result<Conversation> {
        let conversation = create_chat(&self.endpoint, &self.token, &user_id).await?;
        self.store.update_conversation(conversation).await
    }

    pub async fn clean_messages(&self, topic_id: String) -> Result<()> {
        clean_messages(&self.endpoint, &self.token, &topic_id).await
    }

    pub async fn remove_messages(
        &self,
        topic_id: String,
        chat_ids: Vec<String>,
        sync_to_server: bool,
    ) -> Result<()> {
        self.store.remove_messages(&topic_id, &chat_ids).await;

        if sync_to_server {
            remove_messages(&self.endpoint, &self.token, &topic_id, chat_ids).await
        } else {
            Ok(())
        }
    }

    pub async fn get_chat_log(&self, topic_id: String, chat_id: String) -> Option<ChatLog> {
        self.store.get_chat_log(&topic_id, &chat_id).await
    }

    pub async fn search_chat_log(
        &self,
        _topic_id: Option<String>,
        _sender_id: Option<String>,
        _keyword: String,
    ) -> Option<GetChatLogsResult> {
        warn!("search_chat_log not implemented");
        None
    }

    async fn try_sync_chat_logs(&self, topic_id: String, last_seq: Option<i64>, limit: u32) {
        struct DummySyncChatLogsCallback {}
        impl SyncChatLogsCallback for DummySyncChatLogsCallback {}
        self.sync_chat_logs(
            topic_id,
            last_seq,
            limit,
            Box::new(DummySyncChatLogsCallback {}),
        )
        .await;
    }

    pub async fn sync_chat_logs(
        &self,
        topic_id: String,
        last_seq: Option<i64>,
        limit: u32,
        callback: Box<dyn SyncChatLogsCallback>,
    ) {
        let limit = if limit == 0 { MAX_LOGS_LIMIT } else { limit }.min(MAX_LOGS_LIMIT);
        let start_seq = self
            .store
            .get_conversation(&topic_id, true)
            .await
            .map(|c| c.start_seq)
            .unwrap_or(0);

        match self.store.get_chat_logs(&topic_id, last_seq, limit).await {
            Ok(local_logs) => {
                let mut need_fetch = local_logs.items.len() < limit as usize;

                if need_fetch
                    && local_logs.items.len() > 0
                    && local_logs.items.len() < limit as usize
                {
                    need_fetch = local_logs.end_sort_value != start_seq + 1;
                }

                if !need_fetch {
                    callback.on_success(GetChatLogsResult::from_local_logs(local_logs, start_seq));
                    return;
                }
                debug!(
                    "sync_chat_logs local_logs.len: {} start_seq: {} limit: {} local_logs.end_sort_value:{}",
                    local_logs.items.len(),
                    start_seq,
                    limit,
                    local_logs.end_sort_value
                )
            }
            Err(e) => {
                warn!("sync_chat_logs failed: {:?}", e);
            }
        }

        match get_chat_logs_desc(&self.endpoint, &self.token, &topic_id, last_seq, limit).await {
            Ok((mut lr, _)) => {
                let now = now_millis();
                for c in lr.items.iter_mut() {
                    c.cached_at = now;
                    c.status = if c.sender_id == self.user_id {
                        ChatLogStatus::Sent
                    } else {
                        ChatLogStatus::Received
                    };
                    self.store.save_chat_log(&c).await.ok();
                }
                callback.on_success(lr.into());
            }
            Err(e) => {
                warn!("sync_chat_logs failed: {:?}", e);
                callback.on_fail(e);
            }
        };
    }

    pub async fn sync_conversations(
        &self,
        updated_at: Option<String>,
        limit: u32,
        sync_logs: bool,
        callback: Box<dyn SyncConversationsCallback>,
    ) {
        let store_ref = self.store.clone();
        let limit = if limit == 0 {
            MAX_CONVERSATION_LIMIT
        } else {
            limit
        }
        .max(MAX_CONVERSATION_LIMIT);

        let updated_at = updated_at.unwrap_or_default();
        if !updated_at.is_empty() {
            if let Ok(t) = chrono::DateTime::parse_from_rfc3339(&updated_at) {
                if t.timestamp_millis() > 0
                    && now_millis() - t.timestamp_millis()
                        <= 1000 * crate::CONVERSATION_CACHE_EXPIRE_SECS
                {
                    match self.store.get_conversations(&updated_at, limit).await {
                        Ok(r) => {
                            if r.items.len() == limit as usize {
                                let updated_at = r
                                    .items
                                    .last()
                                    .map(|c| c.updated_at.clone())
                                    .unwrap_or_default();

                                if sync_logs {
                                    for c in r.items.iter() {
                                        self.try_sync_chat_logs(c.topic_id.clone(), None, 0).await
                                    }
                                }
                                let count = r.items.len() as u32;
                                if let Some(cb) = store_ref.callback.lock().unwrap().as_ref() {
                                    cb.on_conversations_updated(r.items);
                                }
                                callback.on_success(updated_at, count as u32);
                                return;
                            }
                        }
                        Err(e) => {
                            warn!("sync_conversations failed: {:?}", e);
                        }
                    }
                }
            }
        }

        let mut first_updated_at: Option<String> = None;
        let limit = MAX_CONVERSATION_LIMIT;
        let mut count = 0;
        let mut offset = 0;

        loop {
            let r =
                get_conversations(&self.endpoint, &self.token, &updated_at, offset, limit).await;
            match r {
                Ok(lr) => {
                    count += lr.items.len();
                    offset = lr.offset;
                    if first_updated_at.is_none() && !lr.items.is_empty() {
                        first_updated_at = Some(lr.items.first().unwrap().updated_at.clone());
                    }
                    let conversations = store_ref.merge_conversations(lr.items).await;

                    if sync_logs {
                        for c in conversations.iter() {
                            self.try_sync_chat_logs(c.topic_id.clone(), None, 0).await
                        }
                    }

                    if let Some(cb) = store_ref.callback.lock().unwrap().as_ref() {
                        cb.on_conversations_updated(conversations);
                    }
                    if !lr.has_more {
                        callback.on_success(first_updated_at.unwrap_or_default(), count as u32);
                        break;
                    }
                }
                Err(e) => {
                    warn!("sync_conversations failed: {:?}", e);
                    callback.on_fail(e);
                    break;
                }
            }
        }
    }

    pub async fn get_conversation(&self, topic_id: String, blocking: bool) -> Option<Conversation> {
        self.store.get_conversation(&topic_id, blocking).await
    }

    pub async fn remove_conversation(&self, topic_id: String) {
        self.store.remove_conversation(&topic_id).await
    }

    pub async fn set_conversation_remark(
        &self,
        topic_id: String,
        remark: Option<String>,
    ) -> Result<Conversation> {
        self.store.set_conversation_remark(&topic_id, remark).await
    }

    pub async fn set_conversation_sticky(
        &self,
        topic_id: String,
        sticky: bool,
    ) -> Result<Conversation> {
        self.store.set_conversation_sticky(&topic_id, sticky).await
    }

    pub async fn set_conversation_mute(
        &self,
        topic_id: String,
        mute: bool,
    ) -> Result<Conversation> {
        self.store.set_conversation_mute(&topic_id, mute).await
    }

    pub async fn set_conversation_read(&self, topic_id: String) {
        self.store.set_conversation_read(&topic_id).await
    }

    pub async fn set_conversation_tags(
        &self,
        topic_id: String,
        tags: Option<Tags>,
    ) -> Result<Conversation> {
        self.store.set_conversation_tags(&topic_id, tags).await
    }

    pub async fn set_conversation_extra(
        &self,
        topic_id: String,
        extra: Option<Extra>,
    ) -> Result<Conversation> {
        self.store.set_conversation_extra(&topic_id, extra).await
    }
}

impl Client {
    pub async fn filter_conversation(
        &self,
        predicate: Box<dyn Fn(Conversation) -> Option<Conversation> + Send>,
    ) -> Option<Vec<Conversation>> {
        self.store.filter_conversation(predicate).await
    }
}
