use crate::app::error::AppError;
use crate::modules::tags::application::ports::CryptoService;
use aes::Aes128;
use async_trait::async_trait;
use cmac::{Cmac, Mac};
use hmac::Hmac;
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

pub struct AesCmacService {
    master_key: Vec<u8>,
}

impl AesCmacService {
    pub fn new(master_key_hex: &str) -> Result<Self, AppError> {
        let master_key = hex::decode(master_key_hex)
            .map_err(|error| AppError::Internal(format!("Invalid TAG_SIGNING_MASTER: {error}")))?;

        if master_key.len() < 16 {
            return Err(AppError::Internal(
                "TAG_SIGNING_MASTER must be at least 16 bytes encoded as hex".to_string(),
            ));
        }

        Ok(Self { master_key })
    }

    fn derive_key(&self, key_version: i32, tag_uid: &str) -> Result<[u8; 16], AppError> {
        let salt = hex::decode(tag_uid).map_err(|_| {
            AppError::Validation("tag_uid must be a valid uppercase hex string".to_string())
        })?;

        let mut extract = HmacSha256::new_from_slice(&salt)
            .map_err(|error| AppError::Internal(error.to_string()))?;
        extract.update(&self.master_key);
        let prk = extract.finalize().into_bytes();

        let mut expand = HmacSha256::new_from_slice(&prk)
            .map_err(|error| AppError::Internal(error.to_string()))?;
        expand.update(&key_version.to_be_bytes());
        expand.update(&[0x01]);

        let okm = expand.finalize().into_bytes();
        let mut derived_key = [0_u8; 16];
        derived_key.copy_from_slice(&okm[..16]);
        Ok(derived_key)
    }

    fn build_cmac(
        &self,
        key_version: i32,
        tag_uid: &str,
        message: &[u8],
    ) -> Result<Vec<u8>, AppError> {
        let derived_key = self.derive_key(key_version, tag_uid)?;
        let mut mac = Cmac::<Aes128>::new_from_slice(&derived_key)
            .map_err(|error| AppError::Internal(error.to_string()))?;
        mac.update(message);
        Ok(mac.finalize().into_bytes().to_vec())
    }
}

#[async_trait]
impl CryptoService for AesCmacService {
    async fn verify_cmac(
        &self,
        key_version: i32,
        tag_uid: &str,
        message: &[u8],
        signature: &[u8],
    ) -> Result<bool, AppError> {
        let expected = self.build_cmac(key_version, tag_uid, message)?;
        Ok(expected.as_slice() == signature)
    }

    async fn generate_cmac(
        &self,
        key_version: i32,
        tag_uid: &str,
        message: &[u8],
    ) -> Result<Vec<u8>, AppError> {
        self.build_cmac(key_version, tag_uid, message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn derived_cmac_round_trips() {
        let service = AesCmacService::new("000102030405060708090A0B0C0D0E0F").unwrap();
        let message = [0xAA, 0xBB, 0xCC, 0x01];

        let cmac = service
            .generate_cmac(2, "04AABBCCDD", &message)
            .await
            .unwrap();
        let verified = service
            .verify_cmac(2, "04AABBCCDD", &message, &cmac)
            .await
            .unwrap();

        assert!(verified);
    }
}
