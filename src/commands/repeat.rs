use crate::{
    message::TurtoMessageKind::SetRepeat,
    models::{alias::Context, error::CommandError, toggle::ToggleOption},
    turto_command,
    utils::turto_say,
};
use tracing::{Span, instrument};

#[turto_command(
    short = "Toggle repeating",
    long = "Enable (`on`) or disable (`off`) repeating. When repeating is enabled, turto will repeatly playing the currently playing item."
)]
#[poise::command(slash_command, guild_only)]
#[instrument(
    name = "repeat",
    skip_all,
    parent = ctx.invocation_data::<Span>().await.as_deref().unwrap_or(&Span::none())
    fields(%toggle)
)]
pub async fn repeat(
    ctx: Context<'_>,
    #[description = "Can be`on` or `off`, to toggle repeat function"] toggle: ToggleOption,
) -> Result<(), CommandError> {
    tracing::info!("invoked");

    let toggle = match toggle {
        ToggleOption::On => true,
        ToggleOption::Off => false,
    };

    let guild_id = ctx.guild_id().ok_or(CommandError::GuildOnly)?;
    let mut guild_data = ctx.data().guilds.entry(guild_id).or_default();
    guild_data.config.repeat = toggle;
    drop(guild_data);

    turto_say(ctx, SetRepeat(toggle)).await?;
    Ok(())
}
