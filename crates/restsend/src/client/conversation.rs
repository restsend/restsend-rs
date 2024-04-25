use std::collections::HashMap;

use super::Client;
use crate::callback::{SyncChatLogsCallback, SyncConversationsCallback};
use crate::models::conversation::{Extra, Tags};
use crate::models::{ChatLog, ChatLogStatus, Conversation, GetChatLogsResult};
use crate::request::ChatRequest;
use crate::services::conversation::{
    batch_get_chat_logs_desc, create_chat, set_conversation_read, BatchSyncChatLogs,
};
use crate::services::conversation::{
    clean_messages, get_chat_logs_desc, get_conversations, remove_messages,
};
use crate::utils::{elapsed, now_millis};
use crate::{Result, MAX_CONVERSATION_LIMIT, MAX_LOGS_LIMIT, MAX_SYNC_LOGS_MAX_COUNT};
use log::{info, warn};
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

    #[allow(unused)]
    pub(crate) async fn try_sync_chat_logs(
        &self,
        mut conversation: Conversation,
        limit: Option<u32>,
    ) -> Option<Conversation> {
        match self.store.get_last_log(&conversation.topic_id).await {
            Some(log) => {
                if log.seq >= conversation.last_seq {
                    log::debug!(
                        "try_sync_chat_logs skip, log.seq: {} >= conversation.last_seq: {}",
                        log.seq,
                        conversation.last_seq
                    );
                    return None;
                }
            }
            None => {}
        }

        if conversation.last_seq <= conversation.start_seq {
            return None;
        }

        let r = self
            .fetch_chat_logs_desc(
                conversation.topic_id.clone(),
                Some(conversation.last_seq),
                limit.unwrap_or_default(),
            )
            .await
            .ok()?;

        for c in r.items.iter() {
            if !c.content.unreadable {
                conversation.updated_at = c.created_at.clone();
                conversation.last_message_at = c.created_at.clone();
                conversation.last_message = Some(c.content.clone());
                conversation.last_sender_id = c.sender_id.clone();
                break;
            }
        }

        let last_seq = r.items.first().map(|c| c.seq).unwrap_or_default();
        if last_seq > conversation.last_seq {
            conversation.last_seq = last_seq;
        }

        Some(conversation)
    }

    pub async fn sync_chat_logs(
        &self,
        topic_id: String,
        last_seq: Option<i64>,
        limit: u32,
        callback: Box<dyn SyncChatLogsCallback>,
    ) {
        let limit = if limit == 0 { MAX_LOGS_LIMIT } else { limit }.min(MAX_LOGS_LIMIT);
        let st = now_millis();
        let conversation = self
            .store
            .get_conversation(&topic_id, true)
            .await
            .unwrap_or_default();

        match self.store.get_chat_logs(&topic_id, last_seq, limit).await {
            Ok(local_logs) => {
                let mut need_fetch = local_logs.items.len() < limit as usize;

                if need_fetch
                    && local_logs.items.len() > 0
                    && local_logs.items.len() < limit as usize
                {
                    need_fetch = local_logs.end_sort_value != conversation.start_seq + 1;
                }

                info!(
                    "sync_chat_logs local_logs.len: {} start_seq: {} last_seq: {:?} limit: {} local_logs.end_sort_value:{} need_fetch:{} usage:{:?}",
                    local_logs.items.len(),
                    conversation.start_seq,
                    last_seq,
                    limit,
                    local_logs.end_sort_value,
                    need_fetch,
                    elapsed(st)
                );

                if !need_fetch {
                    callback.on_success(GetChatLogsResult::from_local_logs(
                        local_logs,
                        conversation.start_seq,
                    ));
                    return;
                }
            }
            Err(e) => {
                warn!("sync_chat_logs failed: {:?}", e);
            }
        }

        match self.fetch_chat_logs_desc(topic_id, last_seq, limit).await {
            Ok(lr) => {
                callback.on_success(lr);
            }
            Err(e) => {
                callback.on_fail(e);
            }
        }
    }

    async fn fetch_chat_logs_desc(
        &self,
        topic_id: String,
        last_seq: Option<i64>,
        limit: u32,
    ) -> Result<GetChatLogsResult> {
        match get_chat_logs_desc(&self.endpoint, &self.token, &topic_id, last_seq, limit).await {
            Ok(mut lr) => {
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
                info!(
                    "fetch_chat_logs_desc topic_id: {} last_seq: {:?} limit: {} items.len: {} usage:{:?}",
                    topic_id,
                    last_seq,
                    limit,
                    lr.items.len(),
                    elapsed(now)
                );
                Ok(lr.into())
            }
            Err(e) => {
                warn!("sync_chat_logs failed: {:?}", e);
                Err(e)
            }
        }
    }
    // sync workflows:
    // 1. load all conversations from local db
    // 2. fetch conversations from server with updated_at until has_more is false
    //     2.1. merge conversations from server to local db
    // 3. if sync_logs is true, sync chat logs for all conversations
    // 1. load all conversations from local db
    pub async fn sync_conversations(
        &self,
        updated_at: Option<String>,
        limit: u32,
        sync_logs: bool,
        sync_logs_limit: Option<u32>,
        sync_logs_max_count: Option<u32>,
        callback: Box<dyn SyncConversationsCallback>,
    ) {
        let store_ref = self.store.clone();
        let limit = match limit {
            0 => MAX_CONVERSATION_LIMIT,
            _ => limit,
        }
        .min(MAX_CONVERSATION_LIMIT);
        let sync_logs_max_count = sync_logs_max_count.unwrap_or(MAX_SYNC_LOGS_MAX_COUNT);

        let fetch_local = updated_at.clone().and_then(|s| {
            chrono::DateTime::parse_from_rfc3339(&s).ok().and_then(|t| {
                Some(
                    t.timestamp_millis() > 0
                        && now_millis() - t.timestamp_millis()
                            <= 1000 * crate::CONVERSATION_CACHE_EXPIRE_SECS,
                )
            })
        });

        let first_updated_at = updated_at.clone().unwrap_or_default();
        let mut conversations = HashMap::new();

        if fetch_local.unwrap_or(false) {
            loop {
                match store_ref.get_conversations(&first_updated_at, limit).await {
                    Ok(r) => {
                        r.items.iter().for_each(|c| {
                            conversations.insert(c.topic_id.clone(), c.clone());
                        });

                        if let Some(cb) = store_ref.callback.lock().unwrap().as_ref() {
                            cb.on_conversations_updated(r.items);
                        }
                    }
                    Err(_) => break,
                }
            }
        }

        let mut offset = 0;
        let mut last_updated_at = updated_at.clone().unwrap_or_default();
        let updated_at = updated_at.unwrap_or_default();

        loop {
            match get_conversations(&self.endpoint, &self.token, &updated_at, offset, limit).await {
                Ok(lr) => {
                    offset = lr.offset;

                    if last_updated_at.is_empty() && !lr.items.is_empty() {
                        last_updated_at = lr.items.first().unwrap().updated_at.clone();
                    }

                    let new_conversations = store_ref.merge_conversations(lr.items).await;
                    log::info!(
                        "sync conversations from remote, count: {}",
                        new_conversations.len(),
                    );
                    new_conversations.iter().for_each(|c| {
                        conversations.insert(c.topic_id.clone(), c.clone());
                    });

                    if let Some(cb) = store_ref.callback.lock().unwrap().as_ref() {
                        cb.on_conversations_updated(new_conversations);
                    }
                    if !lr.has_more {
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

        let count = conversations.len() as u32;
        if sync_logs {
            let mut vals: Vec<_> = conversations.into_iter().map(|it| it.1).collect();
            vals.sort_by(|a, b| {
                a.updated_at
                    .parse::<chrono::DateTime<chrono::Local>>()
                    .unwrap()
                    .cmp(
                        &b.updated_at
                            .parse::<chrono::DateTime<chrono::Local>>()
                            .unwrap(),
                    )
            });

            if sync_logs_max_count > 0 {
                vals.truncate(sync_logs_max_count as usize);
            }

            let conversations = vals.into_iter().map(|c| (c.topic_id.clone(), c)).collect();
            self.batch_sync_chatlogs(conversations, sync_logs_limit)
                .await
                .map_err(|e| {
                    warn!("sync_conversations failed: {:?}", e);
                })
                .ok();
        }
        callback.on_success(last_updated_at, count);
    }

    pub async fn batch_sync_chatlogs(
        &self,
        mut conversations: HashMap<String, Conversation>,
        limit: Option<u32>,
    ) -> Result<()> {
        loop {
            let mut try_sync_conversations = vec![];
            for (_, c) in conversations.iter() {
                match self.store.get_last_log(&c.topic_id).await {
                    Some(log) => {
                        if log.seq >= c.last_seq {
                            continue;
                        }
                    }
                    None => {}
                }

                if c.last_seq <= c.start_seq {
                    continue;
                }
                try_sync_conversations.push(c.clone());
            }

            if try_sync_conversations.is_empty() {
                return Ok(());
            }

            let form = try_sync_conversations
                .iter()
                .map(|c| BatchSyncChatLogs {
                    topic_id: c.topic_id.clone(),
                    last_seq: Some(c.last_seq),
                    limit,
                })
                .collect();

            let r = batch_get_chat_logs_desc(&self.endpoint, &self.token, form).await?;
            let now = now_millis();

            let mut updated_conversations = vec![];
            for mut lr in r {
                // flush to local db
                for c in lr.items.iter_mut() {
                    c.cached_at = now;
                    c.status = if c.sender_id == self.user_id {
                        ChatLogStatus::Sent
                    } else {
                        ChatLogStatus::Received
                    };
                    self.store.save_chat_log(&c).await.ok();
                }

                let topic_id = match lr.topic_id {
                    Some(ref topic_id) => topic_id.clone(),
                    None => continue,
                };

                let mut conversation = match conversations.remove(&topic_id) {
                    Some(c) => c,
                    None => continue,
                };

                for c in lr.items.iter() {
                    if c.seq <= conversation.last_seq {
                        continue;
                    }
                    conversation.last_seq = c.seq;
                    if !c.content.unreadable {
                        conversation.updated_at = c.created_at.clone();
                        conversation.last_message_at = c.created_at.clone();
                        conversation.last_message = Some(c.content.clone());
                        conversation.last_sender_id = c.sender_id.clone();
                    }
                    break;
                }
                updated_conversations.push(conversation);
            }
            // callback
            if let Some(cb) = self.store.callback.lock().unwrap().as_ref() {
                cb.on_conversations_updated(updated_conversations.clone());
            }
            conversations = updated_conversations
                .iter()
                .map(|c| (c.topic_id.clone(), c.clone()))
                .collect();
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

    pub async fn set_conversation_read(&self, topic_id: String, heavy: bool) {
        let last_read_at = chrono::Local::now().to_rfc3339();
        self.store
            .set_conversation_read_local(&topic_id, &last_read_at, None)
            .await
            .map(|c| {
                let mut msg = ChatRequest::new_read(&topic_id);
                msg.seq = c.last_seq;
                msg.created_at = last_read_at;
                self.store.emit_topic_read(topic_id.clone(), msg)
            });
        if heavy {
            set_conversation_read(&self.endpoint, &self.token, &topic_id).await
        } else {
            self.do_read(topic_id).await
        }
        .ok();
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
        end_sort_value: Option<i64>,
        limit: Option<u32>,
    ) -> Option<Vec<Conversation>> {
        self.store
            .filter_conversation(predicate, end_sort_value, limit)
            .await
    }
}
