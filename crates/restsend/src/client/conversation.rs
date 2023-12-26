use super::Client;
use crate::callback::{SyncChatLogsCallback, SyncConversationsCallback};
use crate::models::conversation::{Extra, Tags};
use crate::models::{ChatLog, ChatLogStatus, Conversation, GetChatLogsResult};
use crate::services::conversation::create_chat;
use crate::services::conversation::{
    clean_messages, get_chat_logs_desc, get_conversations, remove_messages,
};
use crate::utils::{now_millis, spwan_task};
use crate::{Result, MAX_CONVERSATION_LIMIT, MAX_LOGS_LIMIT};
use log::warn;
use restsend_macros::export_wasm_or_ffi;

#[export_wasm_or_ffi]
impl Client {
    pub async fn create_chat(&self, user_id: String) -> Result<Conversation> {
        let conversation = create_chat(&self.endpoint, &self.token, &user_id).await?;
        self.store.update_conversation(conversation)
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
        self.store.remove_messages(&topic_id, &chat_ids);

        if sync_to_server {
            remove_messages(&self.endpoint, &self.token, &topic_id, chat_ids).await
        } else {
            Ok(())
        }
    }

    pub fn get_chat_log(&self, topic_id: String, chat_id: String) -> Option<ChatLog> {
        self.store.get_chat_log(&topic_id, &chat_id)
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

    pub fn sync_chat_logs(
        &self,
        topic_id: String,
        last_seq: i64,
        limit: u32,
        callback: Box<dyn SyncChatLogsCallback>,
    ) {
        let limit = if limit == 0 { MAX_LOGS_LIMIT } else { limit };
        match self.store.get_chat_logs(&topic_id, last_seq, limit) {
            Ok(local_logs) => {
                if local_logs.items.len() == limit as usize {
                    let r = GetChatLogsResult {
                        has_more: local_logs.end_sort_value > 1,
                        start_seq: local_logs.start_sort_value,
                        end_seq: local_logs.end_sort_value,
                        items: local_logs.items,
                    };
                    callback.on_success(r);
                    return;
                }
                warn!(
                    "sync_chat_logs from local_logs.items.len(): {} last_seq: {} limit: {}",
                    local_logs.items.len(),
                    last_seq,
                    limit,
                );
            }
            Err(e) => {
                warn!("sync_chat_logs failed: {:?}", e);
            }
        }
        let endpoint = self.endpoint.clone();
        let token = self.token.clone();

        let current_user_id = self.user_id.clone();
        let topic_id = topic_id.to_string();
        let store_ref = self.store.clone();

        spwan_task(async move {
            match get_chat_logs_desc(&endpoint, &token, &topic_id, last_seq, limit).await {
                Ok((lr, _)) => {
                    let now = now_millis();
                    let mut items = Vec::new();
                    for c in &lr.items {
                        let mut c = c.clone();
                        c.cached_at = now;
                        c.status = if c.sender_id == current_user_id {
                            ChatLogStatus::Sent
                        } else {
                            ChatLogStatus::Received
                        };
                        match store_ref.save_chat_log(&c) {
                            Ok(_) => items.push(c),
                            Err(_) => {}
                        };
                    }

                    let r = GetChatLogsResult {
                        has_more: lr.has_more,
                        start_seq: lr.items.first().map(|c| c.seq).unwrap_or(0),
                        end_seq: lr.items.last().map(|c| c.seq).unwrap_or(0),
                        items,
                    };
                    callback.on_success(r);
                }
                Err(e) => {
                    warn!("sync_chat_logs failed: {:?}", e);
                    callback.on_fail(e);
                }
            };
        });
    }

    pub fn sync_conversations(
        &self,
        updated_at: Option<String>,
        limit: u32,
        callback: Box<dyn SyncConversationsCallback>,
    ) {
        let store_ref = self.store.clone();
        let limit = if limit == 0 {
            MAX_CONVERSATION_LIMIT
        } else {
            limit
        };

        let local_updated_at = self.store.get_last_conversation_updated_at();
        let updated_at = updated_at.unwrap_or_default();
        if !updated_at.is_empty() && local_updated_at.is_some() {
            match self.store.get_conversations(&updated_at, limit) {
                Ok(r) => {
                    if r.items.len() == limit as usize {
                        let updated_at = r
                            .items
                            .last()
                            .map(|c| c.updated_at.clone())
                            .unwrap_or_default();
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

        let endpoint = self.endpoint.clone();
        let token = self.token.clone();

        spwan_task(async move {
            let mut first_updated_at: Option<String> = None;
            let limit = MAX_CONVERSATION_LIMIT;
            let mut count = 0;
            let mut offset = 0;

            loop {
                let r = get_conversations(&endpoint, &token, &updated_at, offset, limit)
                    .await
                    .map(|lr| {
                        count += lr.items.len();
                        offset = lr.offset;
                        if first_updated_at.is_none() && !lr.items.is_empty() {
                            first_updated_at = Some(lr.items.first().unwrap().updated_at.clone());
                        }
                        let conversations = store_ref.merge_conversations(lr.items);
                        if let Some(cb) = store_ref.callback.lock().unwrap().as_ref() {
                            cb.on_conversations_updated(conversations);
                        }
                        lr.has_more
                    });
                match r {
                    Ok(has_more) => {
                        if !has_more {
                            callback.on_success(first_updated_at.unwrap_or_default(), count as u32);
                            break;
                        }
                    }
                    Err(e) => {
                        warn!("sync_all_conversations failed: {:?}", e);
                        callback.on_fail(e);
                        break;
                    }
                }
            }
        });
    }

    pub fn get_conversation(&self, topic_id: String) -> Option<Conversation> {
        self.store.get_conversation(&topic_id)
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
    pub fn filter_conversation(
        &self,
        predicate: Box<dyn Fn(Conversation) -> Option<Conversation>>,
    ) -> Vec<Conversation> {
        self.store.filter_conversation(predicate)
    }
}
