use crate::models::playing::Playing;
use serenity::{model::prelude::GuildId, prelude::TypeMapKey};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;

pub struct PlayingMap;

impl TypeMapKey for PlayingMap {
    type Value = Arc<RwLock<HashMap<GuildId, Playing>>>;
}
