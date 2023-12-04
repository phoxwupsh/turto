use super::get_http_client;
use crate::{
    handlers::track_end::TrackEndHandler,
    models::playing::Playing,
    typemap::{guild_data::GuildDataMap, playing::PlayingMap},
};
use serenity::{model::prelude::GuildId, prelude::TypeMap};
use songbird::{
    input::{AudioStreamError, AuxMetadata, Compose, YoutubeDl},
    tracks::{ControlError, Track},
    Call, Event, TrackEvent,
};
use std::{error::Error, fmt::Display, sync::Arc};
use tokio::sync::{Mutex, RwLock};
use tracing::error;

pub async fn play_url(
    call: Arc<Mutex<Call>>,
    data: Arc<RwLock<TypeMap>>,
    guild_id: GuildId,
    url: String,
) -> Result<Arc<AuxMetadata>, PlayError> {
    let mut source = YoutubeDl::new(get_http_client(), url.clone());
    let meta = match source.aux_metadata().await {
        Ok(meta) => Arc::new(meta),
        Err(err) => return Err(PlayError::AudioStreamError(err)),
    };

    let track = {
        let guild_data_map = data.read().await.get::<GuildDataMap>().unwrap().clone();
        let volume = guild_data_map.entry(guild_id).or_default().config.volume;
        Track::from(source).volume(*volume)
    };

    let track_handle = {
        let mut call = call.lock().await;
        call.stop();
        call.play_only(track)
    };

    let track_end_handler = TrackEndHandler {
        data: data.clone(),
        call: call.clone(),
        url,
        guild_id,
    };

    if let Err(why) = track_handle.add_event(Event::Track(TrackEvent::End), track_end_handler) {
        error!(
            "Failed to add TrackEndHandler to track {}: {}",
            track_handle.uuid(),
            why
        );
        return Err(PlayError::ControlError(why));
    }

    let playing = Playing {
        track_handle,
        metadata: meta.clone(),
    };

    // Update the current track
    let playing_lock = data.read().await.get::<PlayingMap>().unwrap().clone();
    let _playing = playing_lock.write().await.insert(guild_id, playing);

    Ok(meta)
}

pub async fn play_next(
    call: Arc<Mutex<Call>>,
    data: Arc<RwLock<TypeMap>>,
    guild_id: GuildId,
) -> Option<Result<Arc<AuxMetadata>, PlayError>> {
    let guild_data_map = data.read().await.get::<GuildDataMap>().unwrap().clone();
    let mut guild_data = guild_data_map.entry(guild_id).or_default();
    let next = guild_data.playlist.pop_front();
    drop(guild_data);

    match next {
        Some(next_track) => Some(play_url(call, data, guild_id, next_track.url).await),
        None => None,
    }
}

#[derive(Debug)]
pub enum PlayError {
    ControlError(ControlError),
    AudioStreamError(AudioStreamError),
}

impl Display for PlayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ControlError(e) => f.write_str(&e.to_string()),
            Self::AudioStreamError(e) => f.write_str(&e.to_string()),
        }
    }
}

impl Error for PlayError {}
