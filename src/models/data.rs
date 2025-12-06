use super::{guild::Guilds, playing::Playing};
use crate::{
    message::template::Templates,
    models::{config::TurtoConfig, help::Help},
};
use serenity::all::GuildId;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;

#[derive(Debug)]
pub struct Data {
    pub guilds: Arc<Guilds>,
    pub playing: Arc<RwLock<HashMap<GuildId, Playing>>>,
    pub config: Arc<TurtoConfig>,
    pub help: Help,
    pub templates: Templates,
}
