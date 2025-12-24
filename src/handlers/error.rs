use std::pin::Pin;

use poise::{CreateReply, FrameworkError};
use serenity::all::{CreateAllowedMentions, CreateEmbed};

use crate::models::{data::Data, error::CommandError};

pub fn on_error<'a>(
    error: FrameworkError<'a, Data, CommandError>,
) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
    Box::pin(async move {
        if let Err(err) = handle_error(error).await {
            tracing::error!(error = ?err, "error occurred while handling error");
        }
    })
}

async fn handle_error(
    error: FrameworkError<'_, Data, CommandError>,
) -> Result<(), serenity::Error> {
    match error {
        FrameworkError::Setup { error, .. } => {
            tracing::error!(?error, "failed to setup user data");
        }
        FrameworkError::EventHandler { error, event, .. } => {
            tracing::error!(?error, ?event, "failed to handle event")
        }
        FrameworkError::Command { ctx, error, .. } => {
            let command = ctx.invocation_string();
            let user = ctx.author().id;
            tracing::error!(?error, command, %user, "error occured in command");

            let mentions = CreateAllowedMentions::new()
                .everyone(false)
                .all_roles(false)
                .all_users(false);

            let resp = CreateEmbed::new()
                .title("Internal error")
                .color((255, 0, 0))
                .field("Type", error.cause(), false)
                .description(error.to_string());

            ctx.send(
                CreateReply::default()
                    .embed(resp)
                    .allowed_mentions(mentions),
            )
            .await?;
        }
        FrameworkError::SubcommandRequired { ctx } => {
            let subcommands = ctx
                .command()
                .subcommands
                .iter()
                .map(|s| s.name.as_str())
                .collect::<Vec<_>>();
            let response = format!(
                "You must specify one of the following subcommands: {}",
                subcommands.join(", ")
            );
            ctx.send(CreateReply::default().content(response).ephemeral(true))
                .await?;
        }
        FrameworkError::CommandPanic {
            ctx, payload, ..
        } => {
            tracing::error!(payload, "command panic");
            // Not showing the payload to the user because it may contain sensitive info
            let embed = CreateEmbed::default()
                .title("Internal error")
                .color((255, 0, 0))
                .field("Type", "panic", false)
                .description("panic while processing command");

            ctx.send(CreateReply::default().embed(embed).ephemeral(true))
                .await?;
        }
        FrameworkError::ArgumentParse {
            ctx, input, error, ..
        } => {
            let mut resp = CreateEmbed::new()
                .title("Invalid input")
                .description(error.to_string())
                .color((255, 0, 0));
            if let Some(input) = input {
                resp = resp.field("Your input", input, false);
            }

            let mentions = CreateAllowedMentions::new()
                .everyone(false)
                .all_roles(false)
                .all_users(false);

            ctx.send(
                CreateReply::default()
                    .embed(resp)
                    .allowed_mentions(mentions),
            )
            .await?;
        }
        FrameworkError::CommandStructureMismatch {
            ctx, description, ..
        } => {
            tracing::error!(
                command = ctx.command.name,
                description,
                "failed to deserialize interaction arguments",
            );
        }
        FrameworkError::CommandCheckFailed { ctx, error, .. } => {
            tracing::error!(
                ?error,
                command = ctx.invoked_command_name(),
                "command check failed",
            );
        }
        FrameworkError::CooldownHit {
            remaining_cooldown,
            ctx,
            ..
        } => {
            let resp = CreateEmbed::new()
                .title("You are too fast")
                .description("please wait and retry")
                .color((255, 0, 0))
                .field(
                    "Time remaining",
                    format!("{} seconds", remaining_cooldown.as_secs()),
                    false,
                );

            ctx.send(CreateReply::default().embed(resp).ephemeral(true))
                .await?;
        }
        FrameworkError::MissingBotPermissions {
            missing_permissions,
            ctx,
            ..
        } => {
            let resp = CreateEmbed::new()
                .title("Lacking permissions")
                .description("bot is lacking permissions for executing the command")
                .field("Permissions", missing_permissions.to_string(), false)
                .color((255, 0, 0));
            ctx.send(CreateReply::default().embed(resp).ephemeral(true))
                .await?;
        }
        FrameworkError::MissingUserPermissions {
            missing_permissions,
            ctx,
            ..
        } => {
            let mut resp = CreateEmbed::new()
                .title("Lacking permissions")
                .description("you are lacking permissions for the command")
                .color((255, 0, 0));

            if let Some(perm) = missing_permissions {
                resp = resp.field("Permissions", perm.to_string(), false);
            }

            ctx.send(CreateReply::default().embed(resp).ephemeral(true))
                .await?;
        }
        FrameworkError::NotAnOwner { ctx, .. } => {
            let response = "Only bot owners can call this command";
            ctx.send(CreateReply::default().content(response).ephemeral(true))
                .await?;
        }
        FrameworkError::GuildOnly { ctx, .. } => {
            let response = "You cannot run this command in DMs.";
            ctx.send(CreateReply::default().content(response).ephemeral(true))
                .await?;
        }
        FrameworkError::DmOnly { ctx, .. } => {
            let response = "You cannot run this command outside DMs.";
            ctx.send(CreateReply::default().content(response).ephemeral(true))
                .await?;
        }
        FrameworkError::NsfwOnly { ctx, .. } => {
            let response = "You cannot run this command outside NSFW channels.";
            ctx.send(CreateReply::default().content(response).ephemeral(true))
                .await?;
        }
        // these two error should be unreachable since we don't use prefix commands
        // if this really happened it means there's some problem in our code
        FrameworkError::DynamicPrefix { .. } => unreachable!(),
        FrameworkError::UnknownCommand { .. } => unreachable!(),

        FrameworkError::UnknownInteraction { interaction, .. } => {
            tracing::warn!(?interaction, "received unknown interaction");
        }
        FrameworkError::NonCommandMessage { error, .. } => {
            tracing::warn!(?error, "error in non-command message handler");
        }
        FrameworkError::__NonExhaustive(unreachable) => match unreachable {},
    }
    Ok(())
}
