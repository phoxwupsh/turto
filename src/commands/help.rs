use crate::{
    commands::CommandKind,
    models::{alias::Context, error::CommandError},
};
use poise::CreateReply;
use tracing::{Span, instrument};

#[poise::command(slash_command, guild_only)]
#[instrument(
    name = "help",
    skip_all,
    parent = ctx.invocation_data::<Span>().await.as_deref().unwrap_or(&Span::none())
    fields(%command)
)]
pub async fn help(ctx: Context<'_>, command: CommandKind) -> Result<(), CommandError> {
    tracing::info!("invoked");

    let helps = &ctx.data().help;
    let command_help = helps.view_locale_command_with_fallback(ctx.locale(), command);
    let embed = command_help.create_embed();

    let response = CreateReply::default().embed(embed);

    ctx.send(response).await?;
    Ok(())
}
