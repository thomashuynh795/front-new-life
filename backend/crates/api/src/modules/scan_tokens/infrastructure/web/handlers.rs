use crate::app::http::AppState;
use crate::app::http_error::{map_app_error, require_admin};
use crate::modules::scan_tokens::application::scan_tokens::{
    ConsumeScanTokenRequest, GenerateScanTokensRequest as GenerateTokensCommand,
};
use crate::modules::scan_tokens::domain::entities::StaticScanResult;
use crate::modules::scan_tokens::infrastructure::web::dtos::{
    GenerateScanTokensRequest, GenerateScanTokensResponse, GeneratedScanTokenDto, ScanQuery,
    ScanTokenResponse,
};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use std::sync::Arc;

pub async fn generate_scan_tokens(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(product_public_id): Path<String>,
    Json(payload): Json<GenerateScanTokensRequest>,
) -> impl IntoResponse {
    if let Err(error) = require_admin(&headers, &state.admin_key) {
        return map_app_error(error);
    }

    match state
        .generate_scan_tokens_usecase
        .execute(GenerateTokensCommand {
            product_public_id: product_public_id.clone(),
            count: payload.count,
            ttl_seconds: payload.ttl_seconds,
        })
        .await
    {
        Ok(response) => (
            StatusCode::CREATED,
            Json(GenerateScanTokensResponse {
                product_public_id: response.product_public_id,
                batch_id: response.batch_id,
                tokens: response
                    .tokens
                    .into_iter()
                    .map(|token| GeneratedScanTokenDto {
                        token_id: token.token_id,
                        url: format!(
                            "{}/v1/scan?pid={}&t={}",
                            state.api_base_url, product_public_id, token.token
                        ),
                        token: token.token,
                        expires_at: token.expires_at,
                    })
                    .collect(),
            }),
        )
            .into_response(),
        Err(error) => map_app_error(error),
    }
}

pub async fn revoke_scan_token(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(token_id): Path<uuid::Uuid>,
) -> impl IntoResponse {
    if let Err(error) = require_admin(&headers, &state.admin_key) {
        return map_app_error(error);
    }

    match state.revoke_scan_token_usecase.execute(token_id).await {
        Ok(()) => (StatusCode::OK, "Scan token revoked").into_response(),
        Err(error) => map_app_error(error),
    }
}

pub async fn scan_token(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ScanQuery>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let request = ConsumeScanTokenRequest {
        product_public_id: query.pid,
        token: query.t,
        ip: headers
            .get("x-forwarded-for")
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned),
        user_agent: headers
            .get(axum::http::header::USER_AGENT)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned),
    };

    match state.consume_scan_token_usecase.execute(request).await {
        Ok(response) => (
            status_code_for_scan_result(&response.result),
            Json(ScanTokenResponse {
                product_public_id: response.product_public_id,
                result: response.result.to_string(),
                authentic: response.authentic,
            }),
        )
            .into_response(),
        Err(error) => map_app_error(error),
    }
}

fn status_code_for_scan_result(result: &StaticScanResult) -> StatusCode {
    match result {
        StaticScanResult::Ok => StatusCode::OK,
        StaticScanResult::Replay => StatusCode::CONFLICT,
        StaticScanResult::Invalid => StatusCode::UNAUTHORIZED,
        StaticScanResult::Expired => StatusCode::GONE,
        StaticScanResult::Revoked => StatusCode::FORBIDDEN,
        StaticScanResult::NotFound => StatusCode::NOT_FOUND,
    }
}
