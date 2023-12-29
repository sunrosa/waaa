mod config;

use std::{env, time::Duration};

use log::{debug, error, info, trace};
use pishock_rs::{PiShockAccount, PiShocker};
use regex::Regex;
use serenity::{
    all::{GatewayIntents, Message, Ready, UserId},
    async_trait,
    client::{Context, EventHandler},
    Client,
};

struct Handler {
    shocker: PiShocker,
    config: config::Config,
}

impl Handler {
    async fn word_shock(&self, ctx: Context, msg: Message) {
        let split_sentence = Regex::new(r"(\b[^\s]+\b)").unwrap();

        let message_words: Vec<String> = split_sentence
            .captures_iter(&msg.content)
            .map(|x| x.get(0).unwrap().as_str().to_owned())
            .collect();

        trace!(
            "Message text: \"{}\" -> Split into: \"{}\"",
            msg.content,
            message_words.join(", ")
        );

        let do_shock = 'do_shock: {
            // If the message mentions any of the bot's operators, set do_shock to true.
            if self
                .config
                .discord_config
                .operator_ids
                .iter()
                .any(|x| msg.mentions_user_id(<u64 as Into<UserId>>::into(*x)))
            {
                trace!("Message mentions bot owner. Shock impending...");
                break 'do_shock true;
            }

            // If any of the words in the message are a trigger word, set do_shock to true.
            for word in message_words {
                let word_lowercase = word.to_lowercase();
                if self
                    .config
                    .trigger_words
                    .iter()
                    .any(|x| *x == word_lowercase)
                {
                    trace!("Caught trigger word. Shock impending...");
                    break 'do_shock true;
                }
            }

            // If no predicates match...
            trace!("Message does not match shock parameters.");
            false
        };

        if do_shock {
            let typing = msg.channel_id.start_typing(&ctx.http);

            info!("Shocking!");
            self.shocker
                .shock(40, Duration::from_secs(1))
                .await
                .unwrap();

            typing.stop();
        }
    }
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        self.word_shock(ctx, msg).await;
    }

    async fn ready(&self, _: Context, ready: Ready) {
        info!("{} is ready!", ready.user.name);
    }
}

#[tokio::main]
async fn main() {
    initialize_log();
    let config = get_config().await;

    debug!("{config:?}");

    let shocker = get_shocker(&config).await;

    let gateway_intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    let mut client = Client::builder(&config.discord_config.bot_token, gateway_intents)
        .event_handler(Handler { shocker, config })
        .await
        .expect("Error creating Discord client.");

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

    if let Err(e) = client.start().await {
        println!("Client start error: {e:?}");
    }
}

fn initialize_log() {
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{} {} {}] {}",
                chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"),
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
    info!("Fetching account from information stored in environment variables...");
    let account = PiShockAccount::new(
        config.pishock_config.api_name.clone(),
        config.pishock_config.api_username.clone(),
        config.pishock_config.api_key.clone(),
    );

    info!("Fetching shocker from information stored in environment variables...");
    account
        .get_shocker(
            config.pishock_config.share_code.clone(),
        )
        .await
        .expect("Could not access the shocker tied to the account configured in the environment variables!")
}

async fn get_config() -> config::Config {
    ron::from_str::<config::Config>(
        &tokio::fs::read_to_string("config.ron")
            .await
            .expect("Could not access config.ron."),
    )
    .expect("Could not parse config.ron.")
}
