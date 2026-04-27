use anyhow::{Context, Result};
use api::{app, config, db};
use axum::http::Method;
use std::path::PathBuf;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn build_cors_layer_for_demo() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers(Any)
        .allow_credentials(false)
}

#[tokio::main]
async fn main() -> Result<()> {
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .context("workspace root should exist")?
        .to_path_buf();
    let env_file = workspace_root.join(".env");
    if env_file.exists() {
        dotenvy::from_path(&env_file)
            .with_context(|| format!("Failed to load .env from {}", env_file.display()))?;
    }

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "api=debug,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = config::Config::from_env()?;
    let database_connection = db::connect(&config.database_url)
        .await
        .context("Failed to connect to Postgres")?;

    let state = Arc::new(app::http::AppState {
        database_connection: Arc::new(database_connection),
    });

    let cors = build_cors_layer_for_demo();
    let app = app::http::create_router(state).layer(cors);

    let listener = tokio::net::TcpListener::bind(&config.address)
        .await
        .with_context(|| format!("Failed to bind TCP listener on {}", config.address))?;
    tracing::info!("Listening on {}", config.address);

    axum::serve(listener, app)
        .await
        .context("HTTP server terminated with an error")?;

    Ok(())
}
