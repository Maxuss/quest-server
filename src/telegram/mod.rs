use teloxide::{
    dispatching::{dialogue::InMemStorage, UpdateHandler},
    prelude::*,
    utils::command::BotCommands,
};
use uuid::Uuid;

use crate::common::mongo::MongoDatabase;

use self::player::player_schema;

mod player;
mod spectator;

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Список комманд бота:")]
pub enum Command {
    #[command(description = "Показывает это сообщение")]
    Help,
    #[command(description = "Показывает основную информацию про этого бота.")]
    Start,
    #[command(description = "Начинает процесс регистрации. Берет токен регистрации как аргумент.")]
    Register(String),
    #[command(description = "Отменяет процесс регистрации")]
    Cancel,
    #[command(description = "???")]
    Acknowledge { quest_id: Uuid },
    #[command(description = "???", parse_with = "split")]
    CreateQuest { name: String, assign_to: String },
}

#[allow(unused_variables)]
#[tracing::instrument(skip_all)]
pub async fn start_telegram(token: String, db: MongoDatabase) -> anyhow::Result<()> {
    tracing::info!("Starting telegram bot");

    let bot = Bot::new(token);

    Dispatcher::builder(bot, schema())
        .dependencies(dptree::deps![
            InMemStorage::<player::RegisterDialogueState>::new(),
            db
        ])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
    Ok(())
}

fn schema() -> UpdateHandler<anyhow::Error> {
    let player_side_schema: UpdateHandler<anyhow::Error> = player_schema();
    // let spectator_side_schema: UpdateHandler<anyhow::Error> = spectator_schema();
    player_side_schema
}
