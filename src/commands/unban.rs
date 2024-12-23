use crate::{
    config::get_config,
    messages::{
        TurtoMessage,
        TurtoMessageKind::{AdministratorOnly, Unban},
    },
    models::alias::{Context, Error},
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
    let locale = ctx.locale();

    if !(is_admin || get_config().is_owner(&user_id)) {
        ctx.say(TurtoMessage {
            locale,
            kind: AdministratorOnly,
        })
        .await?;
        return Ok(());
    }

    let mut guild_data = ctx.data().guilds.entry(guild_id).or_default();
    let success = guild_data.config.banned.remove(&user);
    drop(guild_data);

    ctx.say(TurtoMessage {
        locale,
        kind: Unban { success, user },
    })
    .await?;

    Ok(())
}
