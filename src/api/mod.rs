use std::{net::SocketAddr, str::FromStr};

use axum::{routing::post, Extension, Json, Router};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tracing::info;

use crate::ServerConfig;

use self::model::Payload;

pub mod model;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestData {
    hello: String,
    world: String,
}

#[tracing::instrument]
async fn test_post(Json(data): Json<TestData>) -> Payload<TestData> {
    info!("Echoing data!");
    Payload(data)
}

#[tracing::instrument(skip_all)]
pub async fn start_api(cfg: &ServerConfig, db: PgPool) -> anyhow::Result<()> {
    let addr = SocketAddr::from_str(&format!("{}:{}", cfg.api.host, cfg.api.port))?;
    tracing::info!("Starting HTTP server on {}", addr);

    let router = Router::new()
        .route("/test_post", post(test_post))
        .layer(Extension(db));

    axum::Server::bind(&addr)
        .serve(router.into_make_service())
        .await?;

    Ok(())
}
