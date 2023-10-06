use serenity::{model::prelude::GuildId, prelude::TypeMapKey};
use songbird::tracks::TrackHandle;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;

pub struct Playing;

impl TypeMapKey for Playing {
    type Value = Arc<RwLock<HashMap<GuildId, TrackHandle>>>;
}
