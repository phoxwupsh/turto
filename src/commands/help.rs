use crate::{
    models::{alias::Context, command::CommandKind, error::CommandError},
    turto_command,
};
use poise::CreateReply;
use tracing::{Span, instrument};

// `/help help` is not a thing, so `help` is not offered as a choice of itself.
#[turto_command(
    short = "Look up how to use each command",
    long = "Look up how to use each command, `command` is the command to look up.",
    hide_in_help
)]
#[poise::command(slash_command, guild_only)]
#[instrument(
    name = "help",
    skip_all,
    parent = ctx.invocation_data::<Span>().await.as_deref().unwrap_or(&Span::none())
    fields(%command)
)]
pub async fn help(
    ctx: Context<'_>,
    #[description = "The command to look up"] command: CommandKind,
) -> Result<(), CommandError> {
    tracing::info!("invoked");

    let command_help = ctx.data().help.resolve(ctx.locale(), command);
    let embed = command_help.create_embed();

    let response = CreateReply::default().embed(embed);

    ctx.send(response).await?;
    Ok(())
}
