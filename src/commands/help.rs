use std::{time::Duration, sync::Arc};

use serenity::{
    framework::standard::{macros::command, CommandResult},
    model::{
        application::interaction::InteractionResponseType,
        prelude::{Message, interaction::message_component::MessageComponentInteraction},
    },
    prelude::Context, futures::StreamExt,
};

use crate::{models::help::Help, messages::TurtoMessage};

#[command]
async fn help(ctx: &Context, msg: &Message) -> CommandResult {
    let helps = Help::get_helps();
    let command_list = Help::get_command_list();
    let waiting = msg
        .channel_id
        .send_message(ctx, |message| {
            message
                .reference_message(msg)
                .content(TurtoMessage::Help)
                .components(|components| {
                    components.create_action_row(|row| {
                        row.create_select_menu(|menu| {
                            menu.custom_id("help")
                                .placeholder(&helps.placeholder)
                                .options(|options| {
                                    for command_name in command_list.iter() {
                                        options.create_option(|o| o.label(command_name).value(command_name));
                                    }
                                    options
                                })
                        })
                    })
                })
        })
        .await?;

    let mut interactions = waiting
        .await_component_interactions(ctx)
        .timeout(Duration::from_secs(60))
        .build();
    
    let interaction = {
        let res: Arc<MessageComponentInteraction>;
        loop {
            match interactions.next().await {
                Some(interaction) => {
                    if interaction.user == msg.author { // response only if the interaction is send by the user who invoke the help command
                        res = interaction;
                        break;
                    }
                },
                None => { // if there is no interaction sended, delete the select menu
                    waiting.delete(ctx).await?;
                    return Ok(());
                }
            }
        }
        res
    };

    let command_name = &interaction.data.values[0];
    let target_help = helps
        .command_helps
        .get(command_name)
        .expect("Error loading command help");

    interaction
        .create_interaction_response(ctx, |resp| {
            resp.kind(InteractionResponseType::UpdateMessage)
                .interaction_response_data(|data| {
                    data.content(TurtoMessage::CommandHelp { command_name })
                        .components(|components| components)
                        .embed(|embed| {
                            embed
                                .title(command_name)
                                .description(&target_help.description)
                                .field(&helps.usage_field, &target_help.usage, true)
                                .field(&helps.example_field, &target_help.example, true)
                        })
                })
        })
        .await?;

    Ok(())
}
