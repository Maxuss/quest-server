use std::{net::SocketAddr, str::FromStr};

use axum::{
    handler::Handler,
    http::{Method, Uri},
    routing::{get, post},
    Extension, Router,
};

use sqlx::PgPool;

use crate::{api::auth::*, ServerConfig};

use self::model::ServerError;

mod auth;
pub mod model;
mod openapi;

#[tracing::instrument(skip_all)]
pub async fn start_api(cfg: &ServerConfig, db: PgPool) -> anyhow::Result<()> {
    let addr = SocketAddr::from_str(&format!("{}:{}", cfg.api.host, cfg.api.port))?;
    tracing::info!("Starting HTTP server on {}", addr);

    let router = Router::new()
        .route("/user/register", post(register))
        .route("/user/get/:hash", get(get_user))
        .route("/user/avatar/:id", get(get_avatar))
        .route("/api", get(openapi::openapi_route))
        .route("/resources/openapi.yml", get(openapi::openapi_yml_route))
        .fallback(handler404.into_service())
        .layer(Extension(db));

    axum::Server::bind(&addr)
        .serve(router.into_make_service())
        .await?;

    Ok(())
}

async fn handler404(path: Uri, method: Method) -> ServerError {
    ServerError::NOT_FOUND(format!("Endpoint `{path}` for method `{method}` not found"))
}
