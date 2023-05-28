use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::Message,
    prelude::Context,
};

use crate::guild::playlist::{Playlist, Playlists};

#[command]
#[description = "刪除播放清單中的第`index`個項目，`index`是一個1或以上的整數。"]
#[usage = "index"]
#[example = "2"]
async fn remove(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let mut index =  match args.parse::<usize>() { // Check arg is number
        Ok(i) => {
            if i < 1 {
                msg.reply(ctx, "Enter a number no smaller that 1.").await?;
                return  Ok(())
            } else {
                i
            }
        },
        Err(_) => {
            msg.reply(ctx, "Enter a number.").await?;
            return  Ok(())
        }
    };
    index -= 1; // Index start from 1

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

        if index < playlist.len() {
            if let Some(removed) = playlist.remove(index) {
                msg.reply(ctx, format!("❎ {}", removed.title.clone().unwrap()))
                    .await?;
            } // Remove song from playlist
        }
    }
    Ok(())
}
