use crate::{
    handlers::track_end::TrackEndHandler,
    typemap::{guild_data::GuildDataMap, playing::Playing},
};
use serenity::{
    model::prelude::GuildId,
    prelude::TypeMap,
};
use songbird::{
    input::{error::Error as InputError, Metadata, Restartable},
    tracks::TrackError,
    Call, Event, TrackEvent,
};
use std::{error::Error, fmt::Display, sync::Arc};
use tokio::sync::{Mutex, RwLock};
use tracing::error;

pub async fn play_url<S>(
    call: Arc<Mutex<Call>>,
    data: Arc<RwLock<TypeMap>>,
    guild_id: GuildId,
    url: S,
) -> Result<Metadata, PlayError>
where
    S: AsRef<str> + Send + Clone + Sync + 'static,
{
    let source = match Restartable::ytdl(url, true).await {
        // Use restartable for seeking feature
        Ok(s) => s,
        Err(e) => return Err(PlayError::InputError(e)),
    };

    let track = {
        let mut call = call.lock().await;
        call.stop();
        call.play_only_source(source.into())
    };

    let meta = track.metadata().clone();

    let track_end_handler = TrackEndHandler {
        data: data.clone(),
        call: call.clone(),
        guild_id,
    };

    if let Err(why) = track.add_event(Event::Track(TrackEvent::End), track_end_handler) {
        error!(
            "Failed to add TrackEndHandler to track {}: {}",
            track.uuid(),
            why
        );
        return Err(PlayError::TrackError(why));
    }

    let guild_data_map = data.read().await.get::<GuildDataMap>().unwrap().clone();
    let guild_data = guild_data_map.entry(guild_id).or_default();
    if let Err(why) = track.set_volume(*guild_data.config.volume) {
        error!("Failed to set volume for track {}: {}", track.uuid(), why);
        return Err(PlayError::TrackError(why));
    }
    drop(guild_data);

    // Update the current track
    let playing_lock = data.read().await.get::<Playing>().unwrap().clone();
    {
        let _track = playing_lock.write().await.insert(guild_id, track);
    }
    Ok(meta)
}

pub async fn play_next(call: Arc<Mutex<Call>>, data: Arc<RwLock<TypeMap>>, guild_id: GuildId) -> Result<Metadata, PlayError> {
    let guild_data_map = data.read().await.get::<GuildDataMap>().unwrap().clone();
    let mut guild_data = guild_data_map.entry(guild_id).or_default();
    let next = guild_data.playlist.pop_front();
    drop(guild_data);

    match next {
        Some(next_track) => play_url(call, data, guild_id, next_track.url).await,
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
