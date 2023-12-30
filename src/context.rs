use std::collections::HashMap;

use pishock_rs::PiShocker;
use serenity::{all::UserId, prelude::*};

use crate::{config, ShockCooldown};

pub struct Shocker;
impl TypeMapKey for Shocker {
    type Value = PiShocker;
}

pub struct Config;
impl TypeMapKey for Config {
    type Value = config::Config;
}

pub struct UserShockCooldowns;
impl TypeMapKey for UserShockCooldowns {
    type Value = HashMap<UserId, ShockCooldown>;
}
