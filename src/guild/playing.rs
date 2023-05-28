use std::{sync::Arc, collections::HashMap};

use serenity::{prelude::TypeMapKey, model::prelude::GuildId};
use songbird::tracks::TrackHandle;
use tokio::sync::RwLock;

pub struct Playing;

impl TypeMapKey for Playing {
    type Value = Arc<RwLock<HashMap<GuildId, TrackHandle>>>;
}