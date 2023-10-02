use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::Message,
    prelude::Context,
};

use regex::Regex;

use crate::{messages::TurtoMessage, typemap::playlist::Playlists};

enum RemoveType {
    All,
    Index(usize),
    Range { from: usize, to: usize },
}

#[command]
#[bucket = "turto"]
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
                    to,             // to is inclusive thus no need to + 1
                }
            }
            None => match other.parse::<usize>() {
                // Check arg is number
                Ok(i) => {
                    if i < 1 {
                        msg.reply(ctx, TurtoMessage::InvalidRemove).await?;
                        return Ok(());
                    } else {
                        RemoveType::Index(i - 1) // Index start from 1
                    }
                }
                Err(_) => {
                    msg.reply(ctx, TurtoMessage::InvalidRemove).await?;
                    return Ok(());
                }
            },
        },
    };

    let playlists_lock = ctx.data.read().await.get::<Playlists>().unwrap().clone();
    {
        let mut playlists = playlists_lock.lock().await;
        let playlist = playlists.entry(msg.guild_id.unwrap()).or_default();

        match remove_item {
            RemoveType::All => {
                playlist.clear();
                msg.reply(ctx, TurtoMessage::RemovaAll).await?;
            }
            RemoveType::Index(index) => {
                if let Some(removed) = playlist.remove(index) {
                    msg.reply(ctx, TurtoMessage::Remove { title: &removed.title }).await?;
                } else {
                    msg.reply(ctx, TurtoMessage::InvalidRemoveIndex { playlist_length: playlist.len() }).await?;
                }
            }
            RemoveType::Range { from, to } => {
                let playlist_range = 0..=playlist.len();
                if playlist_range.contains(&from) && playlist_range.contains(&to) && from < to {
                    let drained = playlist.drain(from..to);
                    let response = drained
                        .into_iter()
                        .map(|drained_item| {
                            TurtoMessage::Remove {
                                title: &drained_item.title,
                            }
                            .to_string()
                        })
                        .collect::<Vec<_>>()
                        .join("\n");
                    msg.reply(ctx, response).await?;
                } else {
                    msg.reply(ctx, TurtoMessage::InvalidRemoveIndex { playlist_length: playlist.len() }).await?;
                }
            }
        }
    }
    Ok(())
}
