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
use mongodb::bson::doc;
use serde::{Deserialize, Serialize};
use tokio_util::compat::FuturesAsyncReadCompatExt;
use tokio_util::io::ReaderStream;
use uuid::Uuid;

// POST models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterUser {
    card_hash: String,
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
