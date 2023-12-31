use std::collections::HashMap;

use pishock_rs::PiShocker;
use serenity::{all::UserId, prelude::*};

use crate::{config, ShockCooldown};

pub(crate) struct Shocker;
impl TypeMapKey for Shocker {
    type Value = PiShocker;
}

pub(crate) struct Config;
impl TypeMapKey for Config {
    type Value = config::Config;
}

pub(crate) struct UserShockCooldowns;
impl TypeMapKey for UserShockCooldowns {
    type Value = HashMap<UserId, ShockCooldown>;
}
