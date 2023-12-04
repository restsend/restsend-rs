use std::sync::Arc;

use super::attachments::UploadTask;
use super::{ClientStore, PendingRequest};
use crate::models::{ChatLog, ChatLogStatus};
use crate::{
    callback::MessageCallback,
    request::{ChatRequest, ChatRequestType},
};
use anyhow::Result;
use log::warn;
use tokio::sync::oneshot;
use tokio::{
    select,
    sync::mpsc::{unbounded_channel, UnboundedSender},
};

impl ClientStore {
    pub async fn handle_outgoing(&self, outgoing_tx: UnboundedSender<ChatRequest>) {
        let (msg_tx, mut msg_rx) = unbounded_channel::<String>();
        self.msg_tx.lock().unwrap().replace(msg_tx.clone());

        while let Some(req_id) = msg_rx.recv().await {
            let mut outgoings = self.outgoings.lock().unwrap();
            if let Some(pending) = outgoings.remove(&req_id) {
                if pending.is_expired() {
                    continue;
                }
                outgoing_tx.send(pending.req).ok();
                pending.callback.map(|cb| cb.on_sent());
            }
        }
    }

    pub async fn process_incoming(&self, req: ChatRequest) -> Option<ChatRequest> {
        warn!("process_incoming: {:?}", req);

        None
    }

    pub async fn handle_send_fail(&self, req_id: &str) {}
    pub async fn handle_send_success(&self, req_id: &str) {}

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
                if let Err(e) = self.save_outgoing_chat_log(&pending_request.req).await {
                    warn!("save_outgoing_chat_log failed: req_id:{} err:{}", req_id, e);
                }

                // process media
                if pending_request.has_attachment() {
                    let (cancel_tx, cancel_rx) = oneshot::channel();
                    let upload_result_tx = self.upload_result_tx.lock().unwrap().clone();

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

    pub async fn flush_offline_requests(&self) {
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

    async fn save_outgoing_chat_log(&self, req: &ChatRequest) -> Result<()> {
        let t = self
            .message_storage
            .table::<ChatLog>("chat_logs")
            .ok_or(anyhow::anyhow!("save_outgoing_chat_log: get table failed"))?;

        let mut log = ChatLog::from(req);
        log.status = crate::models::ChatLogStatus::Sending;
        t.set(&log.topic_id, &log.id, Some(log.clone()));

        Ok(())
    }

    pub(super) async fn update_outoing_chat_log_state(
        &self,
        topic_id: &str,
        log_id: &str,
        status: ChatLogStatus,
    ) -> Result<()> {
        let t = self
            .message_storage
            .table::<ChatLog>("chat_logs")
            .ok_or(anyhow::anyhow!("save_outgoing_chat_log: get table failed"))?;

        if let Some(log) = t.get(topic_id, log_id) {
            let mut log = log.clone();
            log.status = status;
            t.set(topic_id, log_id, Some(log));
        }
        Ok(())
    }

    pub async fn pause_send(&self, req_id: &str) {
        self.attachment_inner.cancel_send(&req_id).await
    }

    pub async fn cancel_send(&self, req_id: &str) {
        let mut outgoings = self.outgoings.lock().unwrap();
        if let Some(pending) = outgoings.remove(req_id) {
            self.attachment_inner.cancel_send(&pending.req.id).await;
            pending
                .callback
                .map(|cb| cb.on_fail("cancel send".to_string()));
        }
    }
}
