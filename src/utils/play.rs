// ytdl, // Use restartable for seeking feature

use std::{error::Error, fmt::Display};
use serenity::{model::prelude::GuildId, prelude::Context};
use songbird::{
    input::{Metadata, Restartable, error::Error as InputError},
    Event, TrackEvent, tracks::TrackError,
};
use tracing::error;

use crate::{
    typemap::{playing::Playing, playlist::Playlists, config::GuildConfigs},
    handlers::track_end::TrackEndHandler,
    models::{playlist::Playlist, guild::config::GuildConfig},
};

pub async fn play_url<S>(ctx: &Context, guild_id: GuildId, url: S) -> Result<Metadata, PlayError>
where
    S: AsRef<str> + Send + Clone + Sync + 'static,
{
    let manager = songbird::get(ctx)
        .await
        .unwrap()
        .clone();

    let handler_lock = manager.get(guild_id).unwrap(); // When this method is called the bot must be in a voice channel
    let source = match Restartable::ytdl(url, true).await {
        Ok(s) => s,
        Err(e) => return Err(PlayError::InputError(e)),
    };
    // let source = ytdl(&url).await?; // Use restartable for seeking feature

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
            "Error adding TrackEndHandler for track {}: {:?}",
            song.uuid(),
            why
        );
        return Err(PlayError::TrackError(why));
    }

    let settings_lock = ctx
        .data
        .read()
        .await
        .get::<GuildConfigs>()
        .unwrap()
        .clone();
    {
        let mut settings = settings_lock.lock().await;
        let setting = settings
            .entry(guild_id)
            .or_insert_with(GuildConfig::default);
        if let Err(why) = song.set_volume(*setting.volume) {
            error!("Error setting volume of track {}: {:?}", song.uuid(), why);
            return Err(PlayError::TrackError(why));
        }
    }

    // Update the current track
    let playing_lock = ctx
        .data
        .read()
        .await
        .get::<Playing>()
        .unwrap()
        .clone();
    {
        let _track = playing_lock.write().await.insert(guild_id, song);
    }
    Ok(meta)
}

pub async fn play_next(ctx: &Context, guild_id: GuildId) -> Result<Metadata, PlayError> {
    let playlist_lock = ctx
        .data
        .read()
        .await
        .get::<Playlists>()
        .unwrap()
        .clone();
    let mut playlists = playlist_lock.lock().await;
    let playlist = playlists.entry(guild_id).or_insert_with(Playlist::new);

    match playlist.pop_front() {
        Some(next_song) => play_url(ctx, guild_id, next_song.url).await,
        None => Err(PlayError::EmptyPlaylist(guild_id)),
    }
}

#[derive(Debug)]
pub enum PlayError {
    TrackError(TrackError),
    InputError(InputError),
    EmptyPlaylist(GuildId)
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

