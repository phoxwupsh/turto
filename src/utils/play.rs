use super::get_http_client;
use crate::{
    config::get_config,
    handlers::track_end::TrackEndHandler,
    models::{guild::data::GuildData, playing::Playing},
};
use dashmap::DashMap;
use serenity::model::prelude::GuildId;
use songbird::{
    input::{AudioStreamError, AuxMetadata, Compose, Input, LiveInput, YoutubeDl},
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
    let mut source = {
        let source = YoutubeDl::new(get_http_client(), url.as_ref().to_string());
        if let Some(arg) = get_config().cookies_path.clone() {
            let args = vec!["--cookies".to_owned(), arg];
            source.user_args(args)
        } else {
            source
        }
    };

    // If doing this here it will call `YoutubeDl::query` which invoke yt-dlp
    // https://github.com/serenity-rs/songbird/blob/current/src/input/sources/ytdl.rs#L222
    // And since YoutubeDl is lazily instantiated which will become `Input::Lazy`
    // When we try to play `Input::Lazy` it calls `Compose::create_async` which also calls `YoutubeDl::query`
    // https://github.com/serenity-rs/songbird/blob/current/src/driver/tasks/mixer/track.rs#L199
    // https://github.com/serenity-rs/songbird/blob/current/src/driver/tasks/mixer/pool.rs#L31
    // This will cause yt-dlp to be invoke twice
    // let meta = Arc::new(source.aux_metadata().await?);

    // So we do it manually
    // This will make sure the metadata available
    let audio = source.create_async().await?;
    let meta = Arc::new(source.aux_metadata().await.unwrap());
    let input = Input::Live(LiveInput::Raw(audio), Some(Box::new(source)));

    let volume = guild_data.entry(guild_id).or_default().config.volume;
    let track = Track::from(input).volume(*volume);

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
    track_handle
        .add_event(Event::Track(TrackEvent::End), track_end_handler)
        .unwrap();
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
        Some(next) => Some(play_url(call, guild_data, guild_playing, guild_id, next.url).await),
        None => None,
    }
}
