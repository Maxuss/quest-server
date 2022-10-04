use std::path::Path;

use anyhow::bail;
use serde::{Deserialize, Serialize};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{ConnectOptions, PgPool};
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::join;
use tokio::task::JoinHandle;
use tracing::info;
use tracing_appender::rolling;
use tracing_subscriber::prelude::*;

use crate::api::start_api;
use crate::telegram::start_telegram;

pub mod api;
pub mod telegram;

fn prepare_logging() -> anyhow::Result<()> {
    let appender = rolling::minutely("logs/", "latest.log");

    let stdout_log = tracing_subscriber::fmt::layer().pretty();
    let file_log = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_writer(appender);

    tracing_subscriber::registry()
        .with(
            stdout_log
                .with_filter(tracing_subscriber::filter::LevelFilter::DEBUG)
                .and_then(file_log)
                .with_filter(tracing_subscriber::filter::filter_fn(|metadata| {
                    metadata
                        .module_path()
                        .unwrap_or("unknown")
                        .starts_with("quest_server")
                })),
        )
        .init();

    Ok(())
}

#[tracing::instrument]
async fn prepare_config() -> anyhow::Result<ServerConfig> {
    if !Path::new("config.toml").exists() {
        let mut file = File::create("config.toml").await?;
        let cfg = ServerConfig::default();
        file.write_all(toml::to_string_pretty(&cfg)?.as_bytes())
            .await?;

        info!("Config file was generated, make sure to fill it out!");
        bail!("Config does not exist!")
    }

    let mut cfg = File::open("config.toml").await?;
    let mut buf = String::new();
    cfg.read_to_string(&mut buf).await?;

    let cfg: ServerConfig = toml::from_str(&buf)?;
    Ok(cfg)
}

#[tracing::instrument]
async fn prepare_db(cfg: &ServerConfig) -> anyhow::Result<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect_with(
            PgConnectOptions::new()
                .host(&cfg.postgres.host)
                .username(&cfg.postgres.username)
                .password(&cfg.postgres.password)
                .database(&cfg.postgres.database)
                .log_statements(tracing::log::LevelFilter::Off)
                .to_owned(),
        )
        .await?;
    Ok(pool)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    prepare_logging()?;

    info!("Initializing the Cardquest backend...");

    let cfg = if let Ok(cfg) = prepare_config().await {
        cfg
    } else {
        return Ok(());
    };

    let db = prepare_db(&cfg).await?;

    let db_clone = db.clone();
    let tg_key = cfg.telegram.api_key.clone();

    let telegram_handle: JoinHandle<anyhow::Result<()>> =
        tokio::task::spawn(async move { start_telegram(tg_key, db_clone).await });
    let api_handle = tokio::task::spawn(async move { start_api(&cfg, db).await });

    let (tg, api) = join!(telegram_handle, api_handle);

    tg??;
    api??;

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    api: ApiConfig,
    telegram: TelegramConfig,
    postgres: PostgresConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    host: String,
    port: u64,
    record_dev_data: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramConfig {
    api_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostgresConfig {
    database: String,
    host: String,
    username: String,
    password: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        ServerConfig {
            api: ApiConfig {
                host: "127.0.0.1".to_string(),
                port: 4040,
                record_dev_data: true,
            },
            telegram: TelegramConfig {
                api_key: "<ENTER KEY HERE>".to_string(),
            },
            postgres: PostgresConfig {
                database: "cardquest".to_string(),
                host: "localhost".to_string(),
                username: "<USERNAME>".to_string(),
                password: "<PASSWORD>".to_string(),
            },
        }
    }
}
