use crate::{
    message::TurtoMessageKind::SetAutoleave,
    models::{alias::Context, autoleave::AutoleaveType, error::CommandError},
    utils::turto_say,
};
use tracing::{Span, instrument};

#[poise::command(slash_command, guild_only)]
#[instrument(
    name = "autoleave",
    skip_all,
    parent = ctx.invocation_data::<Span>().await.as_deref().unwrap_or(&Span::none())
    fields(%toggle)
)]
pub async fn autoleave(ctx: Context<'_>, toggle: AutoleaveType) -> Result<(), CommandError> {
    tracing::info!("invoked");
    let guild_id = ctx.guild_id().ok_or(CommandError::GuildOnly)?;
    let mut guild_data = ctx.data().guilds.entry(guild_id).or_default();
    guild_data.config.auto_leave = toggle;
    drop(guild_data);

    turto_say(ctx, SetAutoleave(toggle)).await?;
    Ok(())
}
