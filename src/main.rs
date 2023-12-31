mod config;
mod context;

use std::{collections::HashMap, env, time::Duration};

use log::{debug, error, info, trace};
use pishock_rs::{PiShockAccount, PiShocker};
use regex::Regex;
use serenity::{
    all::{GatewayIntents, Message, Ready, UserId},
    async_trait,
    client::{Context, EventHandler},
    Client,
};

/// The number of shocks a user has dealt during the current span of time.
#[derive(Debug, Clone)]
struct ShockCooldown {
    /// The timestamp that the block started at.
    stopwatch: std::time::Instant,
    /// The number of times the user has dealt a shock during the last block.
    shock_count: u32,
}

impl ShockCooldown {
    /// Are there room for more shocks during the current segment? Returns true if the cooldown has room for the shock. Returns false if too many shocks have already been dealt.
    ///
    /// # Parameters
    /// * `segment_length` - The amount of time between segment resets.
    /// * `maximum_shocks` - The maximum number of shocks allowed before a segment reset.
    fn can_shock(&mut self, segment_length: std::time::Duration, maximum_shocks: u32) -> bool {
        // Reset the stopwatch and shock_count if the segment_length has been reached.
        if self.stopwatch.elapsed() >= segment_length {
            self.stopwatch = std::time::Instant::now();
            self.shock_count = 0;
        }

        // Check to see if the shock_count is below or equal to maximum. Return true if so, and increment shock_count.
        if self.shock_count < maximum_shocks {
            true
        } else {
            false
        }
    }
}

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

async fn word_shock(ctx: Context, msg: Message) {
    let mut data = ctx.data.write().await;
    let config = data.get::<context::Config>().unwrap().clone();
    let shocker = data.get::<context::Shocker>().unwrap().clone();

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

    let mut do_shock = 'do_shock: {
        // If the message mentions any of the bot's operators, set do_shock to true.
        if config
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
            if config.trigger_words.iter().any(|x| *x == word_lowercase) {
                trace!("Caught trigger word. Shock impending...");
                break 'do_shock true;
            }
        }

        // If no predicates match...
        trace!("Message does not match shock parameters.");
        false
    };

    {
        let user_shock_cooldowns = data.get_mut::<context::UserShockCooldowns>().unwrap();

        if do_shock
            && !user_shock_cooldowns
                .entry(msg.author.id)
                .or_insert(ShockCooldown {
                    stopwatch: std::time::Instant::now(),
                    shock_count: 0,
                })
                .can_shock(
                    std::time::Duration::from_secs(config.cooldown_segment_duration as u64),
                    config.max_shocks_per_segment,
                )
        {
            // The number of seconds until the segment counter is reset.
            let seconds_until_reset = config.cooldown_segment_duration as u64
                - user_shock_cooldowns
                    .get(&msg.author.id)
                    .unwrap()
                    .stopwatch
                    .elapsed()
                    .as_secs();

            debug!(
            "User has exceeded shock limit for the current segment {}/{} ({} seconds remaining)",
            user_shock_cooldowns
                .get(&msg.author.id)
                .expect(
                    format!(
                        "Could not access user shock cooldown for {}.",
                        msg.author.name
                    )
                    .as_str()
                )
                .shock_count,
            config.max_shocks_per_segment,
            seconds_until_reset,
        );
            msg.channel_id
                .say(
                    &ctx.http,
                    format!("Wait {} seconds...", seconds_until_reset),
                )
                .await
                .unwrap();
            do_shock = false;
        }
    }

    if do_shock {
        let user_shock_cooldowns = data.get_mut::<context::UserShockCooldowns>().unwrap();

        let typing = msg.channel_id.start_typing(&ctx.http);

        info!("Shocking!");
        shocker.shock(40, Duration::from_secs(1)).await.unwrap();

        typing.stop();

        // Update shock cooldown by adding 1 to shock_count.
        let mut shock_cooldown = user_shock_cooldowns.get(&msg.author.id).unwrap().clone();
        shock_cooldown.shock_count += 1;
        user_shock_cooldowns.insert(msg.author.id, shock_cooldown);
    }
}

#[tokio::main]
async fn main() {
    // Initialize log and get config from file.
    initialize_log();
    let config = get_config().await;

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
