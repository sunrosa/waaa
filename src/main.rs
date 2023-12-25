use std::{env, fs::OpenOptions, time::Duration};

use log::{error, info};
use pishock_rs::{PiShockAccount, PiShocker};
use serenity::{
    all::{GatewayIntents, Message, Ready, UserId},
    async_trait,
    client::{Context, EventHandler},
    Client,
};

struct Handler {
    shocker: PiShocker,
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        let message_words: Vec<String> = msg
            .content
            .split(' ')
            .map(|s| s.chars().filter(|c| c.is_alphabetic()).collect::<String>())
            .collect();
        let trigger_words: Vec<String> = env::var("TRIGGER_WORDS")
            .expect("Could not access TRIGGER_WORDS.")
            .split('/')
            .map(str::to_string)
            .collect();

        let do_shock = 'do_shock: {
            // If the message mentions the bot's owner, set do_shock to true.
            if msg.mentions_user_id(UserId::new(
                env::var("OWNER_USER_ID")
                    .expect("Could not access OWNER_USER_ID.")
                    .parse()
                    .expect("Could not parse OWNER_USER_ID into u64."),
            )) {
                break 'do_shock true;
            }

            // If any of the words in the message are a trigger word, set do_shock to true.
            for word in message_words {
                let word_lowercase = word.to_lowercase();
                if trigger_words.iter().any(|x| *x == word_lowercase) {
                    break 'do_shock true;
                }
            }

            // If no predicates match...
            false
        };

        if do_shock {
            self.shocker
                .shock(40, Duration::from_secs(1))
                .await
                .unwrap();
        }
    }

    async fn ready(&self, _: Context, ready: Ready) {
        info!("{} is ready!", ready.user.name);
    }
}

#[tokio::main]
async fn main() {
    initialize_log();
    initialize_env();

    let shocker = get_shocker().await;

    let discord_token = env::var("DISCORD_TOKEN")
        .expect("Could not access Discord token in environment variables.");
    let gateway_intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    let mut client = Client::builder(&discord_token, gateway_intents)
        .event_handler(Handler { shocker })
        .await
        .expect("Error creating Discord client.");

    if let Err(e) = client.start().await {
        println!("Client start error: {e:?}");
    }
}

fn initialize_log() {
    #[cfg(not(debug_assertions))]
    let log_level = simplelog::LevelFilter::Info;

    #[cfg(debug_assertions)]
    let log_level = simplelog::LevelFilter::Trace;

    simplelog::WriteLogger::init(
        log_level,
        simplelog::ConfigBuilder::new()
            .set_time_format_rfc3339()
            .build(),
        OpenOptions::new()
            .append(true)
            .create(true)
            .open("log")
            .expect("Error accessing or creating log file!"),
    )
    .expect("Error initializing logger!");
    info!(
        "STARTED {} {}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
    );
}

fn initialize_env() {
    // Read .env vars and check to see if the file exists.
    info!("Reading .env vars from file...");
    let dotenv_result = dotenvy::dotenv();
    if let Err(e) = dotenv_result {
        if let dotenvy::Error::Io(e) = e {
            if e.kind() == std::io::ErrorKind::NotFound {
                error!(".env file not found! Please create it in the crate root (where Cargo.toml is found).");
                panic!(".env file not found! Please create it in the crate root (where Cargo.toml is found).");
            }
        } else {
            error!("Error reading .env file: {:?}", e);
            panic!("Error reading .env file: {:?}", e);
        }
    }
}

async fn get_shocker() -> PiShocker {
    info!("Fetching account from information stored in environment variables...");
    let account = PiShockAccount::new(
        std::env::var("API_NAME").expect("Could not read API_NAME from .env file."),
        std::env::var("API_USERNAME").expect("Could not read API_USERNAME from .env file."),
        std::env::var("API_KEY").expect("Could not read API_KEY from .env file."),
    );

    info!("Fetching shocker from information stored in environment variables...");
    account
        .get_shocker(
            std::env::var("SHARE_CODE").expect("Could not read SHARE_CODE from .env file."),
        )
        .await
        .expect("Could not access the shocker tied to the account configured in the environment variables!")
}
