use anyhow::{Context, Result};

pub struct Config {
    pub address: String,
    pub database_url: String,
    pub api_domain: String,
    pub token_secret: String,
    pub admin_key: String,
    pub db_wipe_token: Option<String>,
    pub tag_signing_master: String,
    pub default_scan_token_batch_size: u32,
    pub default_scan_token_ttl_seconds: i64,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let address = std::env::var("ADDRESS").unwrap_or_else(|_| "0.0.0.0:8080".into());
        let database_url =
            std::env::var("DATABASE_URL").context("DATABASE_URL is required in the root .env")?;
        let api_domain = std::env::var("API_DOMAIN").unwrap_or_else(|_| "localhost:8080".into());
        let token_secret = std::env::var("TOKEN_SECRET")
            .or_else(|_| std::env::var("HMAC_SECRET"))
            .or_else(|_| std::env::var("SCAN_TOKEN_SECRET"))
            .context(
                "TOKEN_SECRET or legacy SCAN_TOKEN_SECRET/HMAC_SECRET is required in the root .env",
            )?;
        let admin_key = std::env::var("ADMIN_KEY")
            .or_else(|_| std::env::var("ADMIN_API_KEY"))
            .context("ADMIN_KEY is required in the root .env")?;
        let db_wipe_token = std::env::var("DB_WIPE_TOKEN")
            .or_else(|_| std::env::var("WIPE_DB_TOKEN"))
            .ok();
        let tag_signing_master = std::env::var("TAG_SIGNING_MASTER")
            .or_else(|_| std::env::var("MASTER_KEY_HEX"))
            .context("TAG_SIGNING_MASTER is required in the root .env")?;
        let default_scan_token_batch_size = std::env::var("DEFAULT_SCAN_TOKEN_BATCH_SIZE")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(3);
        let default_scan_token_ttl_seconds = std::env::var("DEFAULT_SCAN_TOKEN_TTL_SECONDS")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(86_400);

        Ok(Self {
            address,
            database_url,
            api_domain,
            token_secret,
            admin_key,
            db_wipe_token,
            tag_signing_master,
            default_scan_token_batch_size,
            default_scan_token_ttl_seconds,
        })
    }

    pub fn scan_base_url(&self) -> String {
        if self.api_domain.starts_with("http://") || self.api_domain.starts_with("https://") {
            self.api_domain.trim_end_matches('/').to_string()
        } else {
            format!("https://{}", self.api_domain.trim_end_matches('/'))
        }
    }
}
