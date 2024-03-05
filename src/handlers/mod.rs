use serenity::{
    all::GuildId,
    async_trait,
    model::{prelude::Ready, voice::VoiceState},
    prelude::{Context, EventHandler},
};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;
use tracing::info;

use crate::models::playing::Playing;

pub mod before;
pub mod track_end;

#[derive(Default)]
pub struct SerenityEventHandler {
    pub playing: Arc<RwLock<HashMap<GuildId, Playing>>>,
}

#[async_trait]
impl EventHandler for SerenityEventHandler {
    async fn ready(&self, _: Context, ready: Ready) {
        let name = &ready.user.name;
        let user_id = &ready.user.id;
        let session = &ready.session_id;
        info!(
            "{} is connected with user id {}, session id {}",
            name, user_id, session
        );
    }

    async fn voice_state_update(&self, ctx: Context, _: Option<VoiceState>, new: VoiceState) {
        // if the bot is manually disconnected by the user instead using command,
        // then remove the current track handle (if there is one)
        if new.user_id == ctx.cache.current_user().id {
            self.playing.write().await.remove(&new.guild_id.unwrap());
        }
    }
}
