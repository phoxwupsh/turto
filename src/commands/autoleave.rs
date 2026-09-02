use crate::{
    message::TurtoMessageKind::SetAutoleave,
    models::{alias::Context, autoleave::AutoleaveType, error::CommandError},
    turto_command,
    utils::turto_say,
};
use tracing::{Span, instrument};

#[turto_command(
    short = "Toggle automatic leaving.",
    long = "Enable (`on`, `empty`, `silent`) or disable (`off`) automatic leaving. When automatic leaving is enabled, turto will leave the voice channel automatically when the playlist is empty after playback ends or is stopped.\n\
            - `on`: turto will leave when nothing is playing or no one is in the voice channel\n\
            - `empty`: turto will leave when no one is in the voice channel\n\
            - `silent`: turto will leave when no nothing is playing\n\
            - `off`: turto won't leave automatically"
)]
#[poise::command(slash_command, guild_only)]
#[instrument(
    name = "autoleave",
    skip_all,
    parent = ctx.invocation_data::<Span>().await.as_deref().unwrap_or(&Span::none())
    fields(%toggle)
)]
pub async fn autoleave(
    ctx: Context<'_>,
    #[description = "Toggle autoleave, refer to help command for usage"] toggle: AutoleaveType,
) -> Result<(), CommandError> {
    tracing::info!("invoked");
    let guild_id = ctx.guild_id().ok_or(CommandError::GuildOnly)?;
    let mut guild_data = ctx.data().guilds.entry(guild_id).or_default();
    guild_data.config.auto_leave = toggle;
    drop(guild_data);

    turto_say(ctx, SetAutoleave(toggle)).await?;
    Ok(())
}
