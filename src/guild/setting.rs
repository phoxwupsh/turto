use std::{sync::Arc, collections::HashMap};

use serenity::{prelude::TypeMapKey, model::prelude::GuildId};
use tokio::sync::Mutex;

use crate::models::guild_setting::GuildSetting;

pub struct GuildSettings;

impl TypeMapKey for GuildSettings {
    type Value = Arc<Mutex<HashMap<GuildId, GuildSetting>>>;
}