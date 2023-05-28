use std::{sync::Arc, collections::HashMap};

use serenity::{prelude::TypeMapKey, model::prelude::GuildId};
use tokio::sync::Mutex;

pub struct Volume;

impl TypeMapKey for Volume {
    type Value = Arc<Mutex<HashMap<GuildId, f32>>>;
}