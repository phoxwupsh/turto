use crate::{
    config::help::{get_command_list, get_help},
    messages::TurtoMessage,
};
use serenity::{
    all::ComponentInteractionDataKind,
    builder::{
        CreateActionRow, CreateEmbed, CreateInteractionResponse, CreateInteractionResponseMessage,
        CreateMessage, CreateSelectMenu, CreateSelectMenuKind, CreateSelectMenuOption,
    },
    framework::standard::{macros::command, CommandResult},
    futures::StreamExt,
    model::prelude::Message,
    prelude::Context,
};
use std::time::Duration;

#[command]
#[bucket = "turto"]
async fn help(ctx: &Context, msg: &Message) -> CommandResult {
    let helps = get_help();
    let options = get_command_list()
        .iter()
        .map(|command_name| CreateSelectMenuOption::new(command_name, command_name))
        .collect::<Vec<_>>();
    let acction = CreateActionRow::SelectMenu(
        CreateSelectMenu::new("help", CreateSelectMenuKind::String { options })
            .placeholder(&helps.placeholder),
    );
    let help_message = CreateMessage::new()
        .reference_message(msg)
        .content(TurtoMessage::Help)
        .components(vec![acction]);
    let waiting = msg.channel_id.send_message(ctx, help_message).await?;

    let mut interactions = waiting
        .await_component_interactions(ctx)
        .timeout(Duration::from_secs(60))
        .stream();

    let interaction = loop {
        match interactions.next().await {
            Some(interaction) => {
                if interaction.user == msg.author {
                    // response only if the interaction is send by the user who invoke the help command
                    break interaction;
                }
            }
            None => {
                // if there is no interaction sended, delete the select menu
                waiting.delete(ctx).await?;
                return Ok(());
            }
        }
    };

    let command_name = match &interaction.data.kind {
        ComponentInteractionDataKind::StringSelect { values } => values.get(0).unwrap(),
        _ => panic!("Invalid help select"),
    };
    let target_help = helps.commands.get(command_name).unwrap_or_else(|| {
        panic!(
            "Can't find help configuration of the command \"{}\" in help.toml",
            command_name
        )
    });

    let response = CreateInteractionResponse::UpdateMessage(
        CreateInteractionResponseMessage::new()
            .content(TurtoMessage::CommandHelp {
                command_name: command_name.as_str(),
            })
            .components(vec![])
            .embed(
                CreateEmbed::new()
                    .title(command_name)
                    .description(&target_help.description)
                    .field(&helps.usage_field, &target_help.usage, true)
                    .field(&helps.example_field, &target_help.example, true),
            ),
    );

    interaction.create_response(ctx, response).await?;

    Ok(())
}
