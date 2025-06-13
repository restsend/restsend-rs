use super::{CallbackRef, ClientStore, ClientStoreRef, PendingRequest};
use crate::client::store::conversations::merge_conversation_from_chat;
use crate::client::store::is_cache_expired;
use crate::models::ChatLogStatus;
use crate::utils::now_millis;
use crate::{
    callback::MessageCallback,
    request::{ChatRequest, ChatRequestType},
};
use crate::{PING_TIMEOUT_SECS, REMOVED_CONVERSATION_CACHE_EXPIRE_SECS};
use http::StatusCode;
use log::{info, warn};
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};

impl ClientStore {
    pub async fn handle_outgoing(&self, outgoing_tx: UnboundedSender<ChatRequest>) {
        let (msg_tx, mut msg_rx) = unbounded_channel::<String>();
        self.msg_tx.write().unwrap().replace(msg_tx.clone());

        while let Some(chat_id) = msg_rx.recv().await {
            let outgoings = self.outgoings.read().unwrap();
            if let Some(pending) = outgoings.get(&chat_id) {
                if pending.is_expired() {
                    continue;
                }
                outgoing_tx.send(pending.req.clone()).ok();
                pending.callback.as_ref().map(|cb| cb.on_sent(chat_id));
            }
        }
    }

    pub async fn process_incoming(
        &self,
        req: ChatRequest,
        callback: CallbackRef,
    ) -> Vec<Option<ChatRequest>> {
        let content_type = match req.content.as_ref() {
            Some(content) => content.content_type.clone(),
            None => req.req_type.clone(),
        };

        let chat_id = if req.chat_id.is_empty() {
            "".to_string()
        } else {
            format!("chat_id:{} ", req.chat_id)
        };

        info!(
            "process_incoming, type:{} topic_id:{} seq:{} {}",
            content_type, req.topic_id, req.seq, chat_id
        );

        let topic_id = req.topic_id.clone();
        let chat_id = req.chat_id.clone();
        let ack_seq = req.seq.clone();

        match ChatRequestType::from(&req.req_type) {
            ChatRequestType::Response => {
                let status = if req.code == StatusCode::OK.as_u16() as u32 {
                    ChatLogStatus::Sent
                } else {
                    ChatLogStatus::SendFailed
                };

                if let Some(pending) = self.peek_pending_request(&req.chat_id).await {
                    match status {
                        ChatLogStatus::Sent => {
                            let mut req = req;
                            req.content = pending.req.content.clone();
                            pending.callback.map(|cb| cb.on_ack(req));
                        }
                        ChatLogStatus::SendFailed => {
                            let reason =
                                req.message.unwrap_or(format!("send failed: {}", req.code));
                            pending.callback.map(|cb| cb.on_fail(reason));
                        }
                        _ => {}
                    }
                } else {
                    if content_type == "ping" && req.seq == 0 {
                        match req.content.as_ref() {
                            Some(content) => {
                                let data = match serde_json::from_str::<serde_json::Value>(
                                    &content.text,
                                ) {
                                    Ok(data) => data,
                                    Err(_) => return vec![],
                                };
                                let timestamp = match data["timestamp"].as_i64() {
                                    Some(timestamp) => timestamp,
                                    None => return vec![],
                                };
                                let diff = now_millis() - timestamp;
                                if diff >= PING_TIMEOUT_SECS * 1000 {
                                    warn!("ping timeout:{}", diff);
                                }
                            }
                            _ => {}
                        }
                    }
                }

                self.update_outoing_chat_log_state(&topic_id, &chat_id, status, Some(ack_seq))
                    .await
                    .ok();
                vec![]
            }
            ChatRequestType::Chat => {
                match req.attendee_profile.as_ref() {
                    Some(profile) => {
                        self.update_user(profile.clone()).await.ok();
                    }
                    None => {}
                };

                let mut resps = vec![ChatRequest::new_response(&req, 200)];
                let topic_id = req.topic_id.clone();
                let removed_at = {
                    self.removed_conversations
                        .read()
                        .unwrap()
                        .get(&req.topic_id)
                        .copied()
                };
                if let Some(removed_at) = removed_at {
                    if !is_cache_expired(removed_at, REMOVED_CONVERSATION_CACHE_EXPIRE_SECS) {
                        return resps;
                    }
                    match self.removed_conversations.try_write() {
                        Ok(mut removed_conversations) => {
                            removed_conversations.remove(&req.topic_id);
                        }
                        Err(_) => {}
                    }
                }

                if let Err(e) = self.save_incoming_chat_log(&req).await {
                    warn!(
                        "save_incoming_chat_log failed, chat_id:{} topic_id:{} err:{}",
                        req.chat_id, req.topic_id, e
                    );
                    return resps;
                }

                let req_status = callback
                    .read()
                    .unwrap()
                    .as_ref()
                    .map(|cb| cb.on_new_message(topic_id.clone(), req.clone()))
                    .unwrap_or_default();

                let is_countable =
                    if let Some(cb) = self.countable_callback.read().unwrap().as_ref() {
                        match req.content.as_ref() {
                            Some(content) => cb.is_countable(content.clone()),
                            None => false,
                        }
                    } else {
                        !req.content.as_ref().map(|c| c.unreadable).unwrap_or(false)
                    };

                match merge_conversation_from_chat(
                    self.message_storage.clone(),
                    &req,
                    &req_status,
                    is_countable,
                )
                .await
                {
                    Some(mut conversation) => {
                        if req_status.has_read
                            && req.seq != 0
                            && !req.content.map(|c| c.unreadable).unwrap_or(false)
                        {
                            resps.push(Some(ChatRequest::new_read(
                                &topic_id,
                                conversation.last_seq,
                            )));
                        }
                        if !conversation.is_partial {
                            if let Some(cb) = callback.read().unwrap().as_ref() {
                                conversation.last_seq = req.seq; // don't use conversation.last_seq, it's may be newer
                                cb.on_conversations_updated(vec![conversation]);
                            }
                        } else {
                            self.fetch_conversation(&topic_id, false).await;
                        }
                    }
                    None => {
                        match self.removed_conversations.try_write() {
                            Ok(mut removed_conversations) => {
                                removed_conversations.insert(topic_id.to_string(), now_millis());
                            }
                            Err(_) => {}
                        }
                        self.clear_conversation(&topic_id).await.ok();
                        if let Some(cb) = self.callback.read().unwrap().as_ref() {
                            cb.on_conversation_removed(topic_id);
                        }
                    }
                }
                resps
            }
            ChatRequestType::Read => {
                let resp = ChatRequest::new_response(&req, 200);
                let topic_id = req.topic_id.clone();
                self.set_conversation_read_local(&topic_id, &req.created_at, Some(req.seq))
                    .await;
                self.emit_topic_read(topic_id, req);
                vec![resp]
            }
            _ => {
                warn!("mismatch {:?}", req);
                vec![ChatRequest::new_response(&req, 200)]
            }
        }
    }

    pub async fn handle_send_fail(&self, chat_id: &str) {
        let peek = if let Some(pending) = self.outgoings.read().unwrap().get(chat_id) {
            pending.did_retry();
            pending.is_expired()
        } else {
            false
        };

        if peek {
            if let Some(pending) = self.peek_pending_request(&chat_id).await {
                pending
                    .callback
                    .map(|cb| cb.on_fail("request timeout".to_string()));
            }
        }
    }

    pub async fn peek_pending_request(&self, chat_id: &str) -> Option<PendingRequest> {
        match self.outgoings.try_write() {
            Ok(mut outgoings) => outgoings.remove(chat_id),
            Err(_) => None,
        }
    }

    pub async fn add_pending_request(
        self: ClientStoreRef,
        req: ChatRequest,
        callback: Option<Box<dyn MessageCallback>>,
    ) {
        let chat_id = req.chat_id.clone();
        let pending_request = PendingRequest::new(req, callback);
        match ChatRequestType::from(&pending_request.req.req_type) {
            ChatRequestType::Typing | ChatRequestType::Read => {}
            _ => {
                // save to db
                if let Err(e) = self.save_outgoing_chat_log(&pending_request.req).await {
                    warn!(
                        "save_outgoing_chat_log failed: chat_id:{} err:{}",
                        chat_id, e
                    );
                }
                // process
                let pending_request = match pending_request.has_attachment() {
                    true => match self.submit_upload(pending_request).await {
                        Ok(req) => req,
                        Err(e) => {
                            warn!("submit_upload failed: chat_id:{} err:{}", chat_id, e);
                            return ();
                        }
                    },
                    false => pending_request,
                };
                match self.outgoings.try_write() {
                    Ok(mut outgoings) => {
                        outgoings.insert(chat_id.clone(), pending_request);
                    }
                    Err(_) => {}
                }

                self.try_send(chat_id);
            }
        }
    }

    pub(super) fn try_send(&self, chat_id: String) {
        match self.msg_tx.read().unwrap().as_ref() {
            Some(tx) => match tx.send(chat_id.clone()) {
                Ok(_) => {}
                Err(e) => {
                    warn!("try_send failed: {:?} chat_id:{}", e.to_string(), chat_id);
                }
            },
            None => {
                if let Ok(mut tmps) = self.tmps.try_write() {
                    tmps.push_back(chat_id);
                }
            }
        }
    }

    pub fn flush_offline_requests(&self) {
        let mut tmps = match self.tmps.try_write() {
            Ok(tmps) => tmps,
            Err(_) => return,
        };
        let tx = self.msg_tx.read().unwrap();
        match tx.as_ref() {
            Some(tx) => {
                while let Some(chat_id) = tmps.pop_front() {
                    info!("flush_offline_requests chat_id:{}", chat_id);
                    match tx.send(chat_id.clone()) {
                        Ok(_) => {}
                        Err(e) => {
                            warn!(
                                "flush_offline_requests failed:  {:?} chat_id:{}",
                                e.to_string(),
                                chat_id
                            );
                            break;
                        }
                    }
                }
            }
            None => {}
        }
    }

    pub fn cancel_send(&self, chat_id: &str) {
        match self.outgoings.try_write() {
            Ok(mut outgoings) => {
                if let Some(pending) = outgoings.remove(chat_id) {
                    pending
                        .callback
                        .map(|cb| cb.on_fail("cancel send".to_string()));
                }
            }
            Err(_) => {}
        }

        if let Ok(mut uploadings) = self.upload_tasks.try_write() {
            if let Some(task) = uploadings.remove(chat_id) {
                task.abort();
            }
        }
    }
}
