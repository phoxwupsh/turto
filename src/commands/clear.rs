use crate::{
    messages::{
        TurtoMessage,
        TurtoMessageKind::RemoveAll,
    },
    models::alias::{Context, Error},
};

#[poise::command(slash_command, prefix_command, guild_only)]
pub async fn clear(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();
    let mut guild_data = ctx.data().guilds.entry(guild_id).or_default();
    let locale = ctx.locale();

    guild_data.playlist.clear();
    drop(guild_data);

    ctx.say(TurtoMessage {
        locale,
        kind: RemoveAll,
    })
    .await?;

    Ok(())
}