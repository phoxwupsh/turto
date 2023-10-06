use crate::models::guild::data::GuildData;
use dashmap::DashMap;
use serenity::{model::prelude::GuildId, prelude::TypeMapKey};
use std::sync::Arc;

pub struct GuildDataMap;

impl TypeMapKey for GuildDataMap {
    type Value = Arc<DashMap<GuildId, GuildData>>;
}
