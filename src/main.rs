mod config;
mod context;
mod shock;

use std::collections::HashMap;

use log::{debug, error, info, warn};
use pishock_rs::{
    errors::PiShockError::{ShockerOffline, ShockerPaused},
    PiShockAccount, PiShocker,
};
use ron::error::SpannedError;
use serenity::{
    all::{GatewayIntents, Message, Ready},
    async_trait,
    client::{Context, EventHandler},
    Client,
};
use shock::word_shock;
use thiserror::Error;

use crate::config::Config;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        word_shock(ctx, msg).await;
    }

    async fn ready(&self, _: Context, ready: Ready) {
        info!("{} is ready!", ready.user.name);
    }
}

fn log_panic(message: impl std::fmt::Display, error: impl std::fmt::Display) {
    error!("{message} : {error}");
    panic!("{message}\n{error}");
}

#[tokio::main]
async fn main() {
    // Initialize log and get config from file.
    initialize_log();

    let mut config: Option<Config> = None;
    match get_config().await {
        Ok(o) => config = Some(o),
        Err(e) => match e {
            GetConfigError::IO(io) => match io.kind() {
                std::io::ErrorKind::NotFound => log_panic(
                    "\"config.ron\" not found. It should be in the executable root directory.",
                    io,
                ),
                std::io::ErrorKind::PermissionDenied => {
                    log_panic("Permission to access \"config.ron\" denied.", io)
                }
                _ => panic!("{io}"),
            },
            GetConfigError::Spanned(spanned) => log_panic("Error parsing \"config.ron\".", spanned),
        },
    }
    let config = config.unwrap();

    debug!("{config:?}");

    // Get the shocker from the config.
    let shocker = get_shocker(&config).await;

    // Build gateway intents.
    let gateway_intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    // Build the client.
    let mut client = Client::builder(&config.discord_config.bot_token, gateway_intents)
        .event_handler(Handler)
        .await
        .expect("Error creating Discord client.");

    // Spawn a handler for the CTRL-C signal.
    {
        let shard_manager = client.shard_manager.clone();

        tokio::spawn(async move {
            tokio::signal::ctrl_c()
                .await
                .expect("Could not register ctrl+c handler.");
            info!("Shutting down all shards...");
            shard_manager.shutdown_all().await;
        });
    }

    // Initialize client data to be shared across command invocations and shards.
    {
        let mut data = client.data.write().await;
        data.insert::<context::Shocker>(shocker);
        data.insert::<context::Config>(config);
        data.insert::<context::UserShockCooldowns>(HashMap::new());
    }

    // Start the client.
    if let Err(e) = client.start().await {
        println!("Client start error: {e:?}");
    }
}

fn initialize_log() {
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{} {} {}] {}",
                chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.9f"),
                record.level(),
                record.target(),
                message
            ))
        })
        .level(log::LevelFilter::Warn)
        .level_for(env!("CARGO_PKG_NAME"), log::LevelFilter::Trace)
        .chain(std::io::stdout())
        .chain(fern::log_file("output.log").unwrap())
        .apply()
        .unwrap();

    info!(
        "STARTED {} {}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
    );
}

async fn get_shocker(config: &config::Config) -> PiShocker {
    debug!("Fetching PiShock account.");
    let account = PiShockAccount::new(
        config.pishock_config.api_name.clone(),
        config.pishock_config.api_username.clone(),
        config.pishock_config.api_key.clone(),
    );

    debug!("Fetching PiShocker.");
    loop {
        match account
            .get_shocker(config.pishock_config.share_code.clone())
            .await
        {
            Ok(o) => break o,
            Err(e) => match e {
                ShockerPaused | ShockerOffline => {
                    warn!("Retrying shocker connection...: {e}");
                    tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                    continue;
                }
                _ => panic!("Unrecoverable error accessing PiShock shocker."),
            },
        }
    }
}

#[derive(Debug, Error)]
enum GetConfigError {
    #[error("IO error.")]
    IO(#[from] std::io::Error),
    #[error("Spanned error.")]
    Spanned(#[from] SpannedError),
}

async fn get_config() -> Result<config::Config, GetConfigError> {
    Ok(ron::from_str::<config::Config>(
        &tokio::fs::read_to_string("config.ron").await?,
    )?)
}
