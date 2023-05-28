use serenity::{prelude::Context, model::prelude::GuildId, async_trait};
use songbird::events::{
    Event,
    EventContext,
    EventHandler
};

use crate::utils::play_next;

pub struct PlayNextSong {
    pub ctx: Context,
    pub guild_id: GuildId,
}

#[async_trait]
impl EventHandler for PlayNextSong {
    async fn act(&self, _ctx: &EventContext<'_>) -> Option<Event> {
        let _ = play_next(&self.ctx, self.guild_id).await;
        None
    }
}