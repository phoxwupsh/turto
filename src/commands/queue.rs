use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::Message,
    prelude::Context,
};

use crate::guild::playlist::{Playlist, Playlists};

#[command]
#[description = "在播放清單中加入新的項目，`網址`目前只支援YouTube的影片(直播不行)。"]
#[usage = "網址"]
#[example = "https://youtu.be/dQw4w9WgXcQ"]
#[example = "https://www.youtube.com/watch?v=dQw4w9WgXcQ"]
async fn queue(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let url = args.rest();
    let source = songbird::input::ytdl(&url).await?;
    let metadata = source.metadata.clone();

    let playlists_lock = {
        let data_read = ctx.data.read().await;
        data_read
            .get::<Playlists>()
            .expect("Expected Playlists in TypeMap.")
            .clone()
    };
    {
        let mut playlists = playlists_lock.lock().await;
        let playlist = playlists
            .entry(msg.guild_id.expect("Expected guild_id"))
            .or_insert_with(Playlist::new);

        msg.reply(ctx, format!("✅ {}", metadata.title.clone().unwrap()))
            .await?;
        playlist.push_back(*metadata); // Add song to playlist
    }
    Ok(())
}
