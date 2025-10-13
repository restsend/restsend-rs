use super::Client;
use crate::callback::{SyncChatLogsCallback, SyncConversationsCallback};
use crate::client::store::is_cache_expired;
use crate::models::conversation::{Extra, Tags};
use crate::models::{ChatLog, ChatLogStatus, Conversation, GetChatLogsResult};
use crate::request::ChatRequest;
use crate::services::conversation::{
    batch_get_chat_logs_desc, create_chat, set_all_conversations_read, set_conversation_read,
    BatchSyncChatLogs,
};
use crate::services::conversation::{
    clean_messages, get_chat_logs_desc, get_conversations, remove_messages,
};
use crate::storage::{StoreModel, ValueItem};
use crate::utils::{elapsed, now_millis};
use crate::{
    Result, CONVERSATION_CACHE_EXPIRE_SECS, MAX_CONVERSATION_LIMIT, MAX_LOGS_LIMIT,
    MAX_SYNC_LOGS_MAX_COUNT,
};
use log::{info, warn};
use restsend_macros::export_wasm_or_ffi;
use std::collections::HashMap;

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
    pub async fn sync_chat_logs_quick(
        &self,
        topic_id: String,
        last_seq: Option<i64>,
        limit: u32,
        callback: Box<dyn SyncChatLogsCallback>,
        ensure_conversation_last_version: Option<bool>,
    ) {
        let st = now_millis();
        let limit = if limit == 0 {
            MAX_LOGS_LIMIT / 2
        } else {
            limit
        }
        .min(MAX_LOGS_LIMIT);
        let mut need_fetch_conversation = ensure_conversation_last_version.unwrap_or(false);
        let conversation = self
            .store
            .message_storage
            .readonly_table::<Conversation>()
            .await
            .get("", &topic_id)
            .await;

        match conversation {
            Some(conversation) => {
                if !need_fetch_conversation {
                    need_fetch_conversation = conversation.is_partial
                        || is_cache_expired(conversation.cached_at, CONVERSATION_CACHE_EXPIRE_SECS);
                }

                if !need_fetch_conversation {
                    let t = self.store.message_storage.readonly_table::<ChatLog>().await;
                    match t.last(&topic_id).await {
                        Some(log) => {
                            if log.seq != conversation.last_seq {
                                need_fetch_conversation = true;
                            }
                        }
                        None => {}
                    }
                }

                if !need_fetch_conversation {
                    let store_st = now_millis();
                    match self
                        .store
                        .get_chat_logs(&topic_id, conversation.start_seq, last_seq, limit)
                        .await
                    {
                        Ok((local_logs, mut need_fetch_logs)) => {
                            info!("sync_chat_logs_quick has_more:{} local_logs.len: {} start_seq: {} last_seq: {:?} limit: {} local_logs.start_sort_value:{} local_logs.end_sort_value:{} need_fetch:{} store_cost:{:?} total_cost:{:?}",
                            local_logs.has_more,
                            local_logs.items.len(),
                            conversation.start_seq,
                            last_seq,
                            limit,
                            local_logs.start_sort_value,
                            local_logs.end_sort_value,
                            need_fetch_logs,
                            elapsed(store_st),
                            elapsed(st)
                        );

                            if last_seq.is_none()
                                && conversation.last_seq > local_logs.start_sort_value
                            {
                                need_fetch_logs = true;
                            }

                            let has_more = local_logs.has_more;
                            if local_logs.items.len() == 0 {
                                need_fetch_logs = true;
                            }
                            if !need_fetch_logs {
                                callback.on_success(GetChatLogsResult::from_local_logs(
                                    local_logs, has_more,
                                ));
                                return;
                            }
                        }
                        Err(_) => {}
                    };
                }
            }
            None => {}
        }

        self.fetch_chat_logs_desc(&topic_id, last_seq, limit, callback)
            .await;

        if need_fetch_conversation {
            self.store
                .get_conversation_by(Conversation::new(&topic_id), true)
                .await;
        }
    }

    pub async fn sync_chat_logs_heavy(
        &self,
        topic_id: String,
        last_seq: Option<i64>,
        limit: u32,
        callback: Box<dyn SyncChatLogsCallback>,
        ensure_conversation_last_version: Option<bool>,
    ) {
        let st = now_millis();
        let limit = if limit == 0 {
            MAX_LOGS_LIMIT / 2
        } else {
            limit
        }
        .min(MAX_LOGS_LIMIT);

        let conversation = self
            .store
            .get_conversation(
                &topic_id,
                false,
                ensure_conversation_last_version.unwrap_or(false),
            )
            .await
            .unwrap_or(Conversation::new(&topic_id));

        let store_st = now_millis();
        match self
            .store
            .get_chat_logs(&topic_id, conversation.start_seq, last_seq, limit)
            .await
        {
            Ok((local_logs, need_fetch)) => {
                let has_more = local_logs.end_sort_value > conversation.start_seq + 1;
                info!(
                    "sync_chat_logs_heavy has_more: {} local_logs.len: {} start_seq: {} last_seq: {:?} limit: {} local_logs.start_sort_value:{} local_logs.end_sort_value:{} need_fetch:{} store_cost:{:?} total_cost:{:?}",
                    has_more,
                    local_logs.items.len(),
                    conversation.start_seq,
                    last_seq,
                    limit,
                    local_logs.start_sort_value,
                    local_logs.end_sort_value,
                    need_fetch,
                    elapsed(store_st),
                    elapsed(st)
                );
                if !need_fetch {
                    callback.on_success(GetChatLogsResult::from_local_logs(local_logs, has_more));
                    if conversation.is_partial
                        || is_cache_expired(conversation.cached_at, CONVERSATION_CACHE_EXPIRE_SECS)
                    {
                        self.store.get_conversation_by(conversation, true).await;
                    }
                    return;
                }
            }
            Err(e) => {
                warn!("sync_chat_logs failed: {:?}", e);
            }
        }
        self.fetch_chat_logs_desc(&topic_id, last_seq, limit, callback)
            .await;
    }

    pub async fn save_chat_logs(&self, logs: &Vec<ChatLog>) -> Result<()> {
        let log_t = self.store.message_storage.table::<ChatLog>().await;
        self.store.save_chat_logs(&log_t, logs).await
    }

    async fn fetch_chat_logs_desc(
        &self,
        topic_id: &str,
        last_seq: Option<i64>,
        limit: u32,
        callback: Box<dyn SyncChatLogsCallback>,
    ) {
        let st_fetch = now_millis();
        match get_chat_logs_desc(&self.endpoint, &self.token, topic_id, last_seq, limit).await {
            Ok(mut lr) => {
                let now = now_millis();
                for c in lr.items.iter_mut() {
                    c.cached_at = now;
                    c.status = if c.sender_id == self.user_id {
                        ChatLogStatus::Sent
                    } else {
                        ChatLogStatus::Received
                    };
                }
                let items = lr.items.clone();
                callback.on_success(lr.into());
                let log_t = self.store.message_storage.table::<ChatLog>().await;
                self.store.save_chat_logs(&log_t, &items).await.ok();
                info!(
                    "fetch_chat_logs_desc topic_id: {} last_seq: {:?} limit: {} items.len: {} save_cost:{:?} total_cost:{:?}",
                    topic_id,
                    last_seq,
                    limit,
                    items.len(),
                    elapsed(now),
                    elapsed(st_fetch)
                );
            }
            Err(e) => {
                warn!("sync_chat_logs failed: {:?}", e);
                callback.on_fail(e);
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
        before_updated_at: Option<String>,
        sync_max_count: Option<u32>,
        limit: u32,
        sync_logs: bool,
        sync_logs_limit: Option<u32>,
        sync_logs_max_count: Option<u32>,
        last_removed_at: Option<String>,
        callback: Box<dyn SyncConversationsCallback>,
    ) {
        let store_ref = self.store.clone();
        let limit = match limit {
            0 => MAX_CONVERSATION_LIMIT,
            _ => limit,
        }
        .min(MAX_CONVERSATION_LIMIT);
        let sync_logs_max_count = sync_logs_max_count.unwrap_or(MAX_SYNC_LOGS_MAX_COUNT);

        let mut last_updated_at = updated_at.clone().unwrap_or_default();
        let mut conversations = HashMap::new();
        let sync_max_count = sync_max_count.unwrap_or(0);

        loop {
            match store_ref.get_conversations(&last_updated_at, limit).await {
                Ok(r) => {
                    if r.items.is_empty() {
                        break;
                    }
                    let item_len = r.items.len() as u32;
                    r.items.iter().for_each(|c| {
                        conversations.insert(c.topic_id.clone(), c.clone());
                    });
                    log::info!(
                        "sync conversations from local, item_len: {item_len} first_updated_at: {last_updated_at} has_more:{} limit: {limit} total:{}",
                        r.has_more,
                        conversations.len(),
                    );

                    last_updated_at = r
                        .items
                        .last()
                        .map(|c| c.updated_at.clone())
                        .unwrap_or_default();
                    if let Some(cb) = store_ref.callback.read().unwrap().as_ref() {
                        cb.on_conversations_updated(r.items, None);
                    }
                    if !r.has_more || item_len < limit {
                        break;
                    }
                }
                Err(_) => break,
            }
        }

        let mut offset = 0;
        let mut last_updated_at = updated_at.clone().unwrap_or_default();
        let updated_at = updated_at.unwrap_or_default();
        let mut last_updated_at_remote = before_updated_at;
        let mut last_removed_at = last_removed_at.clone();
        let mut total = 0;
        let mut sync_count_from_remote = 0;
        loop {
            let st_0 = now_millis();
            match get_conversations(
                &self.endpoint,
                &self.token,
                &updated_at,
                last_updated_at_remote.clone(),
                last_removed_at.clone(),
                offset,
                limit,
            )
            .await
            {
                Ok(lr) => {
                    total = lr.total as u32;
                    let api_cost = elapsed(st_0);

                    offset = if lr.last_updated_at.is_none() {
                        lr.offset
                    } else {
                        last_updated_at_remote = lr.last_updated_at.clone();
                        0
                    };
                    sync_count_from_remote += lr.items.len() as u32;
                    if last_updated_at.is_empty() && !lr.items.is_empty() {
                        last_updated_at = lr.items.first().unwrap().updated_at.clone();
                    }
                    if lr.last_removed_at.is_some() {
                        last_removed_at = lr.last_removed_at.clone();
                    }
                    let st = now_millis();

                    if lr.removed.len() > 0 {
                        for topic_id in lr.removed.iter() {
                            store_ref.sync_removed_conversation(&topic_id).await;
                        }
                    }

                    let new_conversations = store_ref.merge_conversations(lr.items).await;
                    let merge_cost = elapsed(st);
                    let st = now_millis();

                    new_conversations.iter().for_each(|c| {
                        conversations.insert(c.topic_id.clone(), c.clone());
                    });

                    let new_conversations_count = new_conversations.len() as u32;
                    if let Some(cb) = store_ref.callback.read().unwrap().as_ref() {
                        cb.on_conversations_updated(new_conversations, Some(lr.total));
                    }

                    log::info!(
                        "sync conversations from remote, count: {new_conversations_count} removed:{} has_more:{} api_cost:{:?} merge_cost: {:?}, callback_cost: {:?}, total_cost: {:?}, sync_max_count: {sync_max_count}",
                        lr.removed.len(),
                        lr.has_more,
                        api_cost,
                        merge_cost,
                        elapsed(st),
                        elapsed(st_0)
                    );
                    if !lr.has_more {
                        break;
                    }
                    if sync_max_count > 0 && sync_count_from_remote >= sync_max_count {
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
        let mut conversations: Vec<_> = conversations.into_iter().map(|it| it.1).collect();
        self.batch_build_unreads(&mut conversations).await;

        if sync_logs {
            conversations.sort_by(|a, b| {
                let a_updated_at =
                    chrono::DateTime::parse_from_rfc3339(&a.updated_at).unwrap_or_default();
                let b_updated_at =
                    chrono::DateTime::parse_from_rfc3339(&b.updated_at).unwrap_or_default();
                b_updated_at.cmp(&a_updated_at)
            });

            if sync_logs_max_count > 0 {
                conversations.truncate(sync_logs_max_count as usize);
            }

            for chunk in conversations.chunks(MAX_SYNC_LOGS_MAX_COUNT as usize) {
                let conversations = chunk
                    .into_iter()
                    .map(|c| (c.topic_id.clone(), c.clone()))
                    .collect();
                self.batch_sync_chatlogs(conversations, sync_logs_limit)
                    .await
                    .map_err(|e| {
                        warn!("sync_conversations failed: {:?}", e);
                    })
                    .ok();
            }
        }
        callback.on_success(last_updated_at, last_removed_at, count, total);
    }

    pub async fn batch_sync_chatlogs(
        &self,
        mut conversations: HashMap<String, Conversation>,
        limit: Option<u32>,
    ) -> Result<()> {
        let mut try_sync_conversations = vec![];
        {
            let log_t = self.store.message_storage.readonly_table::<ChatLog>().await;
            for (_, c) in conversations.iter() {
                match log_t.last(&c.topic_id).await {
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

        let mut updated_conversations = vec![];
        let mut store_conversations = vec![];

        let log_t = self.store.message_storage.table::<ChatLog>().await;
        for mut lr in r {
            // flush to local db
            let now: i64 = now_millis();
            for c in lr.items.iter_mut() {
                c.cached_at = now;
                c.status = if c.sender_id == self.user_id {
                    ChatLogStatus::Sent
                } else {
                    ChatLogStatus::Received
                };
                c.is_countable =
                    if let Some(cb) = self.store.countable_callback.read().unwrap().as_ref() {
                        cb.is_countable(c.content.clone())
                    } else {
                        !c.content.unreadable
                    };
            }
            self.store.save_chat_logs(&log_t, &lr.items).await.ok();

            let topic_id = match lr.topic_id {
                Some(ref topic_id) => topic_id.clone(),
                None => continue,
            };

            let mut conversation = match conversations.remove(&topic_id) {
                Some(c) => c,
                None => continue,
            };

            for c in lr.items.iter() {
                if c.is_countable && c.seq > conversation.last_read_seq {
                    conversation.unread += 1;
                }

                if c.is_countable && c.seq > conversation.last_seq {
                    conversation.last_seq = c.seq;
                    conversation.updated_at = c.created_at.clone();
                    conversation.last_message_at = c.created_at.clone();
                    conversation.last_message = Some(c.content.clone());
                    conversation.last_sender_id = c.sender_id.clone();
                    conversation.last_message_seq = Some(c.seq);
                }
            }
            updated_conversations.push(conversation.clone());

            store_conversations.push(ValueItem {
                partition: "".to_string(),
                key: conversation.topic_id.clone(),
                sort_key: conversation.sort_key(),
                value: Some(conversation),
            })
        }
        // sync to store
        let t = self.store.message_storage.table::<Conversation>().await;
        t.batch_update(&store_conversations).await.ok();
        // callback
        if let Some(cb) = self.store.callback.read().unwrap().as_ref() {
            cb.on_conversations_updated(updated_conversations.clone(), None);
        }
        Ok(())
    }

    pub async fn get_conversation(&self, topic_id: String, blocking: bool) -> Option<Conversation> {
        self.store.get_conversation(&topic_id, blocking, true).await
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
                let mut msg = ChatRequest::new_read(&topic_id, c.last_seq);
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

    pub async fn set_all_conversations_read(&self) {
        self.store.set_all_conversations_read_local().await;

        set_all_conversations_read(&self.endpoint, &self.token)
            .await
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
    pub async fn clear_conversation(&self, topic_id: String) -> Result<()> {
        self.store.clear_conversation(&topic_id).await
    }
}

impl Client {
    async fn batch_build_unreads(&self, conversations: &mut Vec<Conversation>) {
        let log_t = self.store.message_storage.readonly_table::<ChatLog>().await;
        let mut stored_conversations = vec![];
        let st = now_millis();
        // build conversatoin's unread
        for c in conversations.iter_mut() {
            if c.last_read_seq >= c.last_message_seq.unwrap_or(c.last_seq) {
                continue;
            }
            let start_seq = c.start_seq.max(c.last_read_seq);
            let unread_diff = c.last_seq - c.last_read_seq;
            if unread_diff <= 0 {
                continue;
            }

            if unread_diff >= MAX_LOGS_LIMIT as i64 {
                if c.unread < MAX_LOGS_LIMIT as i64 {
                    c.unread = unread_diff;
                    stored_conversations.push(ValueItem {
                        partition: "".to_string(),
                        key: c.topic_id.clone(),
                        sort_key: c.sort_key(),
                        value: Some(c.clone()),
                    });
                    continue;
                }
            }
            let logs = match self
                .store
                .get_chat_logs_with_table(
                    &log_t,
                    &c.topic_id,
                    start_seq,
                    Some(c.last_seq),
                    unread_diff.min(MAX_LOGS_LIMIT as i64) as u32,
                )
                .await
            {
                Ok((logs, _)) => logs,
                Err(_) => continue,
            };
            let mut unread = 0;
            for log in logs.items.iter() {
                let is_countable =
                    if let Some(cb) = self.store.countable_callback.read().unwrap().as_ref() {
                        cb.is_countable(log.content.clone())
                    } else {
                        !log.content.unreadable
                    };

                if is_countable && log.seq > c.last_read_seq {
                    unread += 1;
                }
            }
            if c.unread != unread {
                c.unread = unread;
                stored_conversations.push(ValueItem {
                    partition: "".to_string(),
                    key: c.topic_id.clone(),
                    sort_key: c.sort_key(),
                    value: Some(c.clone()),
                });
            }
        }
        let logs_db_cost = elapsed(st);
        let st_1 = now_millis();
        if !stored_conversations.is_empty() {
            let t = self.store.message_storage.table::<Conversation>().await;
            t.batch_update(&stored_conversations).await.ok();
        }
        log::info!(
            "batch_build_unreads conversations.len:{} need_update.len:{} logs.db_cost:{:?} conversations.db_cost:{:?} total_cost:{:?}",
            conversations.len(),
            stored_conversations.len(),
            logs_db_cost,
            elapsed(st_1),
            elapsed(st)
        );
    }
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
