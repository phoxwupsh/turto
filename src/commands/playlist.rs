use serenity::{
    framework::standard::{macros::command, CommandResult},
    model::prelude::Message,
    prelude::Context,
};

use crate::{
    guild::playlist::{Playlist, Playlists},
    utils::convert_to_emoji,
};

#[command]
#[description = "顯示目前的播放清單，整個伺服器共用同一個播放清單。"]
#[usage = "網址"]
#[example = "https://youtu.be/dQw4w9WgXcQ"]
async fn playlist(ctx: &Context, msg: &Message) -> CommandResult {
    let data_read = ctx.data.read().await;
    let playlists = data_read
        .get::<Playlists>()
        .expect("Expected Playlists in TypeMap.");
    let mut playlists = playlists.lock().await;
    let playlist = playlists
        .entry(msg.guild_id.expect("Expected guild_id"))
        .or_insert_with(Playlist::new);

    let titles: Vec<String> = playlist
        .iter()
        .enumerate()
        .map(|(index, metadata)| {
            // Index each titles
            let index = (index as i32) + 1; // Index start from 1
            let mut line = convert_to_emoji(index);
            line.push_str(&metadata.title.clone().unwrap());
            line
        })
        .collect();
    let response = {
        if titles.len() > 0 {
            titles.join("\n")
        }
        else {
            "🈳".to_string()
        }
    };

    msg.reply(ctx, response).await?;

    Ok(())
}
