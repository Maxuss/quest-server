use log::LevelFilter;
use log4rs::append::console::ConsoleAppender;
use log4rs::append::rolling_file::policy::compound::roll::fixed_window::FixedWindowRoller;
use log4rs::append::rolling_file::policy::compound::trigger::size::SizeTrigger;
use log4rs::append::rolling_file::policy::compound::CompoundPolicy;
use log4rs::append::rolling_file::RollingFileAppender;
use log4rs::config::{Appender, Logger, Root};
use log4rs::encode::pattern::PatternEncoder;
use log4rs::{init_config, Config};
use serde::{Deserialize, Serialize};

fn prepare_logging() -> anyhow::Result<()> {
    let pattern = "[{d(%d-%m-%Y %H:%M:%S)}] {h(\\({l}\\))}: {m}\n";

    let stdout = ConsoleAppender::builder()
        .encoder(Box::new(PatternEncoder::new(pattern)))
        .build();

    let logfile = RollingFileAppender::builder()
        .encoder(Box::new(PatternEncoder::new(pattern)))
        .build(
            "logs/latest.log",
            Box::new(CompoundPolicy::new(
                // 4MB
                Box::new(SizeTrigger::new(4 * 1024)),
                Box::new(FixedWindowRoller::builder().build("logs/old/{}.log.gz", 4)?),
            )),
        )?;

    let config = Config::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .appender(Appender::builder().build("logfile", Box::new(logfile)))
        .logger(Logger::builder().build("cardquest", LevelFilter::Debug))
        .build(
            Root::builder()
                .appender("stdout")
                .appender("logfile")
                .build(LevelFilter::Info),
        )?;

    let _ = init_config(config)?;
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    prepare_logging()?;

    log::info!("Initializing Cardquest backend...");

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
