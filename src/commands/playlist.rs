use serenity::{
    framework::standard::{macros::command, CommandResult},
    model::prelude::Message,
    prelude::Context,
};

use crate::{guild::playlist::Playlists, models::playlist::Playlist, utils::i32_to_emoji};

#[command]
#[bucket = "music"]
async fn playlist(ctx: &Context, msg: &Message) -> CommandResult {
    let playlists_lock = ctx
        .data
        .read()
        .await
        .get::<Playlists>()
        .expect("Expected Playlists in TypeMap.")
        .clone();

    let titles = {
        let mut playlists = playlists_lock.lock().await;
        let playlist = playlists
            .entry(msg.guild_id.expect("Expected guild_id"))
            .or_insert_with(Playlist::new);
        playlist
            .iter()
            .enumerate()
            .map(|(index, playlist_item)| {
                // Index each titles
                let index = (index as i32) + 1; // Index start from 1
                let mut line = i32_to_emoji(index);
                line.push_str(&playlist_item.title);
                line
            })
            .collect::<Vec<String>>()
    };

    let response = {
        if !titles.is_empty() {
            titles.join("\n")
        } else {
            "🈳".to_string()
        }
    };

    msg.reply(ctx, response).await?;

    Ok(())
}
