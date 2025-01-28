use crate::{
    messages::TurtoMessageKind::SetAutoleave,
    models::{
        alias::{Context, Error}, autoleave::AutoleaveType
    }, utils::turto_say,
};

#[poise::command(slash_command, guild_only)]
pub async fn autoleave(ctx: Context<'_> , toggle: AutoleaveType) -> Result<(), Error> {
    let mut guild_data = ctx
        .data()
        .guilds
        .entry(ctx.guild_id().unwrap())
        .or_default();
    guild_data.config.auto_leave = toggle;
    drop(guild_data);

    turto_say(ctx, SetAutoleave(toggle)).await?;
    Ok(())
}