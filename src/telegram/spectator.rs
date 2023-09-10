use mongodb::bson::{doc, Bson};

use teloxide::prelude::*;
use teloxide::types::ParseMode;
use tracing::debug;
use uuid::Uuid;

use crate::common::{
    data::{BsonId, LingeringTask, User},
    mongo::MongoDatabase,
};

#[tracing::instrument(skip(bot, msg, db))]
pub async fn acknowledge(
    bot: Bot,
    msg: Message,
    db: MongoDatabase,
    quest_id: Uuid,
) -> anyhow::Result<()> {
    let quest = db.tasks.find_one(doc! { "id": &quest_id }, None).await?;

    // TODO: actually do something with the acquired lingering quest
    let _quest = if let Some(quest) = quest {
        quest
    } else {
        bot.send_message(msg.chat.id, "Не удалось найти активный квест с таким ID!")
            .await?;
        return Ok(());
    };

    debug!("Acknowledging quest {quest_id}!");

    db.tasks.delete_many(doc! { "id": &quest_id }, None).await?;

    bot.send_message(msg.chat.id, format!("Успешно завершили квест {quest_id}!"))
        .await?;

    Ok(())
}

#[tracing::instrument(skip(bot, msg, db))]
pub async fn create_quest(
    bot: Bot,
    msg: Message,
    db: MongoDatabase,
    (name, assign_to): (String, String),
) -> anyhow::Result<()> {
    let user = db
        .users
        .find_one(doc! { "username": &assign_to }, None)
        .await?;

    let user = if let Some(user) = user {
        user
    } else {
        bot.send_message(msg.chat.id, "Не удалось найти пользователя с таким ником!")
            .await?;
        return Ok(());
    };

    debug!("Assigning the `{name}` quest to player `{assign_to}`!");

    let task_id = BsonId::new();

    db.tasks
        .insert_one(
            LingeringTask {
                id: task_id,
                assigned_to: user.card_hash,
                quest_name: name.clone(),
            },
            None,
        )
        .await?;

    bot.send_message(
        msg.chat.id,
        format!(
            r#"Поручили новый квест {name} игроку {assign_to}
ID активного квеста: `{task_id}`
Используйте `/acknowledge {task_id}` чтобы дать пользователю награду за квест
"#
        ),
    )
    .parse_mode(ParseMode::MarkdownV2)
    .await?;

    Ok(())
}
