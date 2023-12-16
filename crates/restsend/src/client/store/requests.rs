use super::attachments::UploadTask;
use super::{ClientStore, PendingRequest};
use crate::callback::Callback;
use crate::client::store::StoreEvent;
use crate::models::ChatLogStatus;
use crate::{
    callback::MessageCallback,
    request::{ChatRequest, ChatRequestType},
};
use http::StatusCode;
use log::{info, warn};
use std::sync::Arc;
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tokio::sync::oneshot;

impl ClientStore {
    pub async fn handle_outgoing(&self, outgoing_tx: UnboundedSender<ChatRequest>) {
        let (msg_tx, mut msg_rx) = unbounded_channel::<String>();
        self.msg_tx.lock().unwrap().replace(msg_tx.clone());

        while let Some(req_id) = msg_rx.recv().await {
            let outgoings = self.outgoings.lock().unwrap();
            if let Some(pending) = outgoings.get(&req_id) {
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
        callback: Arc<Box<dyn Callback>>,
    ) -> Vec<Option<ChatRequest>> {
        info!("process_incoming: {:?}", req);
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

                if let Some(tx) = self.event_tx.lock().unwrap().as_ref() {
                    tx.send(StoreEvent::Ack(status.clone(), req)).ok();
                }

                self.update_outoing_chat_log_state(&topic_id, &chat_id, status, Some(ack_seq))
                    .ok();
                vec![]
            }
            ChatRequestType::Chat => {
                let r = self.save_incoming_chat_log(&req);
                match r {
                    Ok(_) => match self.update_conversation_from_chat(&req) {
                        Ok(conversation) => {
                            if !conversation.is_partial {
                                callback.on_conversations_updated(vec![conversation]);
                            } else {
                                self.fetch_conversation(&topic_id);
                            }
                        }
                        Err(e) => {
                            warn!("update_conversation_from_chat failed: {:?}", e);
                        }
                    },
                    Err(e) => {
                        warn!(
                            "save_incoming_chat_log failed, req_id:{} topic_id:{} err:{}",
                            req.id, req.topic_id, e
                        );
                    }
                }

                if let Err(e) = self
                    .fetch_or_update_user(&req.attendee, req.attendee_profile.clone())
                    .await
                {
                    warn!("fetch_or_update_user failed: {:?}", e);
                }

                let topic_id = req.topic_id.clone();
                let created_at = req.created_at.clone();
                let resp = ChatRequest::new_response(&req, 200);
                if callback.on_new_message(topic_id.clone(), req) {
                    if let Err(e) = self.update_conversation_read(&topic_id, &created_at) {
                        warn!(
                            "update_conversation_read failed, topic_id:{} error: {:?}",
                            topic_id, e
                        );
                    }
                    vec![resp, Some(ChatRequest::new_read(&topic_id))]
                } else {
                    vec![resp]
                }
            }
            ChatRequestType::Read => {
                let resp = ChatRequest::new_response(&req, 200);
                let topic_id = req.topic_id.clone();
                self.set_conversation_read_local(&topic_id);
                callback.on_topic_read(topic_id, req);
                vec![resp]
            }
            _ => {
                warn!("mismatch {:?}", req);
                vec![ChatRequest::new_response(&req, 200)]
            }
        }
    }

    pub async fn handle_send_fail(&self, req_id: &str) {
        if let Some(tx) = self.event_tx.lock().unwrap().as_ref() {
            tx.send(StoreEvent::SendFail(req_id.to_string())).ok();
        }
    }

    pub async fn handle_send_success(&self, req_id: &str) {
        if let Some(tx) = self.event_tx.lock().unwrap().as_ref() {
            tx.send(StoreEvent::SendSuccess(req_id.to_string())).ok();
        }
    }

    pub async fn peek_pending_request(&self, req_id: &str) -> Option<PendingRequest> {
        let mut outgoings = self.outgoings.lock().unwrap();
        outgoings.remove(req_id)
    }

    pub async fn add_pending_request(
        &self,
        req: ChatRequest,
        callback: Option<Box<dyn MessageCallback>>,
    ) {
        let req_id = req.id.clone();
        let pending_request = PendingRequest::new(req, callback);
        match ChatRequestType::from(&pending_request.req.r#type) {
            ChatRequestType::Typing | ChatRequestType::Read => {}
            _ => {
                // save to db
                if let Err(e) = self.save_outgoing_chat_log(&pending_request.req) {
                    warn!("save_outgoing_chat_log failed: req_id:{} err:{}", req_id, e);
                }

                // process media
                if pending_request.has_attachment() {
                    let (cancel_tx, cancel_rx) = oneshot::channel();
                    let upload_result_tx = self.event_tx.lock().unwrap().clone();

                    if upload_result_tx.is_none() {
                        warn!("upload_result_tx is none");
                        pending_request
                            .callback
                            .map(|cb| cb.on_fail(format!("upload_result_tx is none")));
                        return;
                    }

                    let task = Arc::new(UploadTask::new(
                        upload_result_tx.unwrap(),
                        cancel_tx,
                        pending_request,
                    ));

                    self.attachment_inner
                        .submit_upload(&self.endpoint, &self.token, task.clone(), cancel_rx)
                        .await;
                    return;
                }

                self.outgoings
                    .lock()
                    .unwrap()
                    .insert(req_id.clone(), pending_request);
            }
        }
        self.try_send(req_id);
    }

    pub(super) fn try_send(&self, req_id: String) {
        let tx = self.msg_tx.lock().unwrap();
        match tx.as_ref() {
            Some(tx) => {
                tx.send(req_id).unwrap();
            }
            None => {
                let mut tmps = self.tmps.lock().unwrap();
                tmps.push_back(req_id);
            }
        }
    }
    pub fn flush_offline_requests(&self) {
        let mut tmps = self.tmps.lock().unwrap();
        let tx = self.msg_tx.lock().unwrap();
        match tx.as_ref() {
            Some(tx) => {
                while let Some(req_id) = tmps.pop_front() {
                    tx.send(req_id).unwrap();
                }
            }
            None => {}
        }
    }

    pub fn cancel_send(&self, req_id: &str) {
        let mut outgoings = self.outgoings.lock().unwrap();
        if let Some(pending) = outgoings.remove(req_id) {
            self.attachment_inner.cancel_send(&pending.req.id);
            pending
                .callback
                .map(|cb| cb.on_fail("cancel send".to_string()));
        }
    }
}
