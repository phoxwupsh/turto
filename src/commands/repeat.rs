use crate::{
    message::TurtoMessageKind::SetRepeat,
    models::{
        alias::{Context, Error},
        toggle::ToggleOption,
    },
    utils::turto_say,
};

#[poise::command(slash_command, guild_only)]
pub async fn repeat(ctx: Context<'_>, toggle: ToggleOption) -> Result<(), Error> {
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
