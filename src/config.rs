use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct Config {
    pub pishock_config: PishockConfig,
    pub discord_config: DiscordConfig,
    pub trigger_words: Vec<String>,
    pub cooldown_segment_duration: u32,
    pub max_shocks_per_segment: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct PishockConfig {
    pub api_name: String,
    pub api_username: String,
    pub api_key: String,
    pub share_code: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct DiscordConfig {
    pub bot_token: String,
    pub operator_ids: Vec<u64>,
}
