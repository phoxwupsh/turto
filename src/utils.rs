use serenity::{
    client::Context,
    model::{
        channel::Message,
        prelude::{ChannelId, GuildId},
    },
};
use songbird::{
    // ytdl, // Use restartable for seeking feature
    events::{Event as SongbirdEvent, TrackEvent},
    input::{Metadata, Restartable},
};

use tracing::error;

use crate::{
    error::TurtoError,
    guild::{playing::Playing, playlist::Playlists, setting::Settings},
    handlers::track_end::TrackEndHandler,
    models::{playlist::Playlist, setting::GuildSetting},
};

pub fn i32_to_emoji(num: i32) -> String {
    let num_str = num.to_string();
    let mut emoji_str = String::new();

    if num < 0 {
        emoji_str.push_str("➖");
    }

    for ch in num_str.chars() {
        let emoji = match ch {
            '0' => "0️⃣",
            '1' => "1️⃣",
            '2' => "2️⃣",
            '3' => "3️⃣",
            '4' => "4️⃣",
            '5' => "5️⃣",
            '6' => "6️⃣",
            '7' => "7️⃣",
            '8' => "8️⃣",
            '9' => "9️⃣",
            _ => continue,
        };
        emoji_str.push_str(emoji);
    }
    emoji_str
}

pub async fn user_in_voice_channel(ctx: &Context, msg: &Message) -> Option<ChannelId> {
    let guild = msg.guild(&ctx.cache).unwrap();

    guild
        .voice_states
        .get(&msg.author.id)
        .and_then(|voice_state| voice_state.channel_id)
}

pub async fn bot_in_voice_channel(ctx: &Context, msg: &Message) -> Option<ChannelId> {
    let manager = songbird::get(&ctx)
        .await
        .expect("Songbird Voice client placing in Resource failed.")
        .clone();

    let guild = msg.guild(ctx).unwrap();

    match manager.get(guild.id) {
        Some(_) => {
            return guild
                .voice_states
                .get(&ctx.cache.current_user_id())
                .and_then(|voice_state| voice_state.channel_id)
        }
        None => return None,
    }
}

pub async fn same_voice_channel(ctx: &Context, msg: &Message) -> bool {
    let user_channel = user_in_voice_channel(ctx, msg).await;
    let bot_channel = bot_in_voice_channel(ctx, msg).await;

    user_channel == bot_channel && user_channel.is_some()
}

pub async fn play_song<S>(ctx: &Context, guild_id: GuildId, url: S) -> Result<Metadata, TurtoError>
where
    S: AsRef<str> + Send + Clone + Sync + 'static,
{
    let manager = songbird::get(ctx)
        .await
        .expect("Songbird voice client placement in resource map failed.")
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let source = match Restartable::ytdl(url, true).await {
            Ok(s) => s,
            Err(e) => return Err(TurtoError::InputError(e)),
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

        if let Err(why) = song.add_event(SongbirdEvent::Track(TrackEvent::End), next_song_handler) {
            error!(
                "Error adding TrackEndHandler for track {}: {:?}",
                song.uuid(),
                why
            );
            return Err(TurtoError::TrackError(why));
        }

        let settings_lock = ctx
            .data
            .read()
            .await
            .get::<Settings>()
            .expect("Expected Settings in TypeMap")
            .clone();
        {
            let mut settings = settings_lock.lock().await;
            let setting = settings
                .entry(guild_id)
                .or_insert_with(GuildSetting::default);
            if let Err(why) = song.set_volume(*setting.volume) {
                error!("Error setting volume of track {}: {:?}", song.uuid(), why);
                return Err(TurtoError::TrackError(why));
            }
        }

        // Update the current track
        let playing_lock = ctx
            .data
            .read()
            .await
            .get::<Playing>()
            .expect("Expected Playing in TypeMap")
            .clone();
        {
            let _track = playing_lock.write().await.insert(guild_id, song);
        }
        return Ok(meta);
    }
    Err(TurtoError::InternalError(
        "Error playing song while creating handler",
    ))
}

pub async fn play_next(ctx: &Context, guild_id: GuildId) -> Result<Metadata, TurtoError> {
    let playlist_lock = ctx
        .data
        .read()
        .await
        .get::<Playlists>()
        .expect("Expected Playlists in TypeMap.")
        .clone();
    let mut playlists = playlist_lock.lock().await;
    let playlist = playlists.entry(guild_id).or_insert_with(Playlist::new);

    match playlist.pop_front() {
        Some(next_song) => {
            return play_song(ctx, guild_id, next_song.source_url.clone().unwrap()).await
        }
        None => {
            return Err(TurtoError::EmptyPlaylist);
        }
    }
}
