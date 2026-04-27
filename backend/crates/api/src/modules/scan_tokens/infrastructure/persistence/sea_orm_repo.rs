use crate::app::error::AppError;
use crate::modules::scan_tokens::application::ports::ScanTokenRepository;
use crate::modules::scan_tokens::domain::entities::{
    ScanToken, ScanTokenStatus, TokenBatch, TokenBatchStatus,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use database_model::{scan_token, token_batch};
use sea_orm::sea_query::Expr;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use std::sync::Arc;
use uuid::Uuid;

impl TryFrom<scan_token::Model> for ScanToken {
    type Error = AppError;

    fn try_from(model: scan_token::Model) -> Result<Self, Self::Error> {
        let status = match model.status.as_str() {
            "UNUSED" => ScanTokenStatus::Unused,
            "USED" => ScanTokenStatus::Used,
            "REVOKED" => ScanTokenStatus::Revoked,
            _ => return Err(AppError::Internal("Unknown scan token status".to_string())),
        };

        Ok(ScanToken {
            token_id: model.token_id,
            batch_id: model.batch_id,
            tag_id: model.tag_id,
            product_public_id: model.product_public_id,
            expires_at: model.expires_at.into(),
            status,
            created_at: model.created_at.into(),
            used_at: model.used_at.map(Into::into),
            used_ip: model.used_ip,
            used_user_agent: model.used_user_agent,
            revoked_at: model.revoked_at.map(Into::into),
            token_hash: model.token_hash,
        })
    }
}

pub struct SeaOrmScanTokenRepository {
    db: Arc<DatabaseConnection>,
}

impl SeaOrmScanTokenRepository {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl ScanTokenRepository for SeaOrmScanTokenRepository {
    async fn save_many(&self, tokens: &[ScanToken]) -> Result<(), AppError> {
        if tokens.is_empty() {
            return Ok(());
        }

        let active_models = tokens.iter().map(|token| scan_token::ActiveModel {
            token_id: Set(token.token_id),
            batch_id: Set(token.batch_id),
            tag_id: Set(token.tag_id),
            product_public_id: Set(token.product_public_id.clone()),
            expires_at: Set(token.expires_at.into()),
            status: Set(token.status.to_string()),
            created_at: Set(token.created_at.into()),
            used_at: Set(token.used_at.map(Into::into)),
            used_ip: Set(token.used_ip.clone()),
            used_user_agent: Set(token.used_user_agent.clone()),
            revoked_at: Set(token.revoked_at.map(Into::into)),
            token_hash: Set(token.token_hash.clone()),
        });

        scan_token::Entity::insert_many(active_models)
            .exec(&*self.db)
            .await
            .map_err(|error| AppError::Internal(error.to_string()))?;

        Ok(())
    }

    async fn save_batch(&self, batch: &TokenBatch) -> Result<TokenBatch, AppError> {
        let active_model = token_batch::ActiveModel {
            id: Set(batch.id),
            tag_id: Set(batch.tag_id),
            product_public_id: Set(batch.product_public_id.clone()),
            status: Set(batch.status.to_string()),
            expires_at: Set(batch.expires_at.into()),
            created_at: Set(batch.created_at.into()),
            revoked_at: Set(batch.revoked_at.map(Into::into)),
        };

        token_batch::Entity::insert(active_model)
            .exec(&*self.db)
            .await
            .map_err(|error| AppError::Internal(error.to_string()))?;

        Ok(batch.clone())
    }

    async fn find_by_id(&self, token_id: Uuid) -> Result<Option<ScanToken>, AppError> {
        let result = scan_token::Entity::find_by_id(token_id)
            .one(&*self.db)
            .await
            .map_err(|error| AppError::Internal(error.to_string()))?;

        result.map(ScanToken::try_from).transpose()
    }

    async fn revoke(&self, token_id: Uuid, revoked_at: DateTime<Utc>) -> Result<bool, AppError> {
        let result = scan_token::Entity::update_many()
            .col_expr(scan_token::Column::Status, Expr::value("REVOKED"))
            .col_expr(scan_token::Column::RevokedAt, Expr::value(Some(revoked_at)))
            .filter(scan_token::Column::TokenId.eq(token_id))
            .filter(scan_token::Column::Status.ne("USED"))
            .exec(&*self.db)
            .await
            .map_err(|error| AppError::Internal(error.to_string()))?;

        Ok(result.rows_affected == 1)
    }

    async fn revoke_active_batch_for_tag(
        &self,
        tag_id: Uuid,
        revoked_at: DateTime<Utc>,
    ) -> Result<u64, AppError> {
        let batch_result = token_batch::Entity::update_many()
            .col_expr(token_batch::Column::Status, Expr::value("REVOKED"))
            .col_expr(
                token_batch::Column::RevokedAt,
                Expr::value(Some(revoked_at)),
            )
            .filter(token_batch::Column::TagId.eq(tag_id))
            .filter(token_batch::Column::Status.eq("ACTIVE"))
            .exec(&*self.db)
            .await
            .map_err(|error| AppError::Internal(error.to_string()))?;

        scan_token::Entity::update_many()
            .col_expr(scan_token::Column::Status, Expr::value("REVOKED"))
            .col_expr(scan_token::Column::RevokedAt, Expr::value(Some(revoked_at)))
            .filter(scan_token::Column::TagId.eq(tag_id))
            .filter(scan_token::Column::Status.eq("UNUSED"))
            .exec(&*self.db)
            .await
            .map_err(|error| AppError::Internal(error.to_string()))?;

        Ok(batch_result.rows_affected)
    }

    async fn consume_if_unused(
        &self,
        token_id: Uuid,
        used_at: DateTime<Utc>,
        used_ip: Option<String>,
        used_user_agent: Option<String>,
    ) -> Result<bool, AppError> {
        let result = scan_token::Entity::update_many()
            .col_expr(scan_token::Column::Status, Expr::value("USED"))
            .col_expr(scan_token::Column::UsedAt, Expr::value(used_at))
            .col_expr(scan_token::Column::UsedIp, Expr::value(used_ip))
            .col_expr(
                scan_token::Column::UsedUserAgent,
                Expr::value(used_user_agent),
            )
            .filter(scan_token::Column::TokenId.eq(token_id))
            .filter(scan_token::Column::Status.eq("UNUSED"))
            .exec(&*self.db)
            .await
            .map_err(|error| AppError::Internal(error.to_string()))?;

        Ok(result.rows_affected == 1)
    }
}

impl TryFrom<token_batch::Model> for TokenBatch {
    type Error = AppError;

    fn try_from(model: token_batch::Model) -> Result<Self, Self::Error> {
        let status = match model.status.as_str() {
            "ACTIVE" => TokenBatchStatus::Active,
            "REVOKED" => TokenBatchStatus::Revoked,
            _ => return Err(AppError::Internal("Unknown token batch status".to_string())),
        };

        Ok(TokenBatch {
            id: model.id,
            tag_id: model.tag_id,
            product_public_id: model.product_public_id,
            status,
            expires_at: model.expires_at.into(),
            created_at: model.created_at.into(),
            revoked_at: model.revoked_at.map(Into::into),
        })
    }
}
