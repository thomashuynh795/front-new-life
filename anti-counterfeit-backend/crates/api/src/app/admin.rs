use crate::app::error::AppError;
use crate::app::http::AppState;
use crate::app::http_error::{map_app_error, require_bearer_token};
use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use sea_orm::{ConnectionTrait, Statement};
use serde_json::json;
use std::sync::Arc;

pub async fn wipe_database(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let Some(expected_token) = state.db_wipe_token.as_deref() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "error": "Database wipe route is disabled. Set DB_WIPE_TOKEN to enable it."
            })),
        )
            .into_response();
    };

    if let Err(error) = require_bearer_token(&headers, expected_token) {
        return map_app_error(error);
    }

    let Some(database_connection) = state.database_connection.as_ref() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "error": "Database wipe route is unavailable because no database connection is configured."
            })),
        )
            .into_response();
    };

    let statement = Statement::from_string(
        database_connection.get_database_backend(),
        "TRUNCATE TABLE audit_events, scan_events, scan_tokens, token_batches, items, tags RESTART IDENTITY CASCADE;",
    );

    match database_connection.execute(statement).await {
        Ok(_) => (
            StatusCode::OK,
            Json(json!({
                "message": "All database tables have been wiped."
            })),
        )
            .into_response(),
        Err(error) => map_app_error(AppError::Internal(format!(
            "failed to wipe database: {error}"
        ))),
    }
}
