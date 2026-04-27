use crate::app::error::AppError;
use crate::modules::scan_tokens::application::ports::{
    GeneratedScanToken, ScanTokenService, VerifiedScanToken,
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, TimeZone, Utc};
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

pub struct HmacScanTokenService {
    secret: Vec<u8>,
}

impl HmacScanTokenService {
    pub fn new(secret: impl AsRef<[u8]>) -> Self {
        Self {
            secret: secret.as_ref().to_vec(),
        }
    }

    fn build_mac(
        &self,
        token_id: Uuid,
        product_public_id: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<[u8; 32], AppError> {
        let mut mac = HmacSha256::new_from_slice(&self.secret)
            .map_err(|error| AppError::Internal(error.to_string()))?;
        mac.update(token_id.as_bytes());
        mac.update(product_public_id.as_bytes());
        mac.update(&expires_at.timestamp().to_be_bytes());
        let bytes = mac.finalize().into_bytes();

        let mut output = [0_u8; 32];
        output.copy_from_slice(&bytes);
        Ok(output)
    }
}

impl ScanTokenService for HmacScanTokenService {
    fn generate_token(
        &self,
        product_public_id: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<GeneratedScanToken, AppError> {
        let token_id = Uuid::new_v4();
        let mac = self.build_mac(token_id, product_public_id, expires_at)?;

        let mut payload = Vec::with_capacity(56);
        payload.extend_from_slice(token_id.as_bytes());
        payload.extend_from_slice(&expires_at.timestamp().to_be_bytes());
        payload.extend_from_slice(&mac);

        Ok(GeneratedScanToken {
            token_id,
            token: URL_SAFE_NO_PAD.encode(payload),
        })
    }

    fn parse_and_verify_token(
        &self,
        product_public_id: &str,
        token: &str,
    ) -> Result<VerifiedScanToken, AppError> {
        let payload = URL_SAFE_NO_PAD
            .decode(token)
            .map_err(|error| AppError::Validation(error.to_string()))?;

        if payload.len() != 56 {
            return Err(AppError::Validation(
                "Scan token payload length is invalid".to_string(),
            ));
        }

        let token_id = Uuid::from_slice(&payload[..16])
            .map_err(|error| AppError::Validation(error.to_string()))?;
        let expires_at_bytes: [u8; 8] = payload[16..24]
            .try_into()
            .map_err(|_| AppError::Validation("expires_at is invalid".to_string()))?;
        let expires_at = Utc
            .timestamp_opt(i64::from_be_bytes(expires_at_bytes), 0)
            .single()
            .ok_or_else(|| AppError::Validation("expires_at is invalid".to_string()))?;
        let expected_mac = self.build_mac(token_id, product_public_id, expires_at)?;

        if expected_mac.as_slice() != &payload[24..56] {
            return Err(AppError::Validation(
                "Scan token MAC is invalid".to_string(),
            ));
        }

        Ok(VerifiedScanToken {
            token_id,
            expires_at,
        })
    }

    fn hash_token(&self, token: &str) -> Vec<u8> {
        Sha256::digest(token.as_bytes()).to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn generated_token_round_trips() {
        let service = HmacScanTokenService::new("test-secret");
        let expires_at = Utc::now() + Duration::hours(1);

        let generated = service.generate_token("PID-123", expires_at).unwrap();
        let verified = service
            .parse_and_verify_token("PID-123", &generated.token)
            .unwrap();

        assert_eq!(verified.token_id, generated.token_id);
        assert_eq!(verified.expires_at.timestamp(), expires_at.timestamp());
    }

    #[test]
    fn token_is_bound_to_product_public_id() {
        let service = HmacScanTokenService::new("test-secret");
        let expires_at = Utc::now() + Duration::hours(1);
        let generated = service.generate_token("PID-123", expires_at).unwrap();

        let error = service
            .parse_and_verify_token("PID-999", &generated.token)
            .unwrap_err();

        assert!(matches!(error, AppError::Validation(_)));
    }
}
