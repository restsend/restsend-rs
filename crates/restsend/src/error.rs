#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("auth: invalid password")]
    InvalidPassword(String),
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
    HTTPError(String),
    #[error("json: {0}")]
    JSONError(#[from] serde_json::Error),
    #[error("send ctl message: {0}")]
    SendCtrlMessageError(String),
    #[error("unknown: {0}")]
    UnknownError(String),
}

impl From<reqwest::Error> for ClientError {
    fn from(e: reqwest::Error) -> ClientError {
        ClientError::HTTPError(e.to_string())
    }
}

impl From<url::ParseError> for ClientError {
    fn from(e: url::ParseError) -> ClientError {
        ClientError::HTTPError(e.to_string())
    }
}

impl<T> From<std::sync::PoisonError<T>> for ClientError {
    fn from(e: std::sync::PoisonError<T>) -> ClientError {
        ClientError::StdError(e.to_string())
    }
}

impl From<std::time::SystemTimeError> for ClientError {
    fn from(e: std::time::SystemTimeError) -> ClientError {
        ClientError::StdError(e.to_string())
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

impl From<chrono::ParseError> for ClientError {
    fn from(e: chrono::ParseError) -> ClientError {
        ClientError::StdError(e.to_string())
    }
}

impl From<std::io::Error> for ClientError {
    fn from(e: std::io::Error) -> ClientError {
        ClientError::StdError(format!("io error {}", e.to_string()))
    }
}
