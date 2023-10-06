use std::{sync::Arc, time::Duration};

use serenity::{
    builder::CreateComponents,
    framework::standard::{macros::command, CommandResult},
    futures::StreamExt,
    model::{
        application::interaction::InteractionResponseType,
        prelude::{interaction::message_component::MessageComponentInteraction, Message},
    },
    prelude::Context,
};

use crate::{typemap::guild_data::GuildDataMap, utils::misc::ToEmoji};

#[command]
#[bucket = "turto"]
async fn playlist(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_data_map = ctx.data.read().await.get::<GuildDataMap>().unwrap().clone();
    let guild_data = guild_data_map.entry(msg.guild_id.unwrap()).or_default();
    let waiting: Option<Message>;

    if guild_data.playlist.is_empty() {
        drop(guild_data);
        msg.reply(ctx, "🈳").await?;
        return Ok(());
    } else if guild_data.playlist.len() <= 10 {
        // directly display if the playlist has less than 10 items
        let response = guild_data
            .playlist
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
        drop(guild_data);

        msg.reply(ctx, response).await?;
        return Ok(());
    } else {
        // show the select menu if the playlist has more than 10 items (discord text message has a length limitation of 2000 unicode chars)
        let mut select_components = CreateComponents::default();
        select_components.create_action_row(|r| {
            r.create_select_menu(|menu| {
                menu.custom_id("page_select")
                    .placeholder("📖❓")
                    .options(|options| {
                        let page_num = (guild_data.playlist.len() / 10) + 1;
                        for i in 1..=page_num {
                            options.create_option(|opt| {
                                opt.label("📄".to_string() + &i.to_emoji())
                                    .value(i.to_string())
                            });
                        }
                        options
                    })
            })
        });
        drop(guild_data);

        let select_msg = msg
            .channel_id
            .send_message(ctx, |m| {
                m.reference_message(msg).set_components(select_components)
            })
            .await?;
        waiting = Some(select_msg)
    }

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
            let guild_data = guild_data_map.entry(msg.guild_id.unwrap()).or_default();
            let response = guild_data
                .playlist
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
                .to_owned();
            drop(guild_data);

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
