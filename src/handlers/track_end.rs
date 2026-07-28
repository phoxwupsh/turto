use crate::{player, player::PlayContext, ytdl::YouTubeDl};
use serenity::async_trait;
use songbird::{
    Call,
    events::{Event, EventContext, EventHandler},
    tracks::PlayMode,
};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{Instrument, info_span};

pub struct TrackEndHandler {
    pub call: Arc<Mutex<Call>>,
    pub ytdl_file: YouTubeDl,
    pub ctx: PlayContext,
}

#[async_trait]
impl EventHandler for TrackEndHandler {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        let EventContext::Track(ctx) = ctx else {
            return None;
        };
        let (state, _handle) = ctx[0];
        // Pattern match, not `==`: `PlayMode` compares by the `TrackEvent` it maps
        // to, and `Stop` maps to `End` too. Only a track that ended on its own
        // advances the queue -- a `call.stop()` (a skip, or the stop that precedes
        // the next track) must not.
        let PlayMode::End = state.playing else {
            return None;
        };

        // Hand off to the queue policy. Spawned rather than awaited: `act` runs
        // inline on songbird's single per-driver event task, so awaiting the next
        // track's open would stall every other event for this guild until it
        // finished. `parent: None` because that task is itself instrumented --
        // otherwise the contextual parent is songbird's `runner` span, which never
        // closes.
        let span = info_span!(parent: None, "track_end", guild = %self.ctx.guild_id);
        tokio::spawn(
            player::advance(self.ctx.clone(), self.call.clone(), self.ytdl_file.clone())
                .instrument(span),
        );
        None
    }
}
