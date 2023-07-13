use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::Message,
    prelude::Context,
};

use crate::{guild::playlist::Playlists, models::playlist::Playlist};

#[command]
#[bucket = "music"]
async fn remove(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let mut index = match args.parse::<usize>() {
        // Check arg is number
        Ok(i) => {
            if i < 1 {
                msg.reply(ctx, "Enter a number no smaller that 1.").await?;
                return Ok(());
            } else {
                i
            }
        }
        Err(_) => {
            msg.reply(ctx, "Enter a number.").await?;
            return Ok(());
        }
    };
    index -= 1; // Index start from 1

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

        if index < playlist.len() {
            if let Some(removed) = playlist.remove(index) {
                msg.reply(ctx, format!("❎ {}", removed.title.clone().unwrap()))
                    .await?;
            } // Remove song from playlist
        }
    }
    Ok(())
}
