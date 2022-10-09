use super::model::Payload;
use crate::common::data::RegStageUser;
use axum::{Extension, Json};
use serde::Deserialize;
use sqlx::PgPool;
use tracing::{debug, log::warn};
use uuid::Uuid;

// POST models

#[derive(Debug, Clone, Deserialize)]
pub struct Stage1Register {
    card_hash: String,
}

#[tracing::instrument(skip(pool))]
pub async fn register(
    Json(data): Json<Stage1Register>,
    Extension(pool): Extension<PgPool>,
) -> Payload<RegStageUser> {
    debug!("Performing stage 1 register!");

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
