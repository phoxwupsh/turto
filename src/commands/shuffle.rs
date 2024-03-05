use crate::{
    messages::{
        TurtoMessage,
        TurtoMessageKind::{EmptyPlaylist, Shuffle},
    },
    models::alias::{Context, Error},
};
use rand::{seq::SliceRandom, thread_rng};

#[poise::command(slash_command, guild_only)]
pub async fn shuffle(ctx: Context<'_>) -> Result<(), Error> {
    let guild = ctx.guild_id().unwrap();
    let mut guild_data = ctx.data().guilds.entry(guild).or_default();
    let locale = ctx.locale();
    if guild_data.playlist.is_empty() {
        drop(guild_data);
        ctx.say(TurtoMessage {
            locale,
            kind: EmptyPlaylist,
        })
        .await?;
        return Ok(());
    }
    let playlist = guild_data.playlist.make_contiguous();
    playlist.shuffle(&mut thread_rng());
    drop(guild_data);

    ctx.say(TurtoMessage {
        locale,
        kind: Shuffle,
    })
    .await?;
    Ok(())
}
