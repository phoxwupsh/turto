use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::Message,
    prelude::Context,
};

use crate::{
    guild::playlist::Playlists,
    models::{playlist_item::PlaylistItem, playlist::Playlist},
};

#[command]
#[bucket = "music"]
async fn queue(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    while !args.is_empty() {
        let url = args.single::<String>().unwrap();
        let Ok(source) = songbird::input::ytdl(&url).await else {
            msg.reply(ctx, format!("Cannot find `{}`", url)).await?;
            continue;
        };
        let metadata = source.metadata.clone();

        let playlists_lock = ctx
            .data
            .read()
            .await
            .get::<Playlists>()
            .expect("Expected Playlists in TypeMap.")
            .clone();
        {
            let mut playlists = playlists_lock.lock().await;
            let playlist = playlists
                .entry(msg.guild_id.expect("Expected guild_id"))
                .or_insert_with(Playlist::new);

            msg.reply(ctx, format!("✅ {}", metadata.title.clone().unwrap()))
                .await?;
            playlist.push_back(PlaylistItem::from(*metadata)); // Add song to playlist
        }   
    }
    Ok(())
}
