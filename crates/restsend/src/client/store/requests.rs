use super::{CallbackRef, ClientStore, ClientStoreRef, PendingRequest};
use crate::client::store::conversations::merge_conversation_from_chat;
use crate::models::ChatLogStatus;
use crate::{
    callback::MessageCallback,
    request::{ChatRequest, ChatRequestType},
};
use http::StatusCode;
use log::{info, warn};
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};

impl ClientStore {
    pub async fn handle_outgoing(&self, outgoing_tx: UnboundedSender<ChatRequest>) {
        let (msg_tx, mut msg_rx) = unbounded_channel::<String>();
        self.msg_tx.lock().unwrap().replace(msg_tx.clone());

        while let Some(chat_id) = msg_rx.recv().await {
            let outgoings = self.outgoings.lock().unwrap();
            if let Some(pending) = outgoings.get(&chat_id) {
                if pending.is_expired() {
                    continue;
                }
                outgoing_tx.send(pending.req.clone()).ok();
                pending.callback.as_ref().map(|cb| cb.on_sent());
            }
        }
    }

    pub async fn process_incoming(
        &self,
        req: ChatRequest,
        callback: CallbackRef,
    ) -> Vec<Option<ChatRequest>> {
        let content_type = match req.content.as_ref() {
            Some(content) => content.r#type.clone(),
            None => req.r#type.clone(),
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

        match ChatRequestType::from(&req.r#type) {
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

                let topic_id = req.topic_id.clone();
                let created_at = req.created_at.clone();
                let resp = ChatRequest::new_response(&req, 200);

                let r = callback
                    .lock()
                    .unwrap()
                    .as_ref()
                    .map(|cb| cb.on_new_message(topic_id.clone(), req.clone()));

                let resps = match r {
                    Some(true) => {
                        let last_read_seq = Some(req.seq);
                        if let Err(e) = self
                            .update_conversation_read(&topic_id, &created_at, last_read_seq)
                            .await
                        {
                            warn!(
                                "update_conversation_read failed, topic_id:{} error: {:?}",
                                topic_id, e
                            );
                        }
                        vec![resp, Some(ChatRequest::new_read(&topic_id))]
                    }
                    _ => vec![resp],
                };

                let r = self.save_incoming_chat_log(&req).await;
                match r {
                    Ok(_) => match merge_conversation_from_chat(self.message_storage.clone(), &req)
                        .await
                    {
                        Ok(conversation) => {
                            if !conversation.is_partial {
                                if let Some(cb) = callback.lock().unwrap().as_ref() {
                                    cb.on_conversations_updated(vec![conversation]);
                                }
                            } else {
                                self.fetch_conversation(&topic_id, false).await;
                            }
                        }
                        Err(e) => {
                            warn!("update_conversation_from_chat failed: {:?}", e);
                        }
                    },
                    Err(e) => {
                        warn!(
                            "save_incoming_chat_log failed, chat_id:{} topic_id:{} err:{}",
                            req.chat_id, req.topic_id, e
                        );
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
        let peek = if let Some(pending) = self.outgoings.lock().unwrap().get(chat_id) {
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
        let mut outgoings = self.outgoings.lock().unwrap();
        outgoings.remove(chat_id)
    }

    pub async fn add_pending_request(
        self: ClientStoreRef,
        req: ChatRequest,
        callback: Option<Box<dyn MessageCallback>>,
    ) {
        let chat_id = req.chat_id.clone();
        let pending_request = PendingRequest::new(req, callback);
        match ChatRequestType::from(&pending_request.req.r#type) {
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
                self.outgoings
                    .lock()
                    .unwrap()
                    .insert(chat_id.clone(), pending_request);

                self.try_send(chat_id);
            }
        }
    }

    pub(super) fn try_send(&self, chat_id: String) {
        match self.msg_tx.lock().unwrap().as_ref() {
            Some(tx) => match tx.send(chat_id.clone()) {
                Ok(_) => {}
                Err(e) => {
                    warn!("try_send failed: {:?} chat_id:{}", e.to_string(), chat_id);
                }
            },
            None => {
                let mut tmps = self.tmps.lock().unwrap();
                tmps.push_back(chat_id);
            }
        }
    }

    pub fn flush_offline_requests(&self) {
        let mut tmps = self.tmps.lock().unwrap();
        let tx = self.msg_tx.lock().unwrap();
        match tx.as_ref() {
            Some(tx) => {
                while let Some(chat_id) = tmps.pop_front() {
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
        let mut outgoings = self.outgoings.lock().unwrap();
        if let Some(pending) = outgoings.remove(chat_id) {
            pending
                .callback
                .map(|cb| cb.on_fail("cancel send".to_string()));
        }

        let mut uploadings = self.upload_tasks.lock().unwrap();
        if let Some(task) = uploadings.remove(chat_id) {
            task.abort();
        }
    }
}
