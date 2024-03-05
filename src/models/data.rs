use super::{guild::data::GuildData, playing::Playing};
use dashmap::DashMap;
use serenity::all::GuildId;
use tokio::sync::RwLock;
use std::{collections::HashMap, sync::Arc};

#[derive(Default)]
pub struct Data {
    pub guilds: Arc<DashMap<GuildId, GuildData>>,
    pub playing: Arc<RwLock<HashMap<GuildId, Playing>>>,
}
