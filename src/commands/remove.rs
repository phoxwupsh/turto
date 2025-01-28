use crate::{
    messages::{
        TurtoMessage,
        TurtoMessageKind::{InvalidRangeRemove, InvalidRemove, Remove, RemoveMany},
    },
    models::alias::{Context, Error},
    utils::turto_say,
};

enum RemoveType {
    Index(usize),
    Range { from: usize, to: usize },
}

#[poise::command(slash_command, guild_only)]
pub async fn remove(
    ctx: Context<'_>,
    #[min = 1] which: usize,
    #[min = 1] to_which: Option<usize>,
) -> Result<(), Error> {
    let remove_item = match to_which {
        Some(to_which) => RemoveType::Range {
            from: which - 1, // the playlist index start from 1 so -1
            to: to_which,    // inclusive so no need to -1
        },
        None => RemoveType::Index(which - 1),
    };

    let guild_id = ctx.guild_id().unwrap();
    let mut guild_data = ctx.data().guilds.entry(guild_id).or_default();
    let locale = ctx.locale();
    let length = guild_data.playlist.len();

    match remove_item {
        RemoveType::Index(index) => {
            // Check if the index is out of bounds
            if index >= length {
                drop(guild_data);
                turto_say(ctx, InvalidRemove { length }).await?;
                return Ok(());
            }
            let removed = guild_data.playlist.remove(index).unwrap();
            let title = removed.title.as_str();
            drop(guild_data);
            turto_say(ctx, Remove { title }).await?;
        }
        RemoveType::Range { from, to } => {
            // Check if the range is invalid
            if from > to || length <= from || length <= to {
                drop(guild_data);
                turto_say(ctx, InvalidRangeRemove { from, to, length }).await?;
                return Ok(());
            }
            let drained = guild_data
                .playlist
                .drain(from..to)
                .map(|drained_item| {
                    TurtoMessage {
                        locale,
                        kind: Remove {
                            title: &drained_item.title,
                        },
                    }
                    .to_string()
                })
                .collect::<Vec<_>>();
            drop(guild_data);

            let response = if drained.len() > 10 {
                TurtoMessage {
                    locale,
                    kind: RemoveMany {
                        removed_number: drained.len(),
                    },
                }
                .to_string()
            } else {
                drained.join("\n")
            };
            ctx.say(response).await?;
        }
    }
    Ok(())
}
