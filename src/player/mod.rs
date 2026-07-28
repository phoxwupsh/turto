//! Trigger-agnostic playback operations.
//!
//! The domain layer between the entry points that *cause* work (slash commands,
//! songbird track events, the voice-state handler, cron jobs) and the `ytdl`
//! byte plumbing. Every operation here -- [`play_track`], [`advance`],
//! [`leave`] -- runs the same regardless of what triggered it, so the same track
//! is traced identically whether a user or an auto-advance started it. Each entry
//! point opens its own root span (`command`, `track_end`, `voice_update`, `job`);
//! the spans these functions open carry the domain identity (guild, url) beneath
//! it. (Queue prefetch is coupled to the mutation itself in
//! [`Playlist`](crate::models::playlist::Playlist).)

use crate::{
    handlers::{track_end::TrackEndHandler, track_error::TrackErrorHandler},
    models::{alias::Context, error::CommandError, guild::Guilds, playing::Playing},
    ytdl::{MetadataFuture, YouTubeDl, YouTubeDlError},
};
use serenity::all::GuildId;
use songbird::{Call, Event, TrackEvent, input::Input, tracks::Track};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{Mutex, RwLock};
use tracing::{Instrument, instrument};

/// Start streaming `ytdlfile` on `call`, returning a future that resolves to its
/// metadata -- awaited by the command paths to render the "now playing" reply,
/// dropped by the autonomous ones.
///
/// The one way to start a track, whatever the trigger. [`YouTubeDl::play`] serves
/// the prefetched tempfile when the queue already warmed it and a paced live
/// stream when it did not, so playback starts without waiting for the whole
/// track either way -- unlike [`YouTubeDl::fetch_file`], which blocks until the
/// download completes.
#[instrument(name = "play_track", skip_all, fields(guild = %ctx.guild_id, url = %ytdlfile.url()))]
pub async fn play_track(
    ctx: PlayContext,
    call: Arc<Mutex<Call>>,
    ytdlfile: YouTubeDl,
) -> Result<MetadataFuture, YouTubeDlError> {
    tracing::info!("streaming track");
    let (meta, input) = ytdlfile.play().await?;
    tokio::spawn(spawn_playback(ctx, call, input, ytdlfile).in_current_span());

    Ok(meta)
}

/// Wire an opened `input` into songbird: stop whatever is playing, start the new
/// track, register the end/error handlers, and record it as the guild's current
/// track.
async fn spawn_playback(
    ctx: PlayContext,
    call: Arc<Mutex<Call>>,
    input: Input,
    ytdlfile: YouTubeDl,
) {
    let volume = ctx.data.entry(ctx.guild_id).or_default().config.volume;
    let track = Track::from(input).volume(*volume);
    let track_handle = {
        let mut call = call.lock().await;
        call.stop();
        call.play_only(track)
    };

    let track_end_handler = TrackEndHandler {
        ctx: ctx.clone(),
        call,
        ytdl_file: ytdlfile.clone(),
    };

    // these are infallible since it only returns Err when Event::Core
    track_handle
        .add_event(Event::Track(TrackEvent::End), track_end_handler)
        .unwrap();
    track_handle
        .add_event(Event::Track(TrackEvent::Error), TrackErrorHandler)
        .unwrap();

    {
        let mut guilds_playing = ctx.playing.write().await;
        guilds_playing.insert(
            ctx.guild_id,
            Playing {
                track_handle,
                ytdlfile,
            },
        );
    }
}

/// Decide what plays after a track ends: repeat the current track, advance to the
/// next queued one, or auto-leave an empty queue. Called by the `track_end`
/// trigger, so the queue policy lives here rather than in the event handler.
#[instrument(name = "advance", skip_all, fields(guild = %ctx.guild_id))]
pub async fn advance(ctx: PlayContext, call: Arc<Mutex<Call>>, current: YouTubeDl) {
    let mut guild_data = ctx.data.entry(ctx.guild_id).or_default();
    let repeat = guild_data.config.repeat;
    let auto_leave = guild_data.config.auto_leave;
    let next = if repeat {
        None
    } else {
        guild_data.playlist.pop_front()
    };
    drop(guild_data);

    // The returned metadata future is dropped: there is no reply to render on an
    // autonomous advance, and it is never polled.
    if repeat {
        tracing::info!("repeating current track");
        if let Err(err) = play_track(ctx, call, current).await {
            tracing::warn!(error = %err, "failed to repeat track");
        }
        return;
    }

    if let Some(next) = next {
        tracing::info!(url = next.url(), "advancing to next track");
        if let Err(err) = play_track(ctx, call, next).await {
            tracing::warn!(error = %err, "failed to advance to next track");
        }
    } else if auto_leave.leaves_on_empty_queue() {
        tracing::info!("queue empty; auto-leaving");
        let mut call = call.lock().await;
        leave(&mut call, ctx.guild_id).await;
    }
}

/// Leave the voice channel on an autonomous trigger (queue drained, channel
/// emptied). Takes an already-locked [`Call`] so the caller's emptiness check and
/// the leave happen under one lock. Logs a failure rather than propagating it --
/// there is no command to report back to; the command-path leave keeps its `?`.
#[instrument(name = "leave", skip_all, fields(guild = %guild_id))]
pub async fn leave(call: &mut Call, guild_id: GuildId) {
    let channel = call.current_channel();
    if let Err(err) = call.leave().await {
        tracing::error!(error = %err, ?channel, "failed to leave voice channel");
    } else {
        tracing::info!("left voice channel");
    }
}

/// The slice of shared guild state a playback operation needs, decoupled from the
/// poise command [`Context`] so the autonomous triggers can build one too.
#[derive(Clone)]
pub struct PlayContext {
    pub guild_id: GuildId,
    pub data: Arc<Guilds>,
    pub playing: Arc<RwLock<HashMap<GuildId, Playing>>>,
}

impl TryFrom<Context<'_>> for PlayContext {
    type Error = CommandError;

    fn try_from(value: Context<'_>) -> Result<Self, Self::Error> {
        let guild_id = value.guild_id().ok_or(CommandError::GuildOnly)?;
        Ok(Self {
            guild_id,
            data: value.data().guilds.clone(),
            playing: value.data().playing.clone(),
        })
    }
}
