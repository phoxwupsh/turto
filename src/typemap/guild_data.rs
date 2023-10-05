use std::{sync::Arc, collections::HashMap};

use serenity::{prelude::TypeMapKey, model::prelude::GuildId};
use tokio::sync::Mutex;

use crate::models::guild::data::GuildData;

pub struct GuildDataMap;

impl TypeMapKey for GuildDataMap {
    type Value = Arc<Mutex<HashMap<GuildId, GuildData>>>;
}