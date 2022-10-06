use sqlx::PgPool;
use teloxide::{
    dispatching::{
        dialogue::{self, InMemStorage},
        UpdateHandler,
    },
    net::Download,
    prelude::*,
    types::{InlineKeyboardButton, InlineKeyboardMarkup, InputFile, ParseMode},
};
use tracing::{info, warn};

use crate::common::{data::RegStageUser, fs::create};

use super::Command;

use teloxide::dptree::case;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub enum RegisterDialogueState {
    StartRegister,
    GetUsername {
        id: Uuid,
        card_hash: String,
    },
    GetAvatar {
        username: String,
        id: Uuid,
        card_hash: String,
    },
}

impl Default for RegisterDialogueState {
    fn default() -> Self {
        Self::StartRegister
    }
}

type RegisterDialogue = Dialogue<RegisterDialogueState, InMemStorage<RegisterDialogueState>>;

pub fn player_schema() -> UpdateHandler<anyhow::Error> {
    let command_handler = teloxide::filter_command::<Command, _>()
        .branch(
            case![RegisterDialogueState::StartRegister]
                .branch(case![Command::Start].endpoint(start))
                .branch(case![Command::Register(token)].endpoint(register)),
        )
        .branch(case![Command::Help].endpoint(help))
        .branch(case![Command::Cancel].endpoint(cancel))
        .branch(
            case![crate::telegram::Command::CreateQuest { name, assign_to }]
                .endpoint(super::spectator::create_quest),
        )
        .branch(case![Command::Acknowledge { quest_id }].endpoint(super::spectator::acknowledge));

    let message_handler = Update::filter_message()
        .branch(command_handler)
        .branch(case![RegisterDialogueState::GetUsername { id, card_hash }].endpoint(get_username))
        .branch(
            case![RegisterDialogueState::GetAvatar {
                username,
                id,
                card_hash
            }]
            .endpoint(get_avatar),
        );

    let callback_query_handler = Update::filter_callback_query().branch(
        case![RegisterDialogueState::GetAvatar {
            username,
            id,
            card_hash
        }]
        .endpoint(get_avatar_callback),
    );

    dialogue::enter::<Update, InMemStorage<RegisterDialogueState>, RegisterDialogueState, _>()
        .branch(message_handler)
        .branch(callback_query_handler)
}

async fn help(bot: AutoSend<Bot>, msg: Message) -> anyhow::Result<()> {
    bot.send_message(
        msg.chat.id,
        r#"
/help - Показывает это сообщение.
/start - Основная информация о боте.
<code>/register &lt;token&gt;</code> - Начинает процесс регистрации. Замените <code>&lt;token&gt;</code> на ваш токен регистрации.
/cancel - Отменяет процесс регистрации.
"#,
    )
    .parse_mode(ParseMode::Html)
    .await?;
    Ok(())
}

async fn start(bot: AutoSend<Bot>, msg: Message) -> anyhow::Result<()> {
    bot.send_message(
        msg.chat.id,
        r#"Этот бот позволяет вам регистрироваться на квест\.
Начните процесс регистрации командой `/register <токен>`,
заменив `<токен>`на ваш токен регистрации\."#,
    )
    .parse_mode(ParseMode::MarkdownV2)
    .await?;

    Ok(())
}

#[tracing::instrument(skip(bot, msg, dialogue, pool))]
pub async fn register(
    bot: AutoSend<Bot>,
    msg: Message,
    dialogue: RegisterDialogue,
    token: String,
    pool: PgPool,
) -> anyhow::Result<()> {
    info!("Performing registration for user!");
    if token.len() != 8 {
        bot.send_message(msg.chat.id, "Неверный токен регистрации!")
            .await?;
        return Ok(());
    }
    let user = sqlx::query_as::<_, RegStageUser>(
        "SELECT * FROM users_reg_state WHERE starts_with(card_hash, $1)",
    )
    .bind(&token)
    .fetch_optional(&pool)
    .await?;
    let stage = if let Some(stage) = user {
        stage
    } else {
        bot.send_message(msg.chat.id, "Неверный токен для регистрации!")
            .await?;
        return Ok(());
    };

    bot.send_message(msg.chat.id, "Вы начинаете регистрацию на квест.")
        .await?;

    let deleted = sqlx::query("DELETE FROM users_reg_state WHERE starts_with(card_hash, $1)")
        .bind(&stage.card_hash)
        .execute(&pool)
        .await?;

    if deleted.rows_affected() != 1 {
        warn!(
            "Invalid amount of rows affected for delete operation, expected 1 but got {}",
            deleted.rows_affected()
        )
    }

    bot.send_message(msg.chat.id, "Введите предпочитаемый ник.")
        .await?;

    dialogue
        .update(RegisterDialogueState::GetUsername {
            id: stage.id,
            card_hash: stage.card_hash,
        })
        .await?;

    Ok(())
}

#[tracing::instrument(skip(bot, msg, dialogue, pool))]
async fn get_username(
    bot: AutoSend<Bot>,
    msg: Message,
    dialogue: RegisterDialogue,
    pool: PgPool,
    (id, card_hash): (Uuid, String),
) -> anyhow::Result<()> {
    match msg.text().map(ToOwned::to_owned) {
        Some(username) => {
            if is_username_used(&pool, &username).await? {
                bot
                    .send_message(
                        dialogue.chat_id(),
                        format!("Пользователь с ником `{username}` уже существует\\!\nПожалуйста, выберите другой ник\\.")
                    )
                    .parse_mode(ParseMode::MarkdownV2).await?;
                return Ok(());
            }
            bot.send_message(
                dialogue.chat_id(),
                format!("Вы выбрали использовать ник {username}"),
            )
            .await?;
            bot.send_message(msg.chat.id, "Теперь отправьте изображение для вашего профиля (или нажмите на кнопку ниже чтобы использовать ваше текущее изображение профиля).")
            .reply_markup(make_avatar_keyboard(&msg))
            .await?;

            dialogue
                .update(RegisterDialogueState::GetAvatar {
                    username,
                    id,
                    card_hash,
                })
                .await?;
        }
        None => {
            bot.send_message(msg.chat.id, "Введите ваш ник.").await?;
        }
    }
    Ok(())
}

fn make_avatar_keyboard(msg: &Message) -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![vec![InlineKeyboardButton::callback(
        "Использовать фото профиля из телеграма",
        msg.chat.id.to_string(),
    )]])
}

#[tracing::instrument(skip(bot, msg, dialogue, pool))]
async fn get_avatar(
    bot: AutoSend<Bot>,
    msg: Message,
    dialogue: RegisterDialogue,
    pool: PgPool,
    (username, id, card_hash): (String, Uuid, String),
) -> anyhow::Result<()> {
    match msg.photo().map(ToOwned::to_owned) {
        Some(photo) => {
            let photo = photo
                .first()
                .ok_or_else(|| anyhow::Error::msg("Image not provided!"))?;
            let file = bot.get_file(&photo.file_id).await?;
            let mut out = create(format!("data/image/{card_hash}.png")).await?;
            bot.download_file(&file.file_path, &mut out).await?;

            finish_registration(bot, msg.chat.id, pool, username, id, card_hash).await?;

            dialogue.exit().await?;
        }
        None => {
            bot.send_message(msg.chat.id, "Отправьте фото или нажмите на кнопку ниже!")
                .reply_markup(make_avatar_keyboard(&msg))
                .await?;
        }
    }
    Ok(())
}

#[tracing::instrument(skip(bot, q, dialogue, pool))]
async fn get_avatar_callback(
    bot: AutoSend<Bot>,
    q: CallbackQuery,
    dialogue: RegisterDialogue,
    pool: PgPool,
    (username, id, card_hash): (String, Uuid, String),
) -> anyhow::Result<()> {
    if let Some(_) = &q.data {
        let chat_id = dialogue.chat_id();
        let chat = bot.get_chat(chat_id).await?;
        let photo = if let Some(photo) = chat.photo {
            photo
        } else {
            warn!("User has no profile picture!");
            bot.send_message(chat_id, "У вас нет изображения профиля!")
                .await?;
            return Ok(());
        };
        let file = bot.get_file(photo.small_file_id).await?;
        let mut out = create(format!("data/image/{card_hash}.png")).await?;
        bot.download_file(&file.file_path, &mut out).await?;

        finish_registration(bot, chat_id, pool, username, id, card_hash).await?;

        dialogue.exit().await?;
    }
    Ok(())
}

#[tracing::instrument(skip(bot, id, pool))]
async fn finish_registration(
    bot: AutoSend<Bot>,
    id: ChatId,
    pool: PgPool,
    username: String,
    uuid: Uuid,
    card_hash: String,
) -> anyhow::Result<()> {
    let rows = sqlx::query("INSERT INTO users VALUES($1, $2, $3, $4)")
        .bind(&card_hash)
        .bind(&uuid)
        .bind(&username)
        .bind(id.0)
        .execute(&pool)
        .await?;
    if rows.rows_affected() < 1 {
        bot.send_message(id, "Не удалось провести регистрацию!")
            .await?;
        return Ok(());
    }

    bot.send_photo(id, InputFile::file(format!("data/image/{card_hash}.png")))
        .await?;
    bot.send_message(
        id,
        format!("Регистрация проведена успешно!\nИмя пользователя: {username}"),
    )
    .await?;

    Ok(())
}

#[tracing::instrument(skip_all)]
async fn cancel(
    bot: AutoSend<Bot>,
    msg: Message,
    dialogue: RegisterDialogue,
) -> anyhow::Result<()> {
    tracing::debug!("Registration cancelled!");
    bot.send_message(msg.chat.id, "Регистрация отменена.")
        .await?;
    dialogue.exit().await?;
    Ok(())
}

async fn is_username_used(pool: &PgPool, username: &String) -> anyhow::Result<bool> {
    if sqlx::query("SELECT * FROM users WHERE username = $1")
        .bind(username)
        .fetch_optional(pool)
        .await?
        .is_some()
    {
        Ok(true)
    } else {
        Ok(false)
    }
}
