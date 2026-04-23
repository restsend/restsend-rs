use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::response::Response;
use chrono::Utc;
use futures_util::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

use crate::api::auth_ctx::AuthCtx;
use crate::api::chat::send_chat_message;
use crate::api::error::ApiError;
use crate::app::AppState;
use crate::infra::event::{BackendEvent, ReadEvent, TypingEvent};
use crate::infra::websocket::SessionSender;

struct PresenceSessionGuard {
    state: AppState,
    user_id: String,
    device: String,
}

#[derive(Clone)]
struct WsSessionState {
    chat_limiter: Option<Arc<tokio::sync::Mutex<tokio::time::Interval>>>,
    last_typing_at: Arc<Mutex<Option<Instant>>>,
}

impl WsSessionState {
    fn new(config: &crate::app::AppConfig) -> Self {
        let chat_limiter = if config.ws_per_user_limit > 0 {
            let interval_ms = (1000_u64 / config.ws_per_user_limit.max(1) as u64).max(1);
            let mut interval = tokio::time::interval(Duration::from_millis(interval_ms));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            Some(Arc::new(tokio::sync::Mutex::new(interval)))
        } else {
            None
        };
        Self {
            chat_limiter,
            last_typing_at: Arc::new(Mutex::new(None)),
        }
    }

    fn typing_allowed(&self, typing_interval_ms: u64) -> bool {
        if typing_interval_ms == 0 {
            return false;
        }
        let now = Instant::now();
        let mut guard = self.last_typing_at.lock().unwrap();
        if let Some(last) = *guard {
            if now.duration_since(last) < Duration::from_millis(typing_interval_ms) {
                return false;
            }
        }
        *guard = Some(now);
        true
    }
}

impl Drop for PresenceSessionGuard {
    fn drop(&mut self) {
        let state = self.state.clone();
        let user_id = self.user_id.clone();
        let device = self.device.clone();
        tokio::spawn(async move {
            state.ws_hub.unregister(&user_id, &device).await;
            state.presence_hub.remove_session(&user_id, &device).await;
            tracing::info!(user_id = %user_id, device = %device, "ws session unregistered");
        });
    }
}

#[derive(Debug, Deserialize)]
pub struct WsConnectQuery {
    pub user_id: Option<String>,
    pub device: Option<String>,
    pub nonce: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WsEnvelope {
    #[serde(default)]
    r#type: String,
    #[serde(default)]
    topic_id: String,
    #[serde(default)]
    chat_id: String,
    #[serde(default)]
    message: String,
    #[serde(default)]
    attendee: String,
    #[serde(default)]
    source: String,
    #[serde(default)]
    seq: i64,
    #[serde(default)]
    content: Option<crate::Content>,
}

pub async fn ws_upgrade(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Query(query): Query<WsConnectQuery>,
) -> Response {
    ws.on_upgrade(move |socket| ws_session_loop(state, query, socket))
}

pub async fn ws_connect(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    auth: AuthCtx,
    Query(query): Query<WsConnectQuery>,
) -> Response {
    let query = WsConnectQuery {
        user_id: query.user_id.clone().or(Some(auth.user_id.clone())),
        device: query.device,
        nonce: query.nonce,
    };
    ws.on_upgrade(move |socket| ws_session_loop(state, query, socket))
}

async fn ws_session_loop(state: AppState, query: WsConnectQuery, socket: WebSocket) {
    let base_device = query.device.unwrap_or_else(|| "web".to_string());
    let device = if let Some(nonce) = query.nonce.filter(|v| !v.is_empty()) {
        format!("{}:{}", base_device, nonce)
    } else {
        base_device
    };
    let Some(user_id) = query.user_id else {
        return;
    };
    let session_state = WsSessionState::new(&state.config);
    let (sender_handle, mut rx) = if state.config.ws_client_queue_size > 0 {
        let (tx, rx) = mpsc::channel::<String>(state.config.ws_client_queue_size);
        (SessionSender::Bounded(tx), WsReceiver::Bounded(rx))
    } else {
        let (tx, rx) = mpsc::unbounded_channel::<String>();
        (SessionSender::Unbounded(tx), WsReceiver::Unbounded(rx))
    };

    state
        .ws_hub
        .register(&user_id, &device, sender_handle)
        .await;
    state.presence_hub.upsert_session(&user_id, &device).await;
    tracing::info!(user_id = %user_id, device = %device, "ws session registered");

    let _guard = PresenceSessionGuard {
        state: state.clone(),
        user_id: user_id.clone(),
        device: device.clone(),
    };

    let (mut sender, mut receiver) = socket.split();
    let mut heartbeat = tokio::time::interval(std::time::Duration::from_secs(
        state.config.presence_heartbeat_secs.max(1),
    ));

    loop {
        tokio::select! {
            _ = heartbeat.tick() => {
                state.presence_hub.upsert_session(&user_id, &device).await;
            }
            outbound = rx.recv() => {
                match outbound {
                    Some(msg) => {
                        state.metrics.incr_outbound_ws_messages();
                        if sender.send(Message::Text(msg)).await.is_err() {
                            break;
                        }
                    }
                    None => break,
                }
            }
            inbound = receiver.next() => {
                match inbound {
                    Some(Ok(Message::Text(payload))) => {
                        state.metrics.incr_inbound_ws_messages();
                        state.presence_hub.upsert_session(&user_id, &device).await;
                        if payload == "ping" {
                            state
                                .ws_hub
                                .send_to_device(&user_id, &device, "pong", state.config.ws_drop_on_backpressure)
                                .await;
                            continue;
                        }
                        if let Ok(req) = serde_json::from_str::<WsEnvelope>(&payload) {
                            let state = state.clone();
                            let message_pool = state.message_pool.clone();
                            let user_id = user_id.clone();
                            let device = device.clone();
                            let session_state = session_state.clone();
                            let req_type = req.r#type.clone();
                            let req_topic_id = req.topic_id.clone();
                            let req_chat_id = req.chat_id.clone();
                            let _ = message_pool.submit(async move {
                                let st = std::time::Instant::now();
                                handle_ws_envelope(&state, &user_id, &device, &session_state, req).await;
                                tracing::info!(
                                    user_id = %user_id,
                                    req_type = %req_type,
                                    topic_id = %req_topic_id,
                                    chat_id = %req_chat_id,
                                    elapsed_ms = st.elapsed().as_millis() as u64,
                                    "ws message processed"
                                );
                            }).await;
                        }
                    }
                    Some(Ok(Message::Close(_))) | None | Some(Err(_)) => break,
                    _ => {}
                }
            }
        }
    }
}

enum WsReceiver {
    Unbounded(mpsc::UnboundedReceiver<String>),
    Bounded(mpsc::Receiver<String>),
}

impl WsReceiver {
    async fn recv(&mut self) -> Option<String> {
        match self {
            WsReceiver::Unbounded(rx) => rx.recv().await,
            WsReceiver::Bounded(rx) => rx.recv().await,
        }
    }
}

async fn handle_ws_envelope(
    state: &AppState,
    user_id: &str,
    device: &str,
    session_state: &WsSessionState,
    req: WsEnvelope,
) {
    match req.r#type.as_str() {
        "ping" => {
            let payload = serde_json::to_string(&serde_json::json!({
                "type": "resp",
                "chatId": req.chat_id,
                "seq": req.seq,
                "code": 200,
                "message": req.message,
                "content": req.content,
                "createdAt": Utc::now().to_rfc3339(),
            }))
            .unwrap_or_default();
            state
                .ws_hub
                .send_to_device(
                    user_id,
                    device,
                    &payload,
                    state.config.ws_drop_on_backpressure,
                )
                .await;
        }
        "typing" => {
            if req.topic_id.is_empty() {
                let payload = serde_json::to_string(&serde_json::json!({
                    "type": "resp",
                    "chatId": req.chat_id,
                    "topicId": req.topic_id,
                    "code": 403,
                    "createdAt": Utc::now().to_rfc3339(),
                }))
                .unwrap_or_default();
                state
                    .ws_hub
                    .send_to_device(
                        user_id,
                        device,
                        &payload,
                        state.config.ws_drop_on_backpressure,
                    )
                    .await;
                return;
            }
            if !session_state.typing_allowed(state.config.ws_typing_interval_ms) {
                let payload = serde_json::to_string(&serde_json::json!({
                    "type": "resp",
                    "chatId": req.chat_id,
                    "topicId": req.topic_id,
                    "code": 200,
                    "createdAt": Utc::now().to_rfc3339(),
                }))
                .unwrap_or_default();
                state
                    .ws_hub
                    .send_to_device(
                        user_id,
                        device,
                        &payload,
                        state.config.ws_drop_on_backpressure,
                    )
                    .await;
                return;
            }
            state.event_bus.publish(BackendEvent::Typing(TypingEvent {
                topic_id: req.topic_id.clone(),
                user_id: user_id.to_string(),
                attendee: req.attendee.clone(),
            }));
            let payload = serde_json::to_string(&serde_json::json!({
                "type": "typing",
                "topicId": req.topic_id.clone(),
                "attendee": user_id,
            }))
            .unwrap_or_default();
            if !req.attendee.is_empty() {
                crate::api::push::broadcast_to_user(state, &req.attendee, &payload).await;
            } else if let Ok(members) = state.topic_service.list_members(&req.topic_id).await {
                for member in members {
                    if member != user_id {
                        crate::api::push::broadcast_to_user(state, &member, &payload).await;
                    }
                }
            }
            let ack_payload = serde_json::to_string(&serde_json::json!({
                "type": "resp",
                "chatId": req.chat_id,
                "topicId": req.topic_id,
                "code": 200,
                "createdAt": Utc::now().to_rfc3339(),
            }))
            .unwrap_or_default();
            state
                .ws_hub
                .send_to_device(
                    user_id,
                    device,
                    &ack_payload,
                    state.config.ws_drop_on_backpressure,
                )
                .await;
        }
        "read" => {
            if req.topic_id.is_empty() {
                let payload = serde_json::to_string(&serde_json::json!({
                    "type": "resp",
                    "chatId": req.chat_id,
                    "code": 404,
                    "createdAt": Utc::now().to_rfc3339(),
                }))
                .unwrap_or_default();
                state
                    .ws_hub
                    .send_to_device(
                        user_id,
                        device,
                        &payload,
                        state.config.ws_drop_on_backpressure,
                    )
                    .await;
                return;
            }
            let topic = state.topic_service.get_by_id(&req.topic_id).await;
            let Ok(topic) = topic else {
                let payload = serde_json::to_string(&serde_json::json!({
                    "type": "resp",
                    "chatId": req.chat_id,
                    "topicId": req.topic_id,
                    "code": 404,
                    "createdAt": Utc::now().to_rfc3339(),
                }))
                .unwrap_or_default();
                state
                    .ws_hub
                    .send_to_device(
                        user_id,
                        device,
                        &payload,
                        state.config.ws_drop_on_backpressure,
                    )
                    .await;
                return;
            };
            let members = state
                .topic_service
                .list_members(&req.topic_id)
                .await
                .unwrap_or_default();
            if !members.iter().any(|member| member == user_id) && topic.owner_id != user_id {
                let payload = serde_json::to_string(&serde_json::json!({
                    "type": "resp",
                    "chatId": req.chat_id,
                    "topicId": req.topic_id,
                    "code": 403,
                    "createdAt": Utc::now().to_rfc3339(),
                }))
                .unwrap_or_default();
                state
                    .ws_hub
                    .send_to_device(
                        user_id,
                        device,
                        &payload,
                        state.config.ws_drop_on_backpressure,
                    )
                    .await;
                return;
            }
            let conversation = state
                .conversation_service
                .mark_read(
                    user_id,
                    &req.topic_id,
                    if req.seq > 0 { Some(req.seq) } else { None },
                )
                .await;
            let Ok(conversation) = conversation else {
                let payload = serde_json::to_string(&serde_json::json!({
                    "type": "resp",
                    "chatId": req.chat_id,
                    "topicId": req.topic_id,
                    "code": 404,
                    "createdAt": Utc::now().to_rfc3339(),
                }))
                .unwrap_or_default();
                state
                    .ws_hub
                    .send_to_device(
                        user_id,
                        device,
                        &payload,
                        state.config.ws_drop_on_backpressure,
                    )
                    .await;
                return;
            };

            state.event_bus.publish(BackendEvent::Read(ReadEvent {
                topic_id: req.topic_id.clone(),
                user_id: user_id.to_string(),
                last_read_seq: conversation.last_read_seq,
            }));

            let read_payload = serde_json::to_string(&serde_json::json!({
                "type": "read",
                "topicId": req.topic_id,
                "seq": conversation.last_read_seq,
                "attendee": user_id,
                "createdAt": Utc::now().to_rfc3339(),
            }))
            .unwrap_or_default();
            for member in members {
                if member != user_id {
                    crate::api::push::broadcast_to_user(state, &member, &read_payload).await;
                }
            }

            let ack_payload = serde_json::to_string(&serde_json::json!({
                "type": "resp",
                "topicId": conversation.topic_id,
                "seq": conversation.last_read_seq,
                "chatId": req.chat_id,
                "code": 200,
                "attendee": user_id,
                "createdAt": Utc::now().to_rfc3339(),
            }))
            .unwrap_or_default();
            state
                .ws_hub
                .send_to_device(
                    user_id,
                    device,
                    &ack_payload,
                    state.config.ws_drop_on_backpressure,
                )
                .await;
        }
        "chat" => {
            if let Some(limiter) = &session_state.chat_limiter {
                let mut guard = limiter.lock().await;
                let immediate = tokio::time::timeout(Duration::from_millis(1), guard.tick()).await;
                if immediate.is_err() {
                    let payload = serde_json::to_string(&serde_json::json!({
                        "type": "resp",
                        "chatId": req.chat_id,
                        "topicId": req.topic_id,
                        "code": 429,
                        "createdAt": Utc::now().to_rfc3339(),
                    }))
                    .unwrap_or_default();
                    state
                        .ws_hub
                        .send_to_device(
                            user_id,
                            device,
                            &payload,
                            state.config.ws_drop_on_backpressure,
                        )
                        .await;
                    return;
                }
            }
            let req_topic_id = req.topic_id.clone();
            let req_chat_id = req.chat_id.clone();
            let message = crate::OpenApiChatMessageForm {
                r#type: "chat".to_string(),
                topic_id: req_topic_id.clone(),
                attendee: req.attendee.clone(),
                chat_id: req_chat_id.clone(),
                message: req.message,
                source: req.source,
                content: req.content,
                ..crate::OpenApiChatMessageForm::default()
            };

            match send_chat_message(state, user_id, message).await {
                Ok((effective_form, topic_id, resp)) => {
                    let created_at = effective_form.created_at.clone().unwrap_or_default();
                    let event_payload = serde_json::to_string(&serde_json::json!({
                        "type": "chat",
                        "topicId": topic_id,
                        "seq": resp.seq,
                        "chatId": resp.chat_id,
                        "attendee": user_id,
                        "createdAt": created_at,
                        "content": effective_form.content.clone().or_else(|| {
                            if effective_form.message.is_empty() {
                                None
                            } else {
                                Some(crate::Content {
                                    content_type: if effective_form.r#type.is_empty() { "chat".to_string() } else { effective_form.r#type.clone() },
                                    text: effective_form.message.clone(),
                                    ..crate::Content::default()
                                })
                            }
                        })
                    }))
                    .unwrap_or_default();
                    crate::api::push::broadcast_to_user(state, user_id, &event_payload).await;
                    if let Ok(members) = state.topic_service.list_members(&resp.topic_id).await {
                        for member in members {
                            if member != user_id {
                                crate::api::push::broadcast_to_user(state, &member, &event_payload)
                                    .await;
                            }
                        }
                    }

                    let ack_payload = serde_json::to_string(&serde_json::json!({
                        "type": "resp",
                        "topicId": resp.topic_id,
                        "seq": resp.seq,
                        "chatId": resp.chat_id,
                        "code": resp.code,
                        "attendee": effective_form.attendee,
                        "createdAt": effective_form.created_at.clone().unwrap_or_default(),
                    }))
                    .unwrap_or_default();
                    state
                        .ws_hub
                        .send_to_device(
                            user_id,
                            device,
                            &ack_payload,
                            state.config.ws_drop_on_backpressure,
                        )
                        .await;
                }
                Err(err) => {
                    let code = map_ws_error_code(&err).unwrap_or(500);
                    let payload = serde_json::to_string(&serde_json::json!({
                        "type": "resp",
                        "chatId": req_chat_id,
                        "topicId": req_topic_id,
                        "code": code,
                        "createdAt": Utc::now().to_rfc3339(),
                    }))
                    .unwrap_or_default();
                    state
                        .ws_hub
                        .send_to_device(
                            user_id,
                            device,
                            &payload,
                            state.config.ws_drop_on_backpressure,
                        )
                        .await;
                }
            }
        }
        _ => {}
    }
}

fn map_ws_error_code(err: &ApiError) -> Option<u16> {
    match err {
        ApiError::NotFound => Some(404),
        ApiError::Unauthorized => Some(403),
        ApiError::BadRequest(_) => Some(400),
        ApiError::InvalidToken => Some(401),
        ApiError::Internal(_) => Some(500),
    }
}
