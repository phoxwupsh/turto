use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::Message,
    prelude::Context,
};

use regex::Regex;

use crate::{guild::playlist::Playlists, models::playlist::Playlist};

enum RemoveType {
    All,
    Index(usize),
    Range { from: usize, to: usize },
}

#[command]
#[bucket = "music"]
async fn remove(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let re = Regex::new(r"(?<from>^[1-9]\d*)~(?<to>[1-9]\d*)").unwrap();
    let remove_item = match args.rest() {
        // Determine what to remove
        "all" => RemoveType::All, //Remove all
        other => match re.captures(other) {
            Some(caps) => {
                let from = caps
                    .name("from")
                    .unwrap()
                    .as_str()
                    .parse::<usize>()
                    .unwrap();
                let to = caps.name("to").unwrap().as_str().parse::<usize>().unwrap();
                RemoveType::Range {
                    from: from - 1, // Index start from 1
                    to, // to is inclusive thus + 1
                } 
            }
            None => match other.parse::<usize>() {
                // Check arg is number
                Ok(i) => {
                    if i < 1 {
                        msg.reply(ctx, "Enter a number no smaller that 1.").await?;
                        return Ok(());
                    } else {
                        RemoveType::Index(i - 1) // Index start from 1
                    }
                }
                Err(_) => {
                    msg.reply(ctx, "Enter a number or range.").await?;
                    return Ok(());
                }
            },
        },
    };

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
            .entry(msg.guild_id.unwrap())
            .or_insert_with(Playlist::new);

        match remove_item {
            RemoveType::All => {
                playlist.clear();
                msg.reply(ctx, "Playlist is empty now.").await?;
            }
            RemoveType::Index(index) => {
                if index < playlist.len() {
                    if let Some(removed) = playlist.remove(index) {
                        msg.reply(ctx, format!("❎ {}", removed.title)).await?;
                    } // Remove song from playlist
                }
            }
            RemoveType::Range { from, to } => {
                let playlist_range = 0..playlist.len();
                if playlist_range.contains(&from) && playlist_range.contains(&to) && from < to {
                    let drained = playlist.drain(from..to);
                    let response = drained
                        .into_iter()
                        .map(|drained_item| format!("❎ {}", drained_item.title))
                        .collect::<Vec<_>>()
                        .join("\n");
                    msg.reply(ctx, response).await?;
                } else {
                    msg.reply(ctx, "Invalid range").await?;
                }
            }
        }
    }
    Ok(())
}
