use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RegStageUser {
    pub card_hash: String,
    pub id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub card_hash: String,
    pub id: Uuid,
    pub username: String,
    pub telegram_chat_id: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct LingeringTask {
    pub id: Uuid,
    pub assigned_to: String,
    pub quest_name: String,
}
