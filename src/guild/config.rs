use std::{sync::Arc, collections::HashMap};

use serenity::{prelude::TypeMapKey, model::prelude::GuildId};
use tokio::sync::Mutex;

use crate::models::guild::config::GuildConfig;

pub struct GuildConfigs;

impl TypeMapKey for GuildConfigs {
    type Value = Arc<Mutex<HashMap<GuildId, GuildConfig>>>;
}