use sea_orm::DbErr;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("not found")]
    NotFound,
    #[error("conflict")]
    Conflict,
    #[error("forbidden")]
    Forbidden,
    #[error("validation failed: {0}")]
    Validation(String),
    #[error("storage error: {0}")]
    Storage(String),
}

impl From<DbErr> for DomainError {
    fn from(value: DbErr) -> Self {
        Self::Storage(value.to_string())
    }
}

pub type DomainResult<T> = Result<T, DomainError>;
