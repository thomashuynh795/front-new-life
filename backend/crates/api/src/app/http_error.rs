use crate::app::error::AppError;
use crate::modules::tags::infrastructure::web::dtos::ErrorResponse;
use axum::{
    Json,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};

pub fn map_app_error(error: AppError) -> axum::response::Response {
    let (status, msg) = match error {
        AppError::TagNotFound => (StatusCode::NOT_FOUND, "Tag not found".to_string()),
        AppError::ScanTokenNotFound => (StatusCode::NOT_FOUND, "Scan token not found".to_string()),
        AppError::ProductNotFound => (StatusCode::NOT_FOUND, "Product not found".to_string()),
        AppError::TagAlreadyExists => (StatusCode::CONFLICT, "Tag already exists".to_string()),
        AppError::ReplayDetected => (StatusCode::CONFLICT, "Replay detected".to_string()),
        AppError::TagRevoked => (StatusCode::GONE, "Tag revoked".to_string()),
        AppError::ScanTokenRevoked => (StatusCode::FORBIDDEN, "Scan token revoked".to_string()),
        AppError::ScanTokenExpired => (StatusCode::GONE, "Scan token expired".to_string()),
        AppError::InvalidSignature => (StatusCode::UNAUTHORIZED, "Invalid signature".to_string()),
        AppError::UnsupportedTagMode => {
            (StatusCode::BAD_REQUEST, "Unsupported tag mode".to_string())
        }
        AppError::InvalidKeyVersion => (StatusCode::BAD_REQUEST, "Invalid key version".to_string()),
        AppError::Validation(msg) => (StatusCode::BAD_REQUEST, msg),
        AppError::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized".to_string()),
        AppError::Internal(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Internal server error".to_string(),
        ),
    };

    (status, Json(ErrorResponse { error: msg })).into_response()
}

pub fn require_admin(headers: &HeaderMap, expected_api_key: &str) -> Result<(), AppError> {
    let provided_api_key = headers
        .get("x-admin-key")
        .and_then(|value| value.to_str().ok())
        .ok_or(AppError::Unauthorized)?;

    if provided_api_key == expected_api_key {
        Ok(())
    } else {
        Err(AppError::Unauthorized)
    }
}

pub fn require_bearer_token(headers: &HeaderMap, expected_token: &str) -> Result<(), AppError> {
    let provided_token = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .ok_or(AppError::Unauthorized)?;

    if provided_token == expected_token {
        Ok(())
    } else {
        Err(AppError::Unauthorized)
    }
}
