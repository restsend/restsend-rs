use anyhow::Result;

pub async fn get_conversations(
    endpoint: &str,
    token: &str,
    updated_at: String,
    limit: u32,
) -> Result<ListConversationResult> {
    let mut data = serde_json::json!({ "limit": limit });
    if !updated_at.is_empty() {
        data["updatedAt"] = updated_at.into();
    }
    let mut lr: ListConversationResult = self.json_call("/api/chat/list", data, None)?;
    for c in lr.items.iter_mut() {
        c.cached_at = chrono::Utc::now().to_rfc3339();
        self.db.save_conversation(c)?;
    }
    Ok(lr)
}

pub async fn get_conversation(
    endpoint: &str,
    token: &str,
    topic_id: String,
) -> Result<Conversation> {
    if let Ok(cached_conversation) = self.db.get_conversation(&topic_id) {
        if !is_expired(&cached_conversation.cached_at, CONVERSATION_CACHE) {
            return Ok(cached_conversation);
        }
    }

    self.json_call(
        &format!("/api/chat/info/{}", topic_id),
        serde_json::json!({}),
        None,
    )
    .and_then(|mut c: Conversation| {
        c.cached_at = chrono::Utc::now().to_rfc3339();

        self.db.save_conversation(&c)?;
        Ok(c)
    })
}

pub async fn remove_conversation(endpoint: &str, token: &str, topic_id: String) -> Result<()> {
    self.json_call::<CommonResp>(
        &format!("/api/chat/remove/{}", topic_id),
        serde_json::json!({}),
        None,
    )
    .and_then(|_| self.db.remove_conversation(&topic_id))?;

    if let Some(cb) = self.callback.read().unwrap().as_ref() {
        cb.on_conversation_removed(topic_id);
    }
    Ok(())
}

pub async fn update_conversation(
    endpoint: &str,
    token: &str,
    topic_id: &str,
    data: &serde_json::Value,
) -> Result<()> {
    self.json_call::<CommonResp>(
        &format!("/api/chat/update/{}", topic_id),
        data.clone(),
        None,
    )
    .and(Ok(()))
}

//置顶会话
pub async fn set_conversation_sticky(
    endpoint: &str,
    token: &str,
    topic_id: String,
    sticky: bool,
) -> Result<()> {
    let mut data = serde_json::json!({});
    data["sticky"] = sticky.into();
    self.update_conversation(&topic_id, &data)?;
    self.db.set_conversation_sticky(&topic_id, sticky)
}

pub async fn set_conversation_mute(
    endpoint: &str,
    token: &str,
    topic_id: String,
    mute: bool,
) -> Result<()> {
    let mut data = serde_json::json!({});
    data["mute"] = mute.into();
    self.update_conversation(&topic_id, &data)?;
    self.db.set_conversation_mute(&topic_id, mute)
}

pub async fn sync_conversations(endpoint: &str, token: &str, without_cache: bool) -> Result<()> {
    self.ctrl_tx
        .send(super::CtrlMessageType::ConversationSync(without_cache))
        .map_err(|e| crate::error::ClientError::SendCtrlMessageError(e.to_string()))
}

pub async fn begin_sync_conversations(
    endpoint: &str,
    token: &str,
    without_cache: bool,
) -> Result<()> {
    let mut conversations_sync_at = match without_cache {
        true => String::default(),
        false => match self.db.get_value(KEY_CONVERSATIONS_SYNC_AT) {
            Ok(v) => {
                if is_expired(&v, CONVERSATIONS_SYNC) {
                    String::default()
                } else {
                    v
                }
            }
            Err(_) => String::default(),
        },
    };

    loop {
        let lr = self.get_conversations(conversations_sync_at, LIMIT)?;
        if let Some(cb) = self.callback.read().unwrap().as_ref() {
            cb.on_conversation_updated(lr.items)
        }
        conversations_sync_at = lr.updated_at;
        if !lr.has_more {
            break;
        }
    }
    self.db
        .set_value(KEY_CONVERSATIONS_SYNC_AT, &conversations_sync_at)?;
    Ok(())
}

pub async fn set_conversation_read(endpoint: &str, token: &str, topic_id: String) -> Result<()> {
    self.db.set_conversation_read(&topic_id)?;
    let r: CommonResp = self.json_call(
        &format!("/api/chat/read/{}", topic_id),
        serde_json::json!({}),
        None,
    )?;
    Ok(())
}

pub async fn clean_history(
    endpoint: &str,
    token: &str,
    topic_id: String,
    sync: bool,
) -> Result<()> {
    let vals = serde_json::json!({});
    let r: CommonResp = self.json_call(
        &format!("/api/chat/clear_messages/{}", topic_id),
        vals,
        None,
    )?;
    Ok(())
}

pub async fn remove_messages(
    endpoint: &str,
    token: &str,
    topic_id: String,
    chat_ids: Vec<String>,
    sync: bool,
) -> Result<()> {
    let vals = serde_json::json!({ "ids": chat_ids });
    let r: CommonResp = self.json_call(
        &format!("/api/chat/remove_messages/{}", topic_id),
        vals,
        None,
    )?;
    Ok(())
}

pub async fn sync_chatlogs(
    endpoint: &str,
    token: &str,
    topic_id: String,
    start_seq: u64,
    end_seq: u64,
) -> Result<()> {
    self.ctrl_tx
        .send(super::CtrlMessageType::ChatLogSync(
            topic_id, start_seq, end_seq,
        ))
        .map_err(|e| crate::error::ClientError::SendCtrlMessageError(e.to_string()))
}

pub async fn begin_sync_chatlogs(
    endpoint: &str,
    token: &str,
    topic_id: String,
    start_seq: u64,
    end_seq: u64,
) -> Result<()> {
    loop {
        let lr = self.get_chat_logs_desc(topic_id.clone(), start_seq, end_seq)?;
        let has_more = lr.has_more;
        let chat_logs_sync_at = lr.updated_at.clone();
        debug!(
            "sync logs start_seq:{} end_seq:{} counts:{} last_seq:{}",
            start_seq,
            end_seq,
            lr.items.len(),
            lr.last_seq,
        );

        if let Some(cb) = self.callback.read().unwrap().as_ref() {
            cb.on_topic_logs_sync(topic_id.clone(), lr);
        }

        if !has_more {
            self.db.set_value(&topic_id, &chat_logs_sync_at)?;
            break;
        }
    }
    Ok(())
}

// 倒序的获取聊天记录
pub async fn get_chat_logs_desc(
    endpoint: &str,
    token: &str,
    topic_id: String,
    start_seq: u64,
    end_seq: u64,
) -> Result<ListChatLogResult> {
    let data = serde_json::json!({
        "topicId": topic_id,
        "lastSeq": start_seq,
        "maxSeq": end_seq,
        "limit": LIMIT
    });

    let mut lr: ListChatLogResult =
        self.json_call(&format!("/api/chat/sync/{}", topic_id), data, None)?;

    lr.items.iter_mut().try_for_each(|c| -> Result<()> {
        c.cached_at = chrono::Utc::now().to_rfc3339();
        self.db.save_chat_log(&c).ok();
        Ok(())
    })?;

    let last_seq = lr.items.iter().map(|c| c.seq).max().unwrap_or(0);

    debug!(
        "get logs start_seq:{} end_seq:{} counts:{} last_seq:{}",
        start_seq,
        end_seq,
        lr.items.len(),
        last_seq
    );
    return Ok(lr);
}

pub async fn get_chat_log(
    endpoint: &str,
    token: &str,
    topic_id: String,
    id: String,
) -> Result<ChatLog> {
    self.db.get_chat_log(&topic_id, &id)
}
