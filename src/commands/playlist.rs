use std::{sync::Arc, time::Duration};

use serenity::{
    framework::standard::{macros::command, CommandResult},
    futures::StreamExt,
    model::{
        application::interaction::InteractionResponseType,
        prelude::{interaction::message_component::MessageComponentInteraction, Message},
    },
    prelude::Context,
};

use crate::{typemap::playlist::Playlists, utils::misc::ToEmoji};

#[command]
#[bucket = "turto"]
async fn playlist(ctx: &Context, msg: &Message) -> CommandResult {
    let playlists_lock = ctx.data.read().await.get::<Playlists>().unwrap().clone();

    let waiting: Option<Message>;
    {
        let mut playlists = playlists_lock.lock().await;
        let playlist = playlists.entry(msg.guild_id.unwrap()).or_default();

        if playlist.is_empty() {
            msg.reply(ctx, "🈳").await?;
            return Ok(());
        }

        if playlist.len() <= 10 {
            // directly display if the playlist has less than 10 items
            let response = playlist
                .iter()
                .enumerate()
                .map(|(index, playlist_item)| {
                    let mut line = (index + 1).to_emoji();
                    line.push(' ');
                    line.push_str(&playlist_item.title);
                    line
                })
                .fold(String::new(), |acc, title| acc + &title + "\n")
                .trim_end()
                .to_owned();
            msg.reply(ctx, response).await?;
            return Ok(());
        } else {
            // show the select menu if the playlist has more than 10 items (discord text message has a length limitation of 2000 unicode chars)
            let select_page_msg = msg
                .channel_id
                .send_message(ctx, |m| {
                    m.reference_message(msg).components(|c| {
                        c.create_action_row(|r| {
                            r.create_select_menu(|menu| {
                                menu.custom_id("page_select").placeholder("📖❓").options(
                                    |options| {
                                        let page_num = (playlist.len() / 10) + 1;
                                        for i in 1..=page_num {
                                            options.create_option(|opt| {
                                                opt.label("📄".to_string() + &i.to_emoji())
                                                    .value(i.to_string())
                                            });
                                        }
                                        options
                                    },
                                )
                            })
                        })
                    })
                })
                .await?;
            waiting = Some(select_page_msg)
        }
    } // free the lock early

    if let Some(waiting) = waiting {
        let mut interactions = waiting
            .await_component_interactions(ctx)
            .timeout(Duration::from_secs(60))
            .build();

        let interaction = {
            let res: Arc<MessageComponentInteraction>;
            loop {
                match interactions.next().await {
                    Some(interaction) => {
                        if interaction.user == msg.author {
                            // response only if the interaction is send by the user who invoke the help command
                            res = interaction;
                            break;
                        }
                    }
                    None => {
                        // if there is no interaction sended, delete the select menu
                        waiting.delete(ctx).await?;
                        return Ok(());
                    }
                }
            }
            res
        };
        if let Ok(page_num) = interaction.data.values[0].parse::<usize>() {
            let page_index = page_num - 1;
            let mut playlists = playlists_lock.lock().await;
            let playlist = playlists.entry(msg.guild_id.unwrap()).or_default();
            let response = {
                playlist
                    .iter()
                    .enumerate()
                    .filter(|(index, _playlist_item)| index / 10 == page_index)
                    .map(|(index, playlist_item)| {
                        let mut line = (index + 1).to_emoji();
                        line.push(' ');
                        line.push_str(&playlist_item.title);
                        line
                    })
                    .fold(String::new(), |acc, title| acc + &title + "\n")
                    .trim_end()
                    .to_owned()
            };
            interaction
                .create_interaction_response(ctx, |res| {
                    res.kind(InteractionResponseType::UpdateMessage)
                        .interaction_response_data(|data| {
                            data.content(response).components(|component| component)
                        })
                })
                .await?;
            return Ok(());
        }
    }

    Ok(())
}
