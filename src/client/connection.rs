use std::cmp::max;

use super::Client;
use crate::client::CtrlMessageType;
use crate::error::ClientError;
use crate::models::{ChatLog, Conversation, Topic};
use crate::request::{ChatRequest, ChatRequestType, PendingRequest};
use crate::utils::random_text;
use crate::Result;

use futures_util::{SinkExt, StreamExt};
use http::request::Builder as WSBuilder;
use log::{debug, info, warn};
use tokio::select;
use tokio::signal;
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration, Instant};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

#[derive(Debug)]
pub(crate) enum WSMessage {
    Message(String),
    Connected,
    Close(String),
}

pub(crate) type WSMessageSender = mpsc::UnboundedSender<WSMessage>;
pub(crate) type WSMessageReceiver = mpsc::UnboundedReceiver<WSMessage>;

impl Client {
    pub(crate) async fn serve_websocket(
        ws_builder: WSBuilder,
        send_rx: WSMessageReceiver,
        recv_tx: WSMessageSender,
    ) -> Result<()> {
        let st = Instant::now();
        let r = select! {
            _ = async {
                sleep(Duration::from_secs(super::CONNECT_TIMEOUT_SECS)).await
            } => {
                warn!(
                    "websocket connect timeout elapsed:{} ms",
                    st.elapsed().as_millis()
                );
                Err(ClientError::WebsocketError("connect timeout".to_string()))
            }
            r = connect_async(
                ws_builder
                    .body(())
                    .map_err(|e| ClientError::HTTPError(e.to_string()))?,
            ) => {
                Ok(r)
            }
        }?;

        match r {
            Err(e) => {
                warn!("websocket connect error:{}", e);
                return Err(ClientError::WebsocketError(e.to_string()));
            }
            _ => {
                warn!(
                    "websocket connected elapsed:{} ms",
                    st.elapsed().as_millis()
                );
            }
        }
        let (ws_stream, _) = r.unwrap();

        let (mut writer, mut reader) = ws_stream.split();
        recv_tx.send(WSMessage::Connected).ok();

        let runner_recv_from_server = async move {
            while let Some(msg) = reader.next().await {
                let m = match msg {
                    Ok(msg) => match msg {
                        Message::Binary(data) => {
                            WSMessage::Message(String::from_utf8(data).unwrap_or_default())
                        }
                        Message::Text(data) => WSMessage::Message(data),
                        Message::Ping(_) => {
                            continue;
                        }
                        Message::Pong(_) => {
                            continue;
                        }
                        Message::Close(_) => WSMessage::Close(String::from("close event")),
                        _ => {
                            warn!("websocket recv unknown message");
                            WSMessage::Close(String::from("unknown message"))
                        }
                    },
                    Err(e) => {
                        warn!("websocket recv error:{}", e);
                        WSMessage::Close(e.to_string())
                    }
                };

                if let WSMessage::Close(_) = m {
                    recv_tx.send(m).ok();
                    break;
                }

                if let Err(e) = recv_tx.send(m) {
                    warn!("websocket forward error:{}", e);
                    break;
                }
            }
        };

        let runner_send_to_server = async move {
            let mut send_rx = send_rx;
            while let Some(msg) = send_rx.recv().await {
                match msg {
                    WSMessage::Message(data) => {
                        if let Err(e) = writer.send(Message::Text(data)).await {
                            warn!("websocket send error:{}", e);
                            break;
                        }
                    }
                    WSMessage::Close(_) => {
                        if let Err(e) = writer.close().await {
                            warn!("websocket close error:{}", e);
                        }
                        break;
                    }
                    _ => {}
                }
            }
        };

        select! {
            _ = runner_recv_from_server => {
                warn!("websocket recv from server exit");
            }
            _ = runner_send_to_server => {
                warn!("websocket send to server exit");
            }
        };
        Ok(())
    }
}

impl Client {
    pub(crate) fn ws_send(&self, msg: String, _retry: usize) -> Result<()> {
        if !msg.contains(r#":"nop""#) {
            info!("ws_send {}", msg)
        }

        if let Some(tx) = self.ws_tx.write().unwrap().as_ref() {
            tx.send(msg)
                .map_err(|e| crate::ClientError::WebsocketError(e.to_string()))
        } else {
            warn!("ignore ws send when disconnected {}", msg);
            Ok(())
        }
    }

    // 在本地数据库里面存储消息，等resp之后修改服务端确认收到的状态
    // 发送消息之前，本地是需要有topic的信息，如果没有，需要先获取topic的信息
    // add lift time to req
    pub(crate) fn send_chat_request(&self, topic_id: &str, req: &ChatRequest) -> Result<()> {
        // 写入到本地的数据库
        // 等发送成功之后，从pending_queue里面去掉，并且修改状态
        let content = req
            .content
            .as_ref()
            .ok_or(ClientError::InvalidContent(String::default()))?;
        self.db
            .add_pending_chat_log(&topic_id, &self.net_store.me()?, &req.chat_id, &content)?;

        self.add_pending(&req, super::REQUEST_RETRY_TIMES);
        self.ws_send(serde_json::to_string(&req)?, super::REQUEST_RETRY_TIMES)
    }

    pub fn add_pending(&self, req: &ChatRequest, retry: usize) {
        self.pending_queue
            .lock()
            .unwrap()
            .push_back(PendingRequest::new(req, retry));
    }

    pub fn ack_pending(&self, resp: &ChatRequest) {
        let mut pending_queue = self.pending_queue.lock().unwrap();
        if let Some(idx) = pending_queue.iter().position(|x| x.req.id == *(resp.id)) {
            if let Some(mut req) = pending_queue.remove(idx) {
                req.req.seq = resp.seq;
                self.on_request_ack(&req.req, resp);
            }
        }
    }

    pub fn on_request_ack(&self, req: &ChatRequest, resp: &ChatRequest) {
        let r#type: ChatRequestType = req.r#type.clone().into();
        match r#type {
            ChatRequestType::Chat => {
                // 发送成功，更新消息状态
                let sent_status = if resp.code == 200 {
                    crate::models::chat_log::ChatLogStatus::Sent
                } else {
                    crate::models::chat_log::ChatLogStatus::Failed
                };

                let r = self.db.update_chat_log_sent(
                    &req.topic_id,
                    &req.chat_id,
                    resp.seq,
                    sent_status,
                );
                if let Err(e) = r {
                    warn!(
                        "update_chat_log_fail failed, topic_id: {} chat_id:{} error: {:?}",
                        req.topic_id, req.chat_id, e,
                    );
                }
            }
            _ => {}
        }

        if resp.code != 200 {
            warn!("on_request_ack failed, req: {:?} resp: {:?}", req, resp);
            if let Some(cb) = self.callback.read().unwrap().as_ref() {
                cb.on_send_message_fail(req.topic_id.clone(), req.chat_id.clone(), resp.code);
            }
            return;
        }

        // 发送成功，更新消息状态
        if let Ok(mut topic) = self.get_topic(req.topic_id.clone()) {
            topic.last_seq = std::cmp::max(topic.last_seq, resp.seq);
            self.db.save_topic(&topic).ok();

            // if !topic.multiple {
            //     // 1.如果是单聊, 需要调用 on_topic_message
            //     if let Ok(chat_log) = self.db.get_chat_log(&resp.topic_id, &resp.chat_id) {
            //         self.on_topic_message(&topic.id, chat_log);
            //     } else {
            //         warn!(
            //             "on_request_ack failed to find resp.chat_id in db, req: {:?} resp: {:?}",
            //             req, resp
            //         );
            //     }
            // }
            self.on_topic_updated_with_request(&topic, &req).ok();
        }
    }
    pub fn on_request_timeout(&self, req: &ChatRequest) {
        let r#type: ChatRequestType = req.r#type.clone().into();
        match r#type {
            ChatRequestType::Chat => {
                // 发送失败，更新消息状态
                let r = self.db.update_chat_log_fail(&req.topic_id, &req.chat_id);
                if let Err(e) = r {
                    warn!(
                        "update_chat_log_fail failed, topic_id: {} chat_id:{} error: {:?}",
                        req.topic_id, req.chat_id, e,
                    );
                }
            }
            _ => {}
        }

        if let Some(cb) = self.callback.read().unwrap().as_ref() {
            cb.on_send_message_fail(
                req.topic_id.clone(),
                req.chat_id.clone(),
                http::StatusCode::REQUEST_TIMEOUT.as_u16() as u32,
            );
        }
    }

    fn handle_timeouts_requests(&self) -> Result<()> {
        if !self.net_store.is_running() {
            return Ok(());
        }

        let mut pending_queue = self.pending_queue.lock().unwrap();
        let max_timeout = Duration::from_secs(super::REQUEST_TIMEOUT_SECS);
        pending_queue.retain(|x| {
            if x.created_at.elapsed() < max_timeout {
                true
            } else {
                warn!("request timeout: {:?}", x.req);
                self.on_request_timeout(&x.req);
                false
            }
        });
        Ok(())
    }

    pub fn app_active(&self) {
        self.ctrl_tx.send(CtrlMessageType::Activate).ok();
    }

    pub fn app_deactivate(&self) {
        self.ctrl_tx.send(CtrlMessageType::Deactivate).ok();
    }

    pub fn get_network_state(&self) -> crate::NetworkState {
        self.net_store.get_state()
    }

    pub fn shutdown(&self) -> Result<()> {
        if self.net_store.is_running() {
            // 如果多次调用shutdown，会导致多次发送shutdown信号
            // 只有程序要退出了才能调用shutdown
            self.net_store.set_running(false);
            self.ctrl_tx.send(CtrlMessageType::Shutdown)?;
        }
        Ok(())
    }

    /*
       run_loop 直到调用shutdown之前，都会重试链接，如果连接断开了，再次重试连接
       client.app_active() 会立即调用重连
       如果调用了shutdown, run_loop 会主动退出
       TODO: 如果出现run_loop退出，需要重新调用run_loop
    */
    pub fn run_loop(&self) -> Result<()> {
        self.net_store.set_running(true);

        let ctrl_tx = self.ctrl_tx.clone();
        self.runtime.spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(1)).await;
                ctrl_tx.send(CtrlMessageType::ProcessTimeoutRequests).ok();
                ctrl_tx.send(CtrlMessageType::Reconnect).ok();
            }
        });

        let ctrl_tx = self.ctrl_tx.clone();
        self.runtime.spawn(async move {
            select! {
                _ = signal::ctrl_c() => {
                    warn!("ctrl-c received, shutting down");
                    ctrl_tx.send(CtrlMessageType::Shutdown).ok();
                }
            };
        });

        let ws_state = crate::net::ConnectionState::new();

        loop {
            if !self.net_store.is_running() {
                break;
            }

            let cmd = self.ctrl_rx.lock().unwrap().blocking_recv();
            if cmd.is_none() {
                break;
            }

            let cmd = cmd.unwrap();
            match cmd {
                CtrlMessageType::Shutdown => {
                    warn!("shutdown run_loop");
                    self.net_store.set_running(false);
                    break;
                }
                CtrlMessageType::ProcessTimeoutRequests => {
                    self.handle_timeouts_requests()?;
                }
                CtrlMessageType::Activate => {
                    ws_state.set_active(true);
                    // 立即重连
                    self.ctrl_tx.send(CtrlMessageType::Connect).ok();
                }
                CtrlMessageType::Deactivate => ws_state.set_active(false),
                CtrlMessageType::Reconnect => {
                    if self.net_store.get_state() != crate::NetworkState::Disconnected {
                        continue;
                    }
                    if ws_state.is_timeout()? {
                        debug!("websocket reconnect timeout");
                        self.ctrl_tx.send(CtrlMessageType::Connect).ok();
                    }
                }
                CtrlMessageType::WebSocketMessage(data) => {
                    tokio::task::block_in_place(|| {
                        self.handle_incoming(data);
                    });
                }
                CtrlMessageType::WebSocketClose(reason) => {
                    self.net_store.set_state(crate::NetworkState::Disconnected);
                    ws_state.did_broken();
                    self.on_disconnect(reason);
                }
                CtrlMessageType::WebsocketError(reason) => {
                    warn!("websocket error: {}", reason);
                    self.net_store.set_state(crate::NetworkState::Disconnected);
                    ws_state.did_broken();
                    self.on_disconnect(reason);
                }
                CtrlMessageType::WebSocketConnected => {
                    self.net_store.set_state(crate::NetworkState::Connected);
                    ws_state.did_connect();
                    self.on_connected();
                }

                CtrlMessageType::Connect => {
                    if self.net_store.get_state() != crate::NetworkState::Disconnected {
                        warn!(
                            "websocket state is not Disconnected, state: {:?}",
                            self.net_store.get_state()
                        );
                        continue;
                    }

                    self.net_store.set_state(crate::NetworkState::Connecting);
                    self.on_connecting();
                    let endpoint = self.net_store.endpoint()?;
                    let endpoint = endpoint.replacen("http", "ws", 1);
                    let connect_url = format!(
                        "{}{}?device={}&nonce={}",
                        endpoint,
                        "/api/connect",
                        crate::DEVICE,
                        random_text(6)
                    );

                    let headers = self.net_store.build_headers_for_websocket(&connect_url)?;

                    let mut ws_builder = WSBuilder::new().method("GET").uri(connect_url.clone());
                    let ws_headers_ref = ws_builder.headers_mut().unwrap();
                    for (k, v) in headers.iter() {
                        ws_headers_ref.insert(k.clone(), v.clone());
                    }

                    let (send_tx, send_rx) = tokio::sync::mpsc::unbounded_channel::<WSMessage>();
                    let (recv_tx, recv_rx) = tokio::sync::mpsc::unbounded_channel::<WSMessage>();

                    let (ws_tx, ws_rx) = mpsc::unbounded_channel::<String>();
                    self.ws_tx.write().unwrap().replace(ws_tx.clone());

                    let ctrl_tx = self.ctrl_tx.clone();

                    self.runtime.spawn(async move {
                        let ctrl_tx = ctrl_tx.clone();
                        let r = Self::serve_websocket(ws_builder, send_rx, recv_tx).await;
                        if let Err(e) = r {
                            warn!("connect websocket failed: {:?}", e);
                            ctrl_tx
                                .send(CtrlMessageType::WebsocketError(e.to_string()))
                                .ok();
                        };
                    });

                    let ctrl_tx = self.ctrl_tx.clone();
                    self.runtime.spawn(async move {
                        let ctrl_tx = ctrl_tx.clone();
                        let mut recv_rx = recv_rx;
                        let recv_from_websocket = async {
                            loop {
                                let msg = recv_rx.recv().await;
                                let r = match msg {
                                    None => {
                                        break;
                                    }
                                    Some(msg) => match msg {
                                        WSMessage::Message(data) => ctrl_tx
                                            .send(CtrlMessageType::WebSocketMessage(data.clone())),
                                        WSMessage::Close(reason) => {
                                            ctrl_tx.send(CtrlMessageType::WebSocketClose(reason))
                                        }
                                        WSMessage::Connected => {
                                            ctrl_tx.send(CtrlMessageType::WebSocketConnected)
                                        }
                                    },
                                };
                                if let Err(e) = r {
                                    warn!(
                                        "recv_from_websocket send message failed: {:?}",
                                        e.to_string()
                                    );
                                    break;
                                }
                            }
                        };

                        let send_to_websocket = async move {
                            let mut ws_rx = ws_rx;
                            loop {
                                let msg = ws_rx.recv().await;
                                if msg.is_none() {
                                    break;
                                }
                                if let Err(e) = send_tx.send(WSMessage::Message(msg.unwrap())) {
                                    warn!("send_to_websocket send failed: {:?}", e);
                                    break;
                                }
                            }
                        };

                        let keepalive_runner = async {
                            let mut st = Instant::now();
                            let ws_tx = ws_tx.clone();
                            loop {
                                tokio::time::sleep(Duration::from_secs(5 as u64)).await;
                                if Instant::now().duration_since(st).as_secs()
                                    < super::KEEPALIVE_INTERVAL
                                {
                                    continue;
                                }
                                st = Instant::now();

                                if let Err(e) = ws_tx.send(String::from("{\"type\":\"nop\"}")) {
                                    warn!("keepalive_runner send failed: {:?}", e);
                                    break;
                                }
                            }
                        };

                        select! {
                            _ = recv_from_websocket => { }
                            _ = send_to_websocket => {}
                            _ = keepalive_runner => {}
                        };
                    });
                }
                CtrlMessageType::MediaUpload(url, file_name, key, private) => self
                    .handle_media_upload(url, file_name, key, private)
                    .unwrap(),
                CtrlMessageType::MediaDownload(url, save_to, key) => {
                    self.handle_media_download(url, save_to, key).unwrap()
                }
                CtrlMessageType::MediaCancelDownload(url, key) => {
                    self.handle_media_cancel_download(url, key).unwrap()
                }
                CtrlMessageType::MediaCancelUpload(file_name, key) => {
                    self.handle_media_cancel_upload(file_name, key).unwrap()
                }

                CtrlMessageType::OnMediaDownloadProgress(url, recived, total, key) => {
                    if let Some(cb) = self.callback.read().unwrap().as_ref() {
                        cb.on_download_progress(url, recived, total, key);
                    }
                }
                CtrlMessageType::OnMediaDownloadCancel(url, file_name, reason, key) => {
                    if let Some(cb) = self.callback.read().unwrap().as_ref() {
                        cb.on_download_cancel(url, file_name, reason, key.clone());
                    }
                    self.pending_medias.lock().unwrap().remove(&key);
                }
                CtrlMessageType::OnMediaDownloadDone(url, file_name, total, key) => {
                    if let Some(cb) = self.callback.read().unwrap().as_ref() {
                        cb.on_download_done(url, file_name, total, key.clone());
                    }
                    self.pending_medias.lock().unwrap().remove(&key);
                }

                CtrlMessageType::OnMediaUploadProgress(url, recived, total, key) => {
                    if let Some(cb) = self.callback.read().unwrap().as_ref() {
                        cb.on_upload_progress(url, recived, total, key);
                    }
                }
                CtrlMessageType::OnMediaUploadCancel(url, file_name, reason, key) => {
                    if let Some(cb) = self.callback.read().unwrap().as_ref() {
                        cb.on_upload_cancel(url, file_name, reason, key.clone());
                    }
                    self.pending_medias.lock().unwrap().remove(&key);
                }
                CtrlMessageType::OnMediaUploadDone(url, file_name, total, key) => {
                    if let Some(cb) = self.callback.read().unwrap().as_ref() {
                        cb.on_upload_done(url, file_name, total, key.clone());
                    }
                    self.pending_medias.lock().unwrap().remove(&key);
                }
            }
        }
        warn!("run_loop exit");
        self.net_store.set_running(false);
        Ok(())
    }

    fn handle_incoming(&self, data: String) {
        let req = serde_json::from_str::<ChatRequest>(&data);
        if let Err(e) = req {
            warn!("parse request error: {} data:{}", e, data);
            return;
        }

        debug!("handle incoming: {}", data);

        let req = req.unwrap();
        let status = self.handle_request(&req);
        if let Err(e) = status {
            warn!("on request error: {}", e);
            return;
        }
        if req.id.is_empty() || req.r#type == "resp" {
            return;
        }

        let resp = req.make_response(status.unwrap());
        let resp = serde_json::to_string(&resp);
        if let Err(e) = resp {
            warn!("make response error: {} data:{}", e, data);
            return;
        }
        let resp = resp.unwrap();
        self.ws_send(resp, 0).unwrap();
    }

    fn handle_request(&self, req: &ChatRequest) -> Result<u32> {
        debug!("on request: {:?}", req);

        let r#type = req.r#type.clone().into();
        if req.attendee_profile.is_some() {
            self.db.update_user(req.attendee_profile.as_ref().unwrap());
        }

        match r#type {
            ChatRequestType::Response => {
                self.ack_pending(&req);
            }
            ChatRequestType::Typing => {
                // 更新 attendee_profile
                if let Some(cb) = self.callback.read().unwrap().as_ref() {
                    cb.on_typing(req.topic_id.clone(), req.attendee.clone());
                }
            }
            ChatRequestType::Read => {
                self.db.update_topic_read(&req.topic_id)?;
                if let Some(cb) = self.callback.read().unwrap().as_ref() {
                    cb.on_read(req.topic_id.clone())
                }
            }
            ChatRequestType::Chat => {
                //1. 收到消息存储到数据库， 如果是recall就需要把消息重置
                //2. 更新本地的会话
                //3. 通知UI
                let content = req.content.clone();
                if content.is_none() {
                    warn!("chat content is none {:?}", req);
                    return Ok(400);
                }

                let content = content.unwrap();
                let mut chat_log: ChatLog = req.into();

                if self.db.get_chat_log(&req.topic_id, &req.chat_id).is_err() {
                    if let Err(e) = self.db.save_chat_log(&chat_log) {
                        warn!("save chat log error: {:?} {}", req, e);
                        return Ok(500);
                    }
                }

                if let Some(cb) = self.callback.read().unwrap().as_ref() {
                    match crate::models::ContentType::from(content.r#type) {
                        crate::models::ContentType::Recall => {
                            chat_log.recall = true;
                            self.db.save_chat_log(&chat_log)?;
                            cb.on_recall(req.topic_id.clone(), req.chat_id.clone());
                        }
                        crate::models::ContentType::TopicKickout => {
                            let user_id = if content.mentions.len() > 0 {
                                content.mentions[0].clone()
                            } else {
                                "".to_string()
                            };

                            cb.on_topic_kickoff(
                                req.topic_id.clone(),
                                req.attendee.clone(),
                                user_id,
                            );
                        }
                        crate::models::ContentType::TopicDismiss => {
                            cb.on_topic_dismissed(req.topic_id.clone(), req.attendee.clone());
                        }
                        crate::models::ContentType::TopicSilent => {
                            cb.on_topic_silent(req.topic_id.clone(), content.duration);
                        }
                        crate::models::ContentType::TopicSilentMember => {
                            cb.on_topic_silent_member(
                                req.topic_id.clone(),
                                req.attendee.clone(),
                                content.duration,
                            );
                        }
                        crate::models::ContentType::TopicQuit => {
                            if let Some(topic_member) = &req.attendee_profile {
                                cb.on_topic_member_updated(
                                    req.topic_id.clone(),
                                    topic_member.clone(),
                                    false,
                                );
                            } else {
                                warn!("TopicQuit attendee_profile is none")
                            }
                        }
                        crate::models::ContentType::TopicJoin => {
                            if let Some(topic_member) = &req.attendee_profile {
                                cb.on_topic_member_updated(
                                    req.topic_id.clone(),
                                    topic_member.clone(),
                                    true,
                                );
                            } else {
                                warn!("TopicQuit attendee_profile is none")
                            }
                        }
                        _ => {
                            cb.on_topic_message(req.topic_id.clone(), chat_log.clone());
                        }
                    }
                    if let Ok(conversation) = self.db.get_conversation(&req.topic_id) {
                        cb.on_conversation_updated(vec![conversation]);
                    } else {
                        warn!("conversation not found: {}", req.topic_id);
                        // update conversation
                    }
                }
            }
            ChatRequestType::Kickout => {
                if let Some(cb) = self.callback.read().unwrap().as_ref() {
                    cb.on_kickoff_by_other_client(req.message.clone().unwrap_or_default());
                }
            }
            ChatRequestType::Nop => {}
            _ => {
                warn!("unknown request type: {:?}", r#type)
            }
        };
        Ok(200)
    }

    fn on_connected(&self) {
        if let Some(cb) = self.callback.read().unwrap().as_ref() {
            cb.on_connected();
        }
    }

    fn on_connecting(&self) {
        if let Some(cb) = self.callback.read().unwrap().as_ref() {
            cb.on_connecting();
        }
        //TODO: 将pending的消息重新发送
    }

    fn on_disconnect(&self, reason: String) {
        if let Some(cb) = self.callback.read().unwrap().as_ref() {
            cb.on_net_broken(reason);
        }
    }

    // 通知 Topic 收到消息，并且更新本地的会话状态
    fn on_topic_updated_with_request(&self, topic: &Topic, req: &ChatRequest) -> Result<()> {
        let mut conversation = self
            .db
            .get_conversation(&topic.id)
            .unwrap_or(Conversation::from(topic));

        conversation.last_seq = max(conversation.last_seq, req.seq);
        if conversation.last_seq == req.seq && req.content.is_some() {
            conversation.last_message = Some(req.content.as_ref().unwrap().clone());
        }

        self.db.save_conversation(&conversation)?;
        if let Some(cb) = self.callback.read().unwrap().as_ref() {
            cb.on_conversation_updated(vec![conversation]);
        }
        Ok(())
    }
}
