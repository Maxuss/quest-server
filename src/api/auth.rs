use super::model::Payload;
use crate::{
    api::model::{Error, ServerError},
    common::{
        data::{BsonId, RegStageUser, User},
        mongo::MongoDatabase,
    },
};
use axum::{
    body::StreamBody,
    extract::{Path, State},
    http::header,
    response::IntoResponse,
    Json,
};
use axum::extract::BodyStream;
use mongodb::bson::doc;
use serde::{Deserialize, Serialize};
use tokio_util::compat::FuturesAsyncReadCompatExt;
use tokio_util::io::ReaderStream;
use tracing::warn;
use uuid::Uuid;

// POST models
#[derive(Debug, Clone, Deserialize)]
pub struct RegisterUser {
    card_hash: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConfirmUserRegister {
    user_id: BsonId,
    telegram_chat_id: i64,
    username: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RegisteredUser {
    user_hash: String,
    user_id: BsonId,
    telegram_chat_id: i64
}

#[axum_macros::debug_handler]
#[tracing::instrument(skip(db))]
pub async fn register(
    State(db): State<MongoDatabase>,
    Json(data): Json<RegisterUser>,
) -> Payload<RegStageUser> {
    let user = RegStageUser {
        card_hash: data.card_hash,
        id: BsonId::new(),
    };

    db.reg_stage.insert_one(user.clone(), None).await?;

    Payload(user)
}

#[axum_macros::debug_handler]
#[tracing::instrument(skip(db))]
pub async fn confirm_register(
    State(db): State<MongoDatabase>,
    Json(data): Json<ConfirmUserRegister>
) -> Payload<RegisteredUser> {
    let reg_stage = db.reg_stage.find_one(doc! { "id": data.user_id }, None).await?;

    if let Some(reg) = reg_stage {
        let registered_user = User {
            card_hash: reg.card_hash.clone(),
            id: reg.id.clone(),
            username: data.username,
            telegram_chat_id: data.telegram_chat_id
        };
        db.users.insert_one(registered_user.clone(), None).await?;
        Payload(RegisteredUser {
            user_hash: reg.card_hash.clone(),
            user_id: reg.id,
            telegram_chat_id: data.telegram_chat_id
        })
    } else {
        warn!("Attempted to register unknown user.");
        Err(ServerError::NOT_FOUND("Could not find user".to_owned()))
    }
}

pub async fn set_avatar(
    State(db): State<MongoDatabase>,
    stream: BodyStream
) -> Payload<bool> {
    Payload(true)
}

#[tracing::instrument(skip(db))]
pub async fn get_user(Path(hash): Path<String>, State(db): State<MongoDatabase>) -> Payload<User> {
    if hash.len() != 64 {
        return Error(ServerError::INVALID_FORMAT(format!(
            "SHA256 provided hash is not of valid SHA256 length ({} != 64)",
            hash.len()
        )));
    }

    let expected_user = db.users.find_one(doc! { "card_hash": &hash }, None).await?;

    let user = if let Some(user) = expected_user {
        user
    } else {
        return Error(ServerError::NOT_FOUND(format!(
            "Could not find user with SHA256 card hash {hash}!"
        )));
    };

    Payload(user)
}

#[tracing::instrument(skip(db))]
pub async fn get_avatar(
    Path(id): Path<Uuid>,
    State(db): State<MongoDatabase>,
) -> axum::response::Result<impl IntoResponse, ServerError> {
    let id = mongodb::bson::Uuid::from_uuid_1(id);
    let expected_user = db.users.find_one(doc! { "id": id }, None).await?;
    let any = db.users.find_one(doc! {}, None).await?;

    let user = if let Some(user) = expected_user {
        user
    } else {
        return Err(ServerError::NOT_FOUND(format!(
            "Could not find user with id {id}!"
        )));
    };

    let stream: mongodb::GridFsDownloadStream = db
        .gridfs
        .open_download_stream_by_name(format!("{}.png", user.card_hash), None)
        .await?;
    let body = StreamBody::new(ReaderStream::new(stream.compat()));

    let headers = [(header::CONTENT_TYPE, "image/png")];

    Ok((headers, body))
}
