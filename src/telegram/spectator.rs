use sqlx::PgPool;

use teloxide::prelude::*;
use teloxide::types::ParseMode;
use tracing::debug;
use uuid::Uuid;

use crate::common::data::{LingeringTask, User};

#[tracing::instrument(skip(bot, msg, pool))]
pub async fn acknowledge(
    bot: Bot,
    msg: Message,
    pool: PgPool,
    quest_id: Uuid,
) -> anyhow::Result<()> {
    let quest = sqlx::query_as::<_, LingeringTask>("SELECT * FROM lingering_quests WHERE id = $1")
        .bind(quest_id)
        .fetch_optional(&pool)
        .await?;

    // todo: actually do something with the acquired lingering quest
    let _quest = if let Some(quest) = quest {
        quest
    } else {
        bot.send_message(msg.chat.id, "Не удалось найти активный квест с таким ID!")
            .await?;
        return Ok(());
    };

    debug!("Acknowledging quest {quest_id}!");

    sqlx::query("DELETE FROM lingering_quests WHERE id = $1")
        .bind(quest_id)
        .execute(&pool)
        .await?;

    bot.send_message(msg.chat.id, format!("Успешно завершили квест {quest_id}!"))
        .await?;

    Ok(())
}

#[tracing::instrument(skip(bot, msg, pool))]
pub async fn create_quest(
    bot: Bot,
    msg: Message,
    pool: PgPool,
    (name, assign_to): (String, String),
) -> anyhow::Result<()> {
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = $1")
        .bind(&assign_to)
        .fetch_optional(&pool)
        .await?;

    let user = if let Some(user) = user {
        user
    } else {
        bot.send_message(msg.chat.id, "Не удалось найти пользователя с таким ником!")
            .await?;
        return Ok(());
    };

    debug!("Assigning the `{name}` quest to player `{assign_to}`!");

    let task_id = Uuid::new_v4();

    sqlx::query("INSERT INTO lingering_quests VALUES($1, $2, $3)")
        .bind(task_id)
        .bind(user.card_hash)
        .bind(&name)
        .execute(&pool)
        .await
        .unwrap();

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
