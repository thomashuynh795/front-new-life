use anyhow::{Context, Result};

pub struct Config {
    pub address: String,
    pub database_url: String,
}

impl Config {
    /// Builds the application configuration from environment variables.
    ///
    /// # Errors
    ///
    /// Returns an error when `DATABASE_URL` is missing.
    pub fn from_env() -> Result<Self> {
        let address = std::env::var("ADDRESS").unwrap_or_else(|_| "0.0.0.0:8080".into());
        let database_url =
            std::env::var("DATABASE_URL").context("DATABASE_URL is required in the root .env")?;

        Ok(Self {
            address,
            database_url,
        })
    }
}
