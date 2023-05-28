use std::{sync::Arc, collections::{HashMap, VecDeque}};

use serenity::{prelude::TypeMapKey, model::prelude::GuildId};
use songbird::input::Metadata;
use tokio::sync::Mutex;

pub type Playlist = VecDeque<Metadata>;

pub struct Playlists;

impl TypeMapKey for Playlists {
    type Value = Arc<Mutex<HashMap<GuildId, Playlist>>>;
}