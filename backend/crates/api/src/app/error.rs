use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum AppError {
    #[error("Tag not found")]
    TagNotFound,
    #[error("Scan token not found")]
    ScanTokenNotFound,
    #[error("Product not found")]
    ProductNotFound,
    #[error("Tag already exists")]
    TagAlreadyExists,
    #[error("Unsupported tag mode")]
    UnsupportedTagMode,
    #[error("Invalid key version")]
    InvalidKeyVersion,
    #[error("Replay detected")]
    ReplayDetected,
    #[error("Tag revoked")]
    TagRevoked,
    #[error("Scan token revoked")]
    ScanTokenRevoked,
    #[error("Scan token expired")]
    ScanTokenExpired,
    #[error("Invalid signature")]
    InvalidSignature,
    #[error("Validation error: {0}")]
    Validation(String),
    #[error("Unauthorized")]
    Unauthorized,
    #[error("Internal error: {0}")]
    Internal(String),
}
