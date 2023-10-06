use crate::{
    handlers::track_end::TrackEndHandler,
    typemap::{guild_data::GuildDataMap, playing::Playing},
};
use serenity::{model::prelude::GuildId, prelude::Context};
use songbird::{
    input::{error::Error as InputError, Metadata, Restartable},
    tracks::TrackError,
    Event, TrackEvent,
};
use std::{error::Error, fmt::Display};
use tracing::error;

pub async fn play_url<S>(ctx: &Context, guild_id: GuildId, url: S) -> Result<Metadata, PlayError>
where
    S: AsRef<str> + Send + Clone + Sync + 'static,
{
    let manager = songbird::get(ctx).await.unwrap().clone();

    let handler_lock = manager.get(guild_id).unwrap(); // When this method is called the bot must be in a voice channel
    let source = match Restartable::ytdl(url, true).await {
        // Use restartable for seeking feature
        Ok(s) => s,
        Err(e) => return Err(PlayError::InputError(e)),
    };

    let song = {
        let mut handler = handler_lock.lock().await;
        handler.stop();
        handler.play_only_source(source.into())
    };

    let meta = song.metadata().clone();

    let next_song_handler = TrackEndHandler {
        ctx: ctx.clone(),
        guild_id,
    };

    if let Err(why) = song.add_event(Event::Track(TrackEvent::End), next_song_handler) {
        error!(
            "Error adding TrackEndHandler for track {}: {}",
            song.uuid(),
            why
        );
        return Err(PlayError::TrackError(why));
    }

    let guild_data_map = ctx.data.read().await.get::<GuildDataMap>().unwrap().clone();
    let guild_data = guild_data_map.entry(guild_id).or_default();
    if let Err(why) = song.set_volume(*guild_data.config.volume) {
        error!("Error setting volume of track {}: {}", song.uuid(), why);
        return Err(PlayError::TrackError(why));
    }
    drop(guild_data);

    // Update the current track
    let playing_lock = ctx.data.read().await.get::<Playing>().unwrap().clone();
    {
        let _track = playing_lock.write().await.insert(guild_id, song);
    }
    Ok(meta)
}

pub async fn play_next(ctx: &Context, guild_id: GuildId) -> Result<Metadata, PlayError> {
    let guild_data_map = ctx.data.read().await.get::<GuildDataMap>().unwrap().clone();
    let mut guild_data = guild_data_map.entry(guild_id).or_default();
    let next = guild_data.playlist.pop_front();
    drop(guild_data);

    match next {
        Some(next_song) => play_url(ctx, guild_id, next_song.url).await,
        None => Err(PlayError::EmptyPlaylist(guild_id)),
    }
}

#[derive(Debug)]
pub enum PlayError {
    TrackError(TrackError),
    InputError(InputError),
    EmptyPlaylist(GuildId),
}

impl Display for PlayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyPlaylist(g) => f.write_str(&format!("The playlist of guild {} is empty", g)),
            Self::TrackError(e) => f.write_str(&e.to_string()),
            Self::InputError(e) => f.write_str(&e.to_string()),
        }
    }
}

impl Error for PlayError {}
