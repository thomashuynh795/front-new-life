use crate::app::http::AppState;
use crate::app::http_error::{map_app_error, require_admin};
use crate::modules::tags::application::admin::{
    NextMessagesRequest as NextMessagesCommand, ReconfigureTagRequest as ReconfigureTagCommand,
};
use crate::modules::tags::application::provision::{
    EnrollTagRequest as EnrollTagCommand, OneTimeTokenRecord, TagWritePayload,
};
use crate::modules::tags::application::verify::VerifyRequest;
use crate::modules::tags::domain::entities::{ScanVerdict, TagMode};
use crate::modules::tags::infrastructure::web::dtos::{
    CatalogItemDto, CatalogItemSummaryDto, CatalogTagDto, CatalogTagSummaryDto, EnrollTagRequest,
    EnrollTagResponse, GeneratedMessageDto, GeneratedTokenDto, NextMessagesRequest,
    NextMessagesResponse, ProductDto, ReconfigurePayloadDto, ReconfigureTagRequest,
    ReconfigureTagResponse, RotateKeyResponse, TagPayloadDto, VerifyHintDto, VerifyTagRequest,
    VerifyTagResponse,
};
use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use serde_json::json;
use std::sync::Arc;

pub async fn enroll_tag(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(payload): Json<EnrollTagRequest>,
) -> impl IntoResponse {
    if let Err(error) = require_admin(&headers, &state.admin_key) {
        return map_app_error(error);
    }

    let mode = match TagMode::try_from(payload.mode.as_str()) {
        Ok(mode) => mode,
        Err(message) => return map_app_error(crate::app::error::AppError::Validation(message)),
    };

    match state
        .enroll_usecase
        .execute(EnrollTagCommand {
            tag_uid: payload.tag_uid,
            product_code: payload.product_code,
            size: payload.size,
            color: payload.color,
            mode,
        })
        .await
    {
        Ok(response) => (
            StatusCode::CREATED,
            Json(EnrollTagResponse {
                tag_id: response.tag_id,
                item_id: response.item_id,
                mode: response.mode.to_string(),
                payload: map_tag_payload(&state.api_base_url, response.payload),
            }),
        )
            .into_response(),
        Err(error) => map_app_error(error),
    }
}

pub async fn provision_tag(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(payload): Json<EnrollTagRequest>,
) -> impl IntoResponse {
    enroll_tag(State(state), headers, Json(payload)).await
}

pub async fn verify_tag(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<VerifyTagRequest>,
) -> impl IntoResponse {
    let request = VerifyRequest {
        tag_uid: payload.tag_uid,
        counter: payload.counter,
        cmac: payload.cmac,
    };

    match state.verify_usecase.execute(request).await {
        Ok(response) => (
            status_code_for_verify_verdict(&response.verdict),
            Json(VerifyTagResponse {
                verdict: response.verdict.to_string(),
                product: response.product_info.map(|product| ProductDto {
                    tag_id: product.tag_id,
                }),
            }),
        )
            .into_response(),
        Err(error) => map_app_error(error),
    }
}

pub async fn revoke_tag(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    axum::extract::Path(tag_id): axum::extract::Path<uuid::Uuid>,
) -> impl IntoResponse {
    if let Err(error) = require_admin(&headers, &state.admin_key) {
        return map_app_error(error);
    }

    match state.revoke_usecase.execute(tag_id).await {
        Ok(()) => (StatusCode::OK, "Tag revoked").into_response(),
        Err(error) => map_app_error(error),
    }
}

pub async fn rotate_key(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    axum::extract::Path(tag_id): axum::extract::Path<uuid::Uuid>,
) -> impl IntoResponse {
    if let Err(error) = require_admin(&headers, &state.admin_key) {
        return map_app_error(error);
    }

    match state.rotate_usecase.execute(tag_id).await {
        Ok(new_key_version) => (
            StatusCode::OK,
            Json(RotateKeyResponse {
                new_key_version,
                counter_reset: true,
                message: "Key rotated successfully".to_string(),
            }),
        )
            .into_response(),
        Err(error) => map_app_error(error),
    }
}

pub async fn reconfigure_tag(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    axum::extract::Path(tag_id): axum::extract::Path<uuid::Uuid>,
    Json(payload): Json<ReconfigureTagRequest>,
) -> impl IntoResponse {
    if let Err(error) = require_admin(&headers, &state.admin_key) {
        return map_app_error(error);
    }

    match state
        .reconfigure_usecase
        .execute(ReconfigureTagCommand {
            tag_id,
            reset_counter: payload.reset_counter.unwrap_or(false),
            rotate_key: payload.rotate_key.unwrap_or(false),
            revoke_existing_batch: payload.revoke_existing_batch.unwrap_or(true),
            token_count: payload.token_count,
            ttl_seconds: payload.ttl_seconds,
        })
        .await
    {
        Ok(response) => (
            StatusCode::OK,
            Json(ReconfigureTagResponse {
                tag_id: response.tag_id,
                mode: response.mode.to_string(),
                payload: match response.payload {
                    crate::modules::tags::application::admin::ReconfigurePayload::DynamicCmac {
                        key_version,
                        counter_initial,
                    } => ReconfigurePayloadDto::DynamicCmac {
                        key_version,
                        counter_initial,
                    },
                    crate::modules::tags::application::admin::ReconfigurePayload::OneTimeTokens {
                        revoked_batches,
                        batch_id,
                        records,
                    } => ReconfigurePayloadDto::OneTimeTokens {
                        revoked_batches,
                        batch_id,
                        records: map_generated_tokens(&state.api_base_url, records, None),
                    },
                },
            }),
        )
            .into_response(),
        Err(error) => map_app_error(error),
    }
}

pub async fn next_messages(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    axum::extract::Path(tag_id): axum::extract::Path<uuid::Uuid>,
    Json(payload): Json<NextMessagesRequest>,
) -> impl IntoResponse {
    if let Err(error) = require_admin(&headers, &state.admin_key) {
        return map_app_error(error);
    }

    match state
        .next_messages_usecase
        .execute(NextMessagesCommand {
            tag_id,
            count: payload.count.unwrap_or(3),
            starting_counter: payload.starting_counter,
        })
        .await
    {
        Ok(response) => (
            StatusCode::OK,
            Json(NextMessagesResponse {
                tag_id: response.tag_id,
                tag_uid: response.tag_uid.clone(),
                key_version: response.key_version,
                messages: response
                    .messages
                    .into_iter()
                    .map(|message| GeneratedMessageDto {
                        counter: message.counter,
                        cmac: message.cmac,
                    })
                    .collect(),
                verify_hint: VerifyHintDto {
                    endpoint: "/verify".to_string(),
                    body_template: json!({
                        "tag_uid": response.tag_uid,
                        "counter": "<counter>",
                        "cmac": "<cmac>",
                    }),
                },
            }),
        )
            .into_response(),
        Err(error) => map_app_error(error),
    }
}

pub async fn list_catalog_items(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(error) = require_admin(&headers, &state.admin_key) {
        return map_app_error(error);
    }

    match state.list_catalog_items_usecase.execute().await {
        Ok(items) => (
            StatusCode::OK,
            Json(
                items
                    .into_iter()
                    .map(|entry| CatalogItemDto {
                        item_id: entry.item.id,
                        product_code: entry.item.product_code,
                        size: entry.item.size,
                        color: entry.item.color,
                        created_at: entry.item.created_at,
                        updated_at: entry.item.updated_at,
                        tag: CatalogTagSummaryDto {
                            tag_id: entry.tag.id,
                            tag_uid: entry.tag.tag_uid,
                            mode: entry.tag.mode.to_string(),
                            status: entry.tag.status.to_string(),
                            key_version: entry.tag.key_version,
                            last_counter: entry.tag.last_counter,
                        },
                    })
                    .collect::<Vec<_>>(),
            ),
        )
            .into_response(),
        Err(error) => map_app_error(error),
    }
}

pub async fn list_catalog_tags(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(error) = require_admin(&headers, &state.admin_key) {
        return map_app_error(error);
    }

    match state.list_catalog_tags_usecase.execute().await {
        Ok(tags) => (
            StatusCode::OK,
            Json(
                tags.into_iter()
                    .map(|entry| CatalogTagDto {
                        tag_id: entry.tag.id,
                        tag_uid: entry.tag.tag_uid,
                        mode: entry.tag.mode.to_string(),
                        status: entry.tag.status.to_string(),
                        key_version: entry.tag.key_version,
                        last_counter: entry.tag.last_counter,
                        created_at: entry.tag.created_at,
                        updated_at: entry.tag.updated_at,
                        item: entry.item.map(|item| CatalogItemSummaryDto {
                            item_id: item.id,
                            product_code: item.product_code,
                            size: item.size,
                            color: item.color,
                        }),
                    })
                    .collect::<Vec<_>>(),
            ),
        )
            .into_response(),
        Err(error) => map_app_error(error),
    }
}

fn map_tag_payload(base_url: &str, payload: TagWritePayload) -> TagPayloadDto {
    match payload {
        TagWritePayload::DynamicCmac {
            key_version,
            counter_initial,
            verify_format,
        } => TagPayloadDto::DynamicCmac {
            key_version,
            counter_initial,
            verify_format,
        },
        TagWritePayload::OneTimeTokens { batch_id, records } => TagPayloadDto::OneTimeTokens {
            batch_id,
            records: map_generated_tokens(base_url, records, None),
        },
    }
}

fn map_generated_tokens(
    base_url: &str,
    records: Vec<OneTimeTokenRecord>,
    product_public_id: Option<&str>,
) -> Vec<GeneratedTokenDto> {
    records
        .into_iter()
        .map(|record| {
            let pid = product_public_id.unwrap_or_default();
            let url = if pid.is_empty() {
                format!("{base_url}{}", record.url)
            } else {
                format!("{base_url}/v1/scan?pid={pid}&t={}", record.token)
            };
            GeneratedTokenDto {
                token_id: record.token_id,
                token: record.token,
                url,
                expires_at: record.expires_at,
            }
        })
        .collect()
}

fn status_code_for_verify_verdict(verdict: &ScanVerdict) -> StatusCode {
    match verdict {
        ScanVerdict::Valid => StatusCode::OK,
        ScanVerdict::InvalidSignature => StatusCode::UNAUTHORIZED,
        ScanVerdict::ReplayDetected => StatusCode::CONFLICT,
        ScanVerdict::TagRevoked => StatusCode::GONE,
        ScanVerdict::TagNotFound => StatusCode::NOT_FOUND,
    }
}
