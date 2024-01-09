#[derive(Debug, Clone, thiserror::Error)]
pub enum ClientError {
    #[error("auth: invalid password")]
    InvalidPassword(String),
    #[error("auth: forbidden")]
    Forbidden(String),
    #[error("auth: invalid token")]
    TokenExpired(String),
    #[error("websocket: network broken")]
    NetworkBroken(String),
    #[error("topic: topic is not found {0}")]
    TopicNotFound(String),
    #[error("topic_knock: topic_knock is not found {0}")]
    TopicKnockNotFound(String),
    #[error("chat_log: chat_log is not found {0}")]
    ChatLogNotFound(String),
    #[error("content: invalid content {0}")]
    InvalidContent(String),
    #[error("conversation: conversation is not found {0}")]
    ConversationNotFound(String),
    #[error("user: user is not found {0}")]
    UserNotFound(String),
    #[error("auth: kickoff by other client")]
    KickOffByOtherClient(String),
    #[error("std: {0}")]
    StdError(String),
    #[error("websocket: {0}")]
    WebsocketError(String),
    #[error("http: {0}")]
    HTTP(String),
    #[error("cancel: {0}")]
    UserCancel(String),
    #[error("storage: {0}")]
    Storage(String),
    #[error("{0}")]
    Other(String),
}
