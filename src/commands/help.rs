use std::time::Duration;

use serenity::{
    framework::standard::{macros::command, CommandResult},
    model::{
        application::interaction::InteractionResponseType,
        prelude::Message,
    },
    prelude::Context,
};

use crate::models::help::HELPS;

#[command]
async fn help(ctx: &Context, msg: &Message) -> CommandResult {
    let waiting = msg
        .channel_id
        .send_message(ctx, |message| {
            message
                .reference_message(msg)
                .content(&HELPS.help_msg)
                .components(|components| {
                    components.create_action_row(|row| {
                        row.create_select_menu(|menu| {
                            menu.custom_id("help")
                                .placeholder(&HELPS.placeholder)
                                .options(|options| {
                                    for k in HELPS.command_helps.keys() {
                                        options.create_option(|o| o.label(k).value(k));
                                    }
                                    options
                                })
                        })
                    })
                })
        })
        .await?;

    let interaction = match waiting
        .await_component_interaction(ctx)
        .timeout(Duration::from_secs(60 * 3))
        .await
    {
        Some(x) => x,
        None => {
            msg.reply(ctx, "Timed out").await?;
            return Ok(());
        }
    };

    let command_name = &interaction.data.values[0];
    let target_help = HELPS
        .command_helps
        .get(command_name)
        .expect("Error loading command help");

    interaction
        .create_interaction_response(ctx, |resp| {
            resp.kind(InteractionResponseType::UpdateMessage)
                .interaction_response_data(|data| {
                    data.content(&target_help.help_msg)
                        .components(|components| components)
                        .embed(|embed| {
                            embed
                                .title(target_help.command_name.clone())
                                .description(target_help.description.clone())
                                .field(&HELPS.usage_field, target_help.usage.clone(), true)
                                .field(&HELPS.example_field, target_help.example.join("\n"), true)
                        })
                })
        })
        .await?;

    Ok(())
}
