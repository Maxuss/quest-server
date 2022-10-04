use std::{net::SocketAddr, str::FromStr};

use sqlx::PgPool;

use crate::ServerConfig;

#[tracing::instrument(skip_all)]
pub async fn start_api(cfg: &ServerConfig, db: PgPool) -> anyhow::Result<()> {
    let addr = SocketAddr::from_str(&format!("{}:{}", cfg.api.host, cfg.api.port))?;
    tracing::info!("Starting HTTP server on {}", addr);

    Ok(())
}
