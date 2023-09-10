use std::{net::SocketAddr, str::FromStr};

use axum::{
    http::{Method, Uri},
    routing::{get, post},
    Router,
};

use crate::{api::auth::*, common::mongo::MongoDatabase, ServerConfig};

use self::model::ServerError;

mod auth;
pub mod model;
mod openapi;

#[tracing::instrument(skip_all)]
pub async fn start_api(cfg: &ServerConfig, db: MongoDatabase) -> anyhow::Result<()> {
    let addr = SocketAddr::from_str(&format!("{}:{}", cfg.api.host, cfg.api.port))?;
    tracing::info!("Starting HTTP server on {}", addr);

    let router = Router::new()
        .route("/user/register", post(register))
        .route("/user/get/:hash", get(get_user))
        .route("/user/avatar/:id", get(get_avatar))
        .route("/api", get(openapi::openapi_route))
        .route("/resources/openapi.yml", get(openapi::openapi_yml_route))
        .fallback(handler404)
        .with_state(db);

    axum::Server::bind(&addr)
        .serve(router.into_make_service())
        .await?;

    Ok(())
}

async fn handler404(path: Uri, method: Method) -> ServerError {
    ServerError::NOT_FOUND(format!("Endpoint `{path}` for method `{method}` not found"))
}
