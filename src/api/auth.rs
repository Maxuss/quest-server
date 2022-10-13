use super::model::Payload;
use crate::{
    api::model::{Error, ServerError},
    common::data::{RegStageUser, User},
};
use axum::{body::Body, extract::Path, http::Request, response::Response, Extension, Json};
use serde::Deserialize;
use sqlx::PgPool;
use tower_http::services::fs::ServeFile;
use tower_service::Service;
use tracing::{debug, log::warn};
use uuid::Uuid;

// POST models
#[derive(Debug, Clone, Deserialize)]
pub struct RegisterUser {
    card_hash: String,
}

#[tracing::instrument(skip(pool))]
pub async fn register(
    Json(data): Json<RegisterUser>,
    Extension(pool): Extension<PgPool>,
) -> Payload<RegStageUser> {
    let user = RegStageUser {
        card_hash: data.card_hash,
        id: Uuid::new_v4(),
    };

    let rows = sqlx::query("INSERT INTO users_reg_state VALUES($1, $2)")
        .bind(&user.card_hash)
        .bind(user.id)
        .execute(&pool)
        .await?;

    if rows.rows_affected() != 1 {
        warn!(
            "Invalid amount of rows affect, expected 1 but got {}",
            rows.rows_affected()
        )
    }

    Payload(user)
}

#[tracing::instrument(skip(pool))]
pub async fn get_user(
    Path(hash): Path<String>,
    Extension(pool): Extension<PgPool>,
) -> Payload<User> {
    debug!("Client tried to get id of user with sha256 hash of {hash}");

    if hash.len() != 64 {
        return Error(ServerError::INVALID_FORMAT(format!(
            "SHA256 provided hash is not of valid SHA256 length ({} != 64)",
            hash.len()
        )));
    }

    let expected_user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE card_hash = $1")
        .bind(&hash)
        .fetch_optional(&pool)
        .await?;

    let user = if let Some(user) = expected_user {
        user
    } else {
        return Error(ServerError::NOT_FOUND(format!(
            "Could not find user with SHA256 card hash {hash}!"
        )));
    };

    Payload(user)
}

#[tracing::instrument]
pub async fn get_avatar(
    Path(id): Path<Uuid>,
    Extension(pool): Extension<PgPool>,
    req: Request<Body>,
) -> axum::response::Result<
    Response<tower_http::services::fs::ServeFileSystemResponseBody>,
    ServerError,
> {
    let expected_user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(id)
        .fetch_optional(&pool)
        .await?;

    let user = if let Some(user) = expected_user {
        user
    } else {
        return Err(ServerError::NOT_FOUND(format!(
            "Could not find user with id {id}!"
        )));
    };

    ServeFile::new(format!("data/image/{}.png", user.card_hash))
        .call(req)
        .await
        .map_err(crate::api::model::ServerError::from)
}
