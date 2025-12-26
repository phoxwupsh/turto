use tracing::{Span, instrument};
use crate::{
    message::TurtoMessageKind::SetRepeat,
    models::{alias::Context, error::CommandError, toggle::ToggleOption},
    utils::turto_say,
};

#[poise::command(slash_command, guild_only)]
#[instrument(
    name = "repeat",
    skip_all,
    parent = ctx.invocation_data::<Span>().await.as_deref().unwrap_or(&Span::none())
    fields(%toggle)
)]
pub async fn repeat(ctx: Context<'_>, toggle: ToggleOption) -> Result<(), CommandError> {
    tracing::info!("invoked");

    let toggle = match toggle {
        ToggleOption::On => true,
        ToggleOption::Off => false,
    };

    let mut guild_data = ctx
        .data()
        .guilds
        .entry(ctx.guild_id().unwrap())
        .or_default();
    guild_data.config.repeat = toggle;
    drop(guild_data);

    turto_say(ctx, SetRepeat(toggle)).await?;
    Ok(())
}
