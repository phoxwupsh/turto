use std::error::Error;

use serenity::{
    client::Context,
    model::{channel::Message, prelude::{ChannelId, GuildId}}
};
use songbird::{
    input::{Metadata, Restartable},
    // ytdl, // Use restartable for seeking feature
    events::{
        Event as SongbirdEvent,
        TrackEvent
    }
};

use tracing::error;

use crate::{guild::{
    playing::Playing,
    playlist::{
        Playlists, 
        Playlist
    }, volume::{Volume, GuildVolume}
}, handlers::track_end::PlayNextSong};

pub fn convert_to_emoji(num: i32) -> String {
    let num_str = num.to_string();
    let mut emoji_str = String::new();

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

    guild.voice_states.get(&msg.author.id)
        .and_then(|voice_state| voice_state.channel_id)
}

pub async fn bot_in_voice_channel(ctx: &Context, msg: &Message) -> Option<ChannelId> {
    let manager = songbird::get(&ctx).await
        .expect("Songbird Voice client placing in Resource failed.")
        .clone();

    let guild = msg.guild(ctx).unwrap();

    match manager.get(guild.id) {
        Some(_) => return guild.voice_states.get(&ctx.cache.current_user_id())
            .and_then(|voice_state| voice_state.channel_id),
        None => return None
    }
}

pub async fn same_voice_channel(ctx: &Context, msg: &Message) -> bool {
    let user_channel = user_in_voice_channel(ctx, msg).await;
    let bot_channel = bot_in_voice_channel(ctx, msg).await;

    user_channel == bot_channel && user_channel.is_some()
}

pub async fn play_song<S>(ctx: &Context, guild_id: GuildId, url: S) -> Result<Metadata, Box<dyn Error + Send + Sync>> where 
    S: AsRef<str> + Send + Clone + Sync + 'static
{
    let manager = songbird::get(ctx).await
        .expect("Songbird voice client placement in resource map failed.")
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let source = Restartable::ytdl(url, true).await?;
        // let source = ytdl(&url).await?; // Use restartable for seeking feature

        let song = {
            let mut handler = handler_lock.lock().await;
            handler.stop(); 
            handler.play_only_source(source.into())
        };
        
        let meta = song.metadata().clone();

        let next_song_handler = PlayNextSong { 
            ctx: ctx.clone(), 
            guild_id,
        };

        if let Err(why) = song.add_event(SongbirdEvent::Track(TrackEvent::End), next_song_handler) {
            error!("Error adding next song handler for track {}: {:?}", song.uuid(), why);
        }

        let volume_lock = {
            let data_read = ctx.data.read().await;
            data_read.get::<Volume>().expect("Expected Playing in TypeMap").clone()
        };
        {
            let mut volume = volume_lock.lock().await;
            let new_volume = volume.entry(guild_id).or_insert(GuildVolume::default());
            if let Err(why) = song.set_volume(**new_volume) {
                error!("Error setting volume of track {}: {:?}", song.uuid(), why);
            }
        }

        // Update the current track
        let playing_lock = {
            let data_read = ctx.data.read().await;
            data_read.get::<Playing>().expect("Expected Playing in TypeMap").clone()
        };
        {
            let mut playing = playing_lock.write().await;
            let _ = playing.insert(guild_id, song);
        }
        return Ok(meta);
    }
    Err(String::from("Error playing song while creating handler").into())
}

pub async fn play_next(ctx: &Context, guild_id: GuildId) -> Result<Metadata, Box<dyn Error + Send + Sync>>  {
    let data_read = ctx.data.read().await;
    let playlists = data_read.get::<Playlists>().expect("Expected Playlists in TypeMap.");
    let mut playlists = playlists.lock().await;
    let playlist = playlists.entry(guild_id).or_insert_with(Playlist::new);

    match playlist.pop_front() {
        Some(next_song) =>{
            return play_song(ctx, guild_id, next_song.source_url.clone().unwrap()).await
        },
        None => {
            return Err("The playlist is empty".into());
        }
    }
}