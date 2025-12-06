use crate::{
    handlers::{track_end::TrackEndHandler, track_error::TrackErrorHandler},
    models::{alias::Context, guild::Guilds, playing::Playing},
    ytdl::{YouTubeDl, YouTubeDlMetadata},
};
use serenity::all::GuildId;
use songbird::{Call, Event, TrackEvent, input::Input, tracks::Track};
use std::{collections::HashMap, future::Future, pin::Pin, sync::Arc};
use tokio::sync::{Mutex, RwLock};

pub async fn play_ytdlfile_meta(
    ctx: PlayContext,
    call: Arc<Mutex<Call>>,
    ytdlfile: YouTubeDl,
) -> std::io::Result<Pin<Box<dyn Future<Output = std::io::Result<Arc<YouTubeDlMetadata>>> + Send>>>
{
    let (meta, input) = ytdlfile.play().await?;
    tokio::spawn(play_ytdlfile_inner(ctx, call, input, ytdlfile));

    Ok(meta)
}

pub async fn play_ytdlfile(
    ctx: PlayContext,
    call: Arc<Mutex<Call>>,
    ytdlfile: YouTubeDl,
) -> std::io::Result<()> {
    let input = ytdlfile.fetch_file().await?;
    tokio::spawn(play_ytdlfile_inner(ctx, call, input, ytdlfile));

    Ok(())
}

async fn play_ytdlfile_inner(
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

#[derive(Clone)]
pub struct PlayContext {
    pub guild_id: GuildId,
    pub data: Arc<Guilds>,
    pub playing: Arc<RwLock<HashMap<GuildId, Playing>>>,
}

impl PlayContext {
    pub fn from_ctx(ctx: Context<'_>) -> Option<Self> {
        Some(Self {
            guild_id: ctx.guild_id()?,
            data: ctx.data().guilds.clone(),
            playing: ctx.data().playing.clone(),
        })
    }
}
