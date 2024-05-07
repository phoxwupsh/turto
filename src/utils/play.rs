use super::get_http_client;
use crate::{
    handlers::track_end::TrackEndHandler,
    models::{guild::data::GuildData, playing::Playing},
};
use dashmap::DashMap;
use serenity::model::prelude::GuildId;
use songbird::{
    input::{AudioStreamError, AuxMetadata, Compose, YoutubeDl},
    tracks::Track,
    Call, Event, TrackEvent,
};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{Mutex, RwLock};

pub async fn play_url(
    call: Arc<Mutex<Call>>,
    guild_data: Arc<DashMap<GuildId, GuildData>>,
    guild_playing: Arc<RwLock<HashMap<GuildId, Playing>>>,
    guild_id: GuildId,
    url: impl AsRef<str>,
) -> Result<Arc<AuxMetadata>, AudioStreamError> {
    let mut source = YoutubeDl::new(get_http_client(), url.as_ref().to_string());
    let meta = Arc::new(source.aux_metadata().await?);

    let volume = guild_data.entry(guild_id).or_default().config.volume;
    let track = Track::from(source).volume(*volume);

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

    // This is infallible
    track_handle.add_event(Event::Track(TrackEvent::End), track_end_handler).unwrap();
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
) -> Option<Result<Arc<AuxMetadata>, AudioStreamError>> {
    let next = guild_data.entry(guild_id).or_default().playlist.pop_front();

    match next {
        Some(next) => {
            Some(play_url(call, guild_data, guild_playing, guild_id, next.url).await)
        }
        None => None,
    }
}