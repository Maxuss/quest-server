use serde::{Deserialize, Serialize};

pub type BsonId = mongodb::bson::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegStageUser {
    pub card_hash: String,
    pub id: BsonId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub card_hash: String,
    pub id: BsonId,
    pub username: String,
    pub telegram_chat_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LingeringTask {
    pub id: BsonId,
    pub assigned_to: String,
    pub quest_name: String,
}
