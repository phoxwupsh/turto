use crate::{
    messages::{
        TurtoMessage,
        TurtoMessageKind::{InvalidRemove, Remove},
    },
    models::alias::{Context, Error},
};

enum RemoveType {
    Index(usize),
    Range { from: usize, to: usize },
}

#[poise::command(slash_command, guild_only)]
pub async fn remove(
    ctx: Context<'_>,
    #[min = 0] which: usize,
    #[min = 0] to_which: Option<usize>,
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

    match remove_item {
        RemoveType::Index(index) => {
            let title: String;
            let response = if let Some(removed) = guild_data.playlist.remove(index) {
                title = removed.title.clone();
                TurtoMessage {
                    locale,
                    kind: Remove { title: &title },
                }
            } else {
                TurtoMessage {
                    locale,
                    kind: InvalidRemove {
                        length: guild_data.playlist.len(),
                    },
                }
            };
            drop(guild_data);

            ctx.say(response).await?;
        }
        RemoveType::Range { from, to } => {
            let playlist_range = 0..=guild_data.playlist.len();
            let response =
                if playlist_range.contains(&from) && playlist_range.contains(&to) && from < to {
                    let drained = guild_data.playlist.drain(from..to);
                    drained
                        .into_iter()
                        .map(|drained_item| {
                            TurtoMessage {
                                locale,
                                kind: Remove {
                                    title: &drained_item.title,
                                },
                            }
                            .to_string()
                        })
                        .collect::<Vec<_>>()
                        .join("\n")
                } else {
                    TurtoMessage {
                        locale,
                        kind: InvalidRemove {
                            length: guild_data.playlist.len(),
                        },
                    }
                    .to_string()
                };
            drop(guild_data);

            ctx.say(response).await?;
        }
    }
    Ok(())
}
