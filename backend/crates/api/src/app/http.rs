use crate::app::admin as admin_handlers;
use crate::modules::scan_tokens::application::scan_tokens::{
    ConsumeScanTokenUseCase, GenerateScanTokensUseCase,
};
use crate::modules::scan_tokens::infrastructure::web::handlers as scan_token_handlers;
use crate::modules::tags::application::admin::{
    ListCatalogItemsUseCase, ListCatalogTagsUseCase, NextMessagesUseCase, ReconfigureTagUseCase,
    RevokeScanTokenUseCase, RevokeTagUseCase, RotateKeyUseCase,
};
use crate::modules::tags::application::provision::EnrollTagUseCase;
use crate::modules::tags::application::verify::VerifyTagUseCase;
use crate::modules::tags::infrastructure::web::handlers as tag_handlers;
use axum::{
    Router,
    routing::{get, post},
};
use sea_orm::DatabaseConnection;
use std::sync::Arc;
use tower_http::trace::TraceLayer;

pub struct AppState {
    pub api_base_url: String,
    pub admin_key: String,
    pub db_wipe_token: Option<String>,
    pub database_connection: Option<Arc<DatabaseConnection>>,
    pub enroll_usecase: Arc<EnrollTagUseCase>,
    pub verify_usecase: Arc<VerifyTagUseCase>,
    pub revoke_usecase: Arc<RevokeTagUseCase>,
    pub rotate_usecase: Arc<RotateKeyUseCase>,
    pub reconfigure_usecase: Arc<ReconfigureTagUseCase>,
    pub next_messages_usecase: Arc<NextMessagesUseCase>,
    pub list_catalog_items_usecase: Arc<ListCatalogItemsUseCase>,
    pub list_catalog_tags_usecase: Arc<ListCatalogTagsUseCase>,
    pub revoke_scan_token_usecase: Arc<RevokeScanTokenUseCase>,
    pub generate_scan_tokens_usecase: Arc<GenerateScanTokensUseCase>,
    pub consume_scan_token_usecase: Arc<ConsumeScanTokenUseCase>,
}

pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/provision", post(tag_handlers::provision_tag))
        .route("/admin/database/wipe", post(admin_handlers::wipe_database))
        .route("/admin/tags/enroll", post(tag_handlers::enroll_tag))
        .route("/admin/items", get(tag_handlers::list_catalog_items))
        .route("/admin/tags", get(tag_handlers::list_catalog_tags))
        .route("/verify", post(tag_handlers::verify_tag))
        .route(
            "/admin/tags/{tag_id}/revoke",
            post(tag_handlers::revoke_tag),
        )
        .route(
            "/admin/tags/{tag_id}/rotate-key",
            post(tag_handlers::rotate_key),
        )
        .route(
            "/admin/tags/{tag_id}/reconfigure",
            post(tag_handlers::reconfigure_tag),
        )
        .route(
            "/admin/tags/{tag_id}/next-messages",
            post(tag_handlers::next_messages),
        )
        .route(
            "/admin/scan-tokens/{token_id}/revoke",
            post(scan_token_handlers::revoke_scan_token),
        )
        .route("/v1/scan", get(scan_token_handlers::scan_token))
        .route(
            "/v1/products/{pid}/scan-tokens",
            post(scan_token_handlers::generate_scan_tokens),
        )
        .with_state(state)
        .layer(TraceLayer::new_for_http())
}

pub async fn health_check() -> &'static str {
    "OK"
}
