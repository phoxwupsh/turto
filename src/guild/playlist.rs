use std::{
    collections::{HashMap},
    sync::Arc,
};

use serenity::{model::prelude::GuildId, prelude::TypeMapKey};
use tokio::sync::Mutex;

use crate::models::playlist::Playlist;

pub struct Playlists;

impl TypeMapKey for Playlists {
    type Value = Arc<Mutex<HashMap<GuildId, Playlist>>>;
}
