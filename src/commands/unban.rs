use crate::{
    message::TurtoMessageKind::{AdministratorOnly, Unban},
    models::alias::{Context, Error},
    utils::turto_say,
};
use serenity::all::UserId;

#[poise::command(slash_command, guild_only)]
pub async fn unban(ctx: Context<'_>, user: UserId) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();
    let user_id = ctx.author().id;

    // Since this is a guild only command interaction
    let is_admin = ctx
        .author_member()
        .await
        .unwrap()
        .permissions
        .unwrap()
        .administrator();

    if !(is_admin || ctx.data().config.is_owner(&user_id)) {
        turto_say(ctx, AdministratorOnly).await?;
        return Ok(());
    }

    let mut guild_data = ctx.data().guilds.entry(guild_id).or_default();
    let success = guild_data.config.banned.remove(&user);
    drop(guild_data);

    turto_say(ctx, Unban { success, user }).await?;
    Ok(())
}
