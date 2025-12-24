use crate::{
    message::TurtoMessageKind::{EmptyPlaylist, Shuffle},
    models::{alias::Context, error::CommandError},
    utils::turto_say,
};
use rand::{seq::SliceRandom, thread_rng};

#[poise::command(slash_command, guild_only)]
pub async fn shuffle(ctx: Context<'_>) -> Result<(), CommandError> {
    let guild = ctx.guild_id().unwrap();
    let mut guild_data = ctx.data().guilds.entry(guild).or_default();
    if guild_data.playlist.is_empty() {
        drop(guild_data);
        turto_say(ctx, EmptyPlaylist).await?;
        return Ok(());
    }
    let playlist = guild_data.playlist.make_contiguous();
    playlist.shuffle(&mut thread_rng());
    drop(guild_data);

    turto_say(ctx, Shuffle).await?;
    Ok(())
}
