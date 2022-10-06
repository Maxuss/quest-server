use sqlx::PgPool;
use teloxide::{
    dispatching::{dialogue::InMemStorage, UpdateHandler},
    prelude::*,
    utils::command::BotCommands,
};
use uuid::Uuid;

use self::player::player_schema;

mod player;
mod spectator;

#[derive(BotCommands, Clone)]
#[command(
    rename = "lowercase",
    description = "Список комманд бота:",
    parse_with = "split"
)]
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
    #[command(description = "???")]
    CreateQuest { name: String, assign_to: String },
}

#[allow(unused_variables)]
#[tracing::instrument(skip_all)]
pub async fn start_telegram(token: String, pool: PgPool) -> anyhow::Result<()> {
    tracing::info!("Starting telegram bot");

    let bot = Bot::new(token).auto_send();

    Dispatcher::builder(bot, schema())
        .dependencies(dptree::deps![
            InMemStorage::<player::RegisterDialogueState>::new(),
            pool
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
