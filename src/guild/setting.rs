use std::{sync::Arc, collections::HashMap};

use serenity::{prelude::TypeMapKey, model::prelude::GuildId};
use tokio::sync::Mutex;

use crate::models::setting::GuildSetting;

pub struct Settings;

impl TypeMapKey for Settings {
    type Value = Arc<Mutex<HashMap<GuildId, GuildSetting>>>;
}