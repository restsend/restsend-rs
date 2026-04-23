use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatEvent {
    pub topic_id: String,
    pub sender_id: String,
    pub chat_id: String,
    pub seq: i64,
    pub created_at: String,
    pub content: Option<crate::Content>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationUpdateEvent {
    pub topic_id: String,
    pub owner_id: String,
    pub fields: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationRemovedEvent {
    pub topic_id: String,
    pub owner_id: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TopicSimpleEvent {
    pub topic_id: String,
    pub admin_id: String,
    pub source: String,
    #[serde(skip)]
    pub webhooks: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TopicUserEvent {
    pub topic_id: String,
    pub admin_id: String,
    pub user_id: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TopicNoticeEvent {
    pub topic_id: String,
    pub admin_id: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TopicKnockEvent {
    pub topic_id: String,
    pub admin_id: String,
    pub user_id: String,
    pub message: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TopicSilentEvent {
    pub topic_id: String,
    pub admin_id: String,
    pub user_id: String,
    pub duration: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TopicChangeOwnerEvent {
    pub topic_id: String,
    pub admin_id: String,
    pub user_id: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadEvent {
    pub topic_id: String,
    pub user_id: String,
    pub last_read_seq: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TypingEvent {
    pub topic_id: String,
    pub user_id: String,
    pub attendee: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadFileEvent {
    pub topic_id: String,
    pub user_id: String,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserGuestCreateEvent {
    pub user_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::large_enum_variant)]
pub enum BackendEvent {
    Chat(ChatEvent),
    ConversationUpdate(ConversationUpdateEvent),
    ConversationRemoved(ConversationRemovedEvent),
    TopicCreate(TopicSimpleEvent),
    TopicUpdate(TopicSimpleEvent),
    TopicDismiss(TopicSimpleEvent),
    TopicJoin(TopicUserEvent),
    TopicQuit(TopicUserEvent),
    TopicKickout(TopicUserEvent),
    TopicNotice(TopicNoticeEvent),
    TopicKnock(TopicKnockEvent),
    TopicKnockAccept(TopicKnockEvent),
    TopicKnockReject(TopicKnockEvent),
    TopicSilent(TopicSilentEvent),
    TopicSilentMember(TopicSilentEvent),
    TopicChangeOwner(TopicChangeOwnerEvent),
    Read(ReadEvent),
    Typing(TypingEvent),
    UploadFile(UploadFileEvent),
    UserGuestCreate(UserGuestCreateEvent),
}

impl BackendEvent {
    pub fn event_name(&self) -> &'static str {
        match self {
            BackendEvent::Chat(_) => "chat",
            BackendEvent::ConversationUpdate(_) => "conversation.update",
            BackendEvent::ConversationRemoved(_) => "conversation.removed",
            BackendEvent::TopicCreate(_) => "topic.create",
            BackendEvent::TopicUpdate(_) => "topic.update",
            BackendEvent::TopicDismiss(_) => "topic.dismiss",
            BackendEvent::TopicJoin(_) => "topic.join",
            BackendEvent::TopicQuit(_) => "topic.quit",
            BackendEvent::TopicKickout(_) => "topic.kickout",
            BackendEvent::TopicNotice(_) => "topic.notice",
            BackendEvent::TopicKnock(_) => "topic.knock",
            BackendEvent::TopicKnockAccept(_) => "topic.knock.accept",
            BackendEvent::TopicKnockReject(_) => "topic.knock.reject",
            BackendEvent::TopicSilent(_) => "topic.silent",
            BackendEvent::TopicSilentMember(_) => "topic.silent.member",
            BackendEvent::TopicChangeOwner(_) => "topic.changeowner",
            BackendEvent::Read(_) => "read",
            BackendEvent::Typing(_) => "typing",
            BackendEvent::UploadFile(_) => "upload.file",
            BackendEvent::UserGuestCreate(_) => "user.guest.create",
        }
    }

    pub fn topic_id(&self) -> Option<&str> {
        match self {
            BackendEvent::Chat(v) => Some(&v.topic_id),
            BackendEvent::ConversationUpdate(v) => Some(&v.topic_id),
            BackendEvent::ConversationRemoved(v) => Some(&v.topic_id),
            BackendEvent::TopicCreate(v)
            | BackendEvent::TopicUpdate(v)
            | BackendEvent::TopicDismiss(v) => Some(&v.topic_id),
            BackendEvent::TopicJoin(v)
            | BackendEvent::TopicQuit(v)
            | BackendEvent::TopicKickout(v) => Some(&v.topic_id),
            BackendEvent::TopicNotice(v) => Some(&v.topic_id),
            BackendEvent::TopicKnock(v)
            | BackendEvent::TopicKnockAccept(v)
            | BackendEvent::TopicKnockReject(v) => Some(&v.topic_id),
            BackendEvent::TopicSilent(v) | BackendEvent::TopicSilentMember(v) => Some(&v.topic_id),
            BackendEvent::TopicChangeOwner(v) => Some(&v.topic_id),
            BackendEvent::Read(v) => Some(&v.topic_id),
            BackendEvent::Typing(v) => Some(&v.topic_id),
            BackendEvent::UploadFile(v) => Some(&v.topic_id),
            BackendEvent::UserGuestCreate(_) => None,
        }
    }

    pub fn use_topic_webhooks(&self) -> bool {
        !matches!(
            self,
            BackendEvent::ConversationUpdate(_) | BackendEvent::ConversationRemoved(_)
        )
    }

    pub fn should_send_webhook(&self) -> bool {
        !matches!(self, BackendEvent::Typing(_))
    }

    pub fn explicit_webhooks(&self) -> &[String] {
        match self {
            BackendEvent::TopicCreate(v)
            | BackendEvent::TopicUpdate(v)
            | BackendEvent::TopicDismiss(v) => &v.webhooks,
            _ => &[],
        }
    }

    pub fn data_payload(&self) -> serde_json::Value {
        match self {
            BackendEvent::Chat(v) => {
                serde_json::to_value(v).unwrap_or_else(|_| serde_json::json!({}))
            }
            BackendEvent::ConversationUpdate(v) => {
                serde_json::to_value(v).unwrap_or_else(|_| serde_json::json!({}))
            }
            BackendEvent::ConversationRemoved(v) => {
                serde_json::to_value(v).unwrap_or_else(|_| serde_json::json!({}))
            }
            BackendEvent::TopicCreate(v)
            | BackendEvent::TopicUpdate(v)
            | BackendEvent::TopicDismiss(v) => {
                serde_json::to_value(v).unwrap_or_else(|_| serde_json::json!({}))
            }
            BackendEvent::TopicJoin(v)
            | BackendEvent::TopicQuit(v)
            | BackendEvent::TopicKickout(v) => {
                serde_json::to_value(v).unwrap_or_else(|_| serde_json::json!({}))
            }
            BackendEvent::TopicNotice(v) => {
                serde_json::to_value(v).unwrap_or_else(|_| serde_json::json!({}))
            }
            BackendEvent::TopicKnock(v)
            | BackendEvent::TopicKnockAccept(v)
            | BackendEvent::TopicKnockReject(v) => {
                serde_json::to_value(v).unwrap_or_else(|_| serde_json::json!({}))
            }
            BackendEvent::TopicSilent(v) | BackendEvent::TopicSilentMember(v) => {
                serde_json::to_value(v).unwrap_or_else(|_| serde_json::json!({}))
            }
            BackendEvent::TopicChangeOwner(v) => {
                serde_json::to_value(v).unwrap_or_else(|_| serde_json::json!({}))
            }
            BackendEvent::Read(v) => {
                serde_json::to_value(v).unwrap_or_else(|_| serde_json::json!({}))
            }
            BackendEvent::Typing(v) => {
                serde_json::to_value(v).unwrap_or_else(|_| serde_json::json!({}))
            }
            BackendEvent::UploadFile(v) => {
                serde_json::to_value(v).unwrap_or_else(|_| serde_json::json!({}))
            }
            BackendEvent::UserGuestCreate(v) => {
                serde_json::to_value(v).unwrap_or_else(|_| serde_json::json!({}))
            }
        }
    }
}

#[derive(Clone)]
pub struct EventBus {
    sender: broadcast::Sender<BackendEvent>,
}

impl EventBus {
    pub fn new(buffer_size: usize) -> Self {
        let (sender, _) = broadcast::channel(buffer_size.max(64));
        Self { sender }
    }

    pub fn publish(&self, event: BackendEvent) {
        let event_name = event.event_name();
        if let Err(err) = self.sender.send(event) {
            tracing::warn!(event = event_name, error = %err, "event bus publish failed");
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<BackendEvent> {
        self.sender.subscribe()
    }
}
