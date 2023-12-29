use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub pishock_config: PishockConfig,
    pub discord_config: DiscordConfig,
    pub trigger_words: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PishockConfig {
    pub api_name: String,
    pub api_username: String,
    pub api_key: String,
    pub share_code: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DiscordConfig {
    pub bot_token: String,
    pub operator_ids: Vec<u64>,
}
