use super::get_http_client;
use crate::{
    handlers::track_end::TrackEndHandler,
    models::{guild::data::GuildData, playing::Playing},
};
use dashmap::DashMap;
use serenity::model::prelude::GuildId;
use songbird::{
    input::{AudioStreamError, AuxMetadata, Compose, YoutubeDl},
    tracks::{ControlError, Track},
    Call, Event, TrackEvent,
};
use std::{collections::HashMap, error::Error, fmt::Display, sync::Arc};
use tokio::sync::{Mutex, RwLock};
use tracing::error;

pub async fn play_url(
    call: Arc<Mutex<Call>>,
    guild_data: Arc<DashMap<GuildId, GuildData>>,
    guild_playing: Arc<RwLock<HashMap<GuildId, Playing>>>,
    guild_id: GuildId,
    url: impl AsRef<str>,
) -> Result<Arc<AuxMetadata>, PlayError> {
    let mut source = YoutubeDl::new(get_http_client(), url.as_ref().to_owned());
    let meta = match source.aux_metadata().await {
        Ok(meta) => Arc::new(meta),
        Err(err) => return Err(PlayError::AudioStreamError(err)),
    };

    let track = {
        let volume = guild_data.entry(guild_id).or_default().config.volume;
        Track::from(source).volume(*volume)
    };

    let track_handle = {
        let mut call = call.lock().await;
        call.stop();
        call.play_only(track)
    };

    let track_end_handler = TrackEndHandler {
        guild_data: guild_data.clone(),
        guild_playing: guild_playing.clone(),
        call: call.clone(),
        url: url.as_ref().into(),
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
    let _playing = guild_playing.write().await.insert(guild_id, playing);

    Ok(meta)
}

pub async fn play_next(
    call: Arc<Mutex<Call>>,
    guild_data: Arc<DashMap<GuildId, GuildData>>,
    guild_playing: Arc<RwLock<HashMap<GuildId, Playing>>>,
    guild_id: GuildId,
) -> Option<Result<Arc<AuxMetadata>, PlayError>> {
    let next = guild_data.entry(guild_id).or_default().playlist.pop_front();

    match next {
        Some(next_track) => {
            Some(play_url(call, guild_data, guild_playing, guild_id, &next_track.url).await)
        }
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
