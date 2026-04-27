use axum::{Router, extract::State, routing::get};
use sea_orm::DatabaseConnection;
use std::sync::Arc;
use tower_http::trace::TraceLayer;

pub struct AppState {
    pub database_connection: Arc<DatabaseConnection>,
}

pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .with_state(state)
        .layer(TraceLayer::new_for_http())
}

pub async fn health_check(State(state): State<Arc<AppState>>) -> &'static str {
    let _database_connection = &state.database_connection;

    "OK"
}
