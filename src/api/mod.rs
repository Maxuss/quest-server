use std::{net::SocketAddr, str::FromStr};

use axum::{
    routing::{get, post},
    Extension, Router,
};

use sqlx::PgPool;

use crate::{api::auth::*, ServerConfig};

mod auth;
pub mod model;

#[tracing::instrument(skip_all)]
pub async fn start_api(cfg: &ServerConfig, db: PgPool) -> anyhow::Result<()> {
    let addr = SocketAddr::from_str(&format!("{}:{}", cfg.api.host, cfg.api.port))?;
    tracing::info!("Starting HTTP server on {}", addr);

    let router = Router::new()
        .route("/user/register", post(register))
        .route("/user/get/:hash", get(get_id))
        .layer(Extension(db));

    axum::Server::bind(&addr)
        .serve(router.into_make_service())
        .await?;

    Ok(())
}
