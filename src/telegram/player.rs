use futures::{StreamExt, TryStreamExt};
use mongodb::bson::{doc, Bson};
use teloxide::{
    dispatching::{
        dialogue::{self, InMemStorage},
        UpdateHandler,
    },
    net::Download,
    prelude::*,
    types::{InlineKeyboardButton, InlineKeyboardMarkup, InputFile, ParseMode},
};
use tokio::io::AsyncWriteExt;
use tracing::{info, warn};

use crate::common::{
    data::{BsonId, RegStageUser, User},
    fs::create,
    mongo::MongoDatabase,
};

use super::Command;

use teloxide::dptree::case;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub enum RegisterDialogueState {
    StartRegister,
    GetUsername {
        id: BsonId,
        card_hash: String,
    },
    GetAvatar {
        username: String,
        id: BsonId,
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

async fn help(bot: Bot, msg: Message) -> anyhow::Result<()> {
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

async fn start(bot: Bot, msg: Message) -> anyhow::Result<()> {
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

#[tracing::instrument(skip(bot, msg, dialogue, db))]
pub async fn register(
    bot: Bot,
    msg: Message,
    dialogue: RegisterDialogue,
    token: String,
    db: MongoDatabase,
) -> anyhow::Result<()> {
    info!("Performing registration for user!");
    if token.len() != 8 {
        bot.send_message(msg.chat.id, "Неверный токен регистрации!")
            .await?;
        return Ok(());
    }
    let pattern = format!("^{token}");
    let user = db
        .reg_stage
        .find_one(doc! { "card_hash": { "$regex": &pattern } }, None)
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

    let deleted = db
        .reg_stage
        .delete_one(doc! { "card_hash": { "$regex": &pattern } }, None)
        .await?;

    if deleted.deleted_count != 1 {
        warn!(
            "Invalid amount of documents affected by delete operation, expected 1 but got {}",
            deleted.deleted_count
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

#[tracing::instrument(skip(bot, msg, dialogue, db))]
async fn get_username(
    bot: Bot,
    msg: Message,
    dialogue: RegisterDialogue,
    db: MongoDatabase,
    (id, card_hash): (BsonId, String),
) -> anyhow::Result<()> {
    match msg.text().map(ToOwned::to_owned) {
        Some(username) => {
            if is_username_used(&db, &username).await? {
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

#[tracing::instrument(skip(bot, msg, dialogue, db))]
async fn get_avatar(
    bot: Bot,
    msg: Message,
    dialogue: RegisterDialogue,
    db: MongoDatabase,
    (username, id, card_hash): (String, BsonId, String),
) -> anyhow::Result<()> {
    match msg.photo().map(ToOwned::to_owned) {
        Some(photo) => {
            let photo = photo
                .first()
                .ok_or_else(|| anyhow::Error::msg("Image not provided!"))?;
            let file = bot.get_file(&photo.file.id).await?;
            let mut out = create(format!("data/image/{card_hash}.png")).await?;
            bot.download_file(&file.path, &mut out).await?;

            finish_registration(bot, msg.chat.id, db, username, id, card_hash).await?;

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

#[tracing::instrument(skip(bot, q, dialogue, db))]
async fn get_avatar_callback(
    bot: Bot,
    q: CallbackQuery,
    dialogue: RegisterDialogue,
    db: MongoDatabase,
    (username, id, card_hash): (String, BsonId, String),
) -> anyhow::Result<()> {
    if q.data.is_some() {
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
        bot.send_message(chat_id, "Будет использовано фото профиля.")
            .await?;

        let stream = bot.download_file_stream(&file.path);
        let stream = stream.map(|it| std::io::Result::Ok(it.unwrap())).boxed();
        db.gridfs
            .upload_from_futures_0_3_reader(
                format!("{card_hash}.png"),
                stream.into_async_read(),
                None,
            )
            .await?;

        finish_registration(bot, chat_id, db, username, id, card_hash).await?;

        dialogue.exit().await?;
    }
    Ok(())
}

#[tracing::instrument(skip(bot, id, db))]
async fn finish_registration(
    bot: Bot,
    id: ChatId,
    db: MongoDatabase,
    username: String,
    uuid: BsonId,
    card_hash: String,
) -> anyhow::Result<()> {
    let new_user = User {
        card_hash,
        id: uuid,
        username: username.clone(),
        telegram_chat_id: id.0,
    };
    db.users.insert_one(new_user, None).await?;

    bot.send_message(
        id,
        format!("Регистрация проведена успешно!\nИмя пользователя: {username}"),
    )
    .await?;

    Ok(())
}

#[tracing::instrument(skip_all)]
async fn cancel(bot: Bot, msg: Message, dialogue: RegisterDialogue) -> anyhow::Result<()> {
    tracing::debug!("Registration cancelled!");
    bot.send_message(msg.chat.id, "Регистрация отменена.")
        .await?;
    dialogue.exit().await?;
    Ok(())
}

async fn is_username_used(db: &MongoDatabase, username: &str) -> anyhow::Result<bool> {
    let found = db
        .users
        .find_one(doc! { "username": username }, None)
        .await?;
    Ok(found.is_some())
}
