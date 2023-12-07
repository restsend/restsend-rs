#[derive(Debug, thiserror::Error, uniffi::Error)]
#[uniffi(flat_error)]
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
    #[error("json: {0}")]
    JSON(#[from] serde_json::Error),
    #[error("cancel: {0}")]
    UserCancel(String),
    #[error("storage: {0}")]
    Storage(String),
    #[error("{0}")]
    Other(String),
}

impl From<reqwest::Error> for ClientError {
    fn from(e: reqwest::Error) -> ClientError {
        ClientError::HTTP(e.to_string())
    }
}

impl From<std::num::ParseIntError> for ClientError {
    fn from(e: std::num::ParseIntError) -> ClientError {
        ClientError::StdError(e.to_string())
    }
}

impl<T> From<tokio::sync::mpsc::error::SendError<T>> for ClientError {
    fn from(e: tokio::sync::mpsc::error::SendError<T>) -> ClientError {
        ClientError::StdError(e.to_string())
    }
}

impl From<std::io::Error> for ClientError {
    fn from(e: std::io::Error) -> ClientError {
        ClientError::StdError(format!("io error {}", e.to_string()))
    }
}
