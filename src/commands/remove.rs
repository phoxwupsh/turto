use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::Message,
    prelude::Context,
};

use regex::Regex;

use crate::{messages::TurtoMessage, typemap::guild_data::GuildDataMap};

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

    let guild_data_map = ctx.data.read().await.get::<GuildDataMap>().unwrap().clone();
    let mut guild_data = guild_data_map.entry(msg.guild_id.unwrap()).or_default();

    match remove_item {
        RemoveType::All => {
            guild_data.playlist.clear();
            drop(guild_data);

            msg.reply(ctx, TurtoMessage::RemovaAll).await?;
        }
        RemoveType::Index(index) => {
            let title: String;
            let response = if let Some(removed) = guild_data.playlist.remove(index) {
                title = removed.title.clone();
                TurtoMessage::Remove { title: &title }
            } else {
                TurtoMessage::InvalidRemoveIndex {
                    playlist_length: guild_data.playlist.len(),
                }
            };
            drop(guild_data);
            
            msg.reply(ctx, response).await?;
        }
        RemoveType::Range { from, to } => {
            let playlist_range = 0..=guild_data.playlist.len();
            let response =
                if playlist_range.contains(&from) && playlist_range.contains(&to) && from < to {
                    let drained = guild_data.playlist.drain(from..to);
                    drained
                        .into_iter()
                        .map(|drained_item| {
                            TurtoMessage::Remove {
                                title: &drained_item.title,
                            }
                            .to_string()
                        })
                        .collect::<Vec<_>>()
                        .join("\n")
                } else {
                    TurtoMessage::InvalidRemoveIndex {
                        playlist_length: guild_data.playlist.len(),
                    }
                    .to_string()
                };
            drop(guild_data);
            
            msg.reply(ctx, response).await?;
        }
    }
    Ok(())
}
