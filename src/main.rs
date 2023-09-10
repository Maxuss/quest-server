use std::path::Path;

use anyhow::bail;
use mongodb::options::ClientOptions;
use mongodb::{Client, Database};
use serde::{Deserialize, Serialize};
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::join;
use tokio::task::JoinHandle;
use tracing::info;
use tracing_subscriber::prelude::*;

use crate::api::start_api;

use crate::common::mongo::MongoDatabase;
use crate::telegram::start_telegram;

pub mod api;
pub mod common;
pub mod telegram;

fn prepare_logging() -> anyhow::Result<()> {
    let stdout_log = tracing_subscriber::fmt::layer().compact();
    tracing_subscriber::registry()
        .with(
            stdout_log
                .with_filter(tracing_subscriber::filter::LevelFilter::DEBUG)
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
async fn prepare_db(cfg: &ServerConfig) -> anyhow::Result<Database> {
    let uri = format!(
        "mongodb://{}:{}@{}:27017/{}?retryWrites=true",
        cfg.mongo.username, cfg.mongo.password, cfg.mongo.host, cfg.mongo.database
    );

    let mut opts = ClientOptions::parse(uri).await?;
    opts.app_name = Some(String::from("card-quest"));

    let client = Client::with_options(opts)?;
    let db = client.database("quest");

    info!("Connected to MongoDB successfully!");

    Ok(db)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    prepare_logging()?;

    info!("Initializing the Cardquest backend...");

    // prepare_fs().await?;

    let cfg = if let Ok(cfg) = prepare_config().await {
        cfg
    } else {
        return Ok(());
    };

    let db = MongoDatabase::new(prepare_db(&cfg).await?);

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
    mongo: MongoConfig,
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
pub struct MongoConfig {
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
            mongo: MongoConfig {
                database: "quest".to_string(),
                host: "localhost".to_string(),
                username: "<USERNAME>".to_string(),
                password: "<PASSWORD>".to_string(),
            },
        }
    }
}
