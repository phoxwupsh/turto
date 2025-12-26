use tracing::{Span, instrument};
use crate::{
    message::TurtoMessageKind::SetVolume,
    models::{alias::Context, error::CommandError, guild::volume::GuildVolume},
    utils::turto_say,
};

#[poise::command(slash_command, guild_only)]
#[instrument(
    name = "volume",
    skip_all,
    parent = ctx.invocation_data::<Span>().await.as_deref().unwrap_or(&Span::none())
    fields(value)
)]
pub async fn volume(
    ctx: Context<'_>,
    #[min = 0]
    #[max = 100]
    value: Option<usize>,
) -> Result<(), CommandError> {
    tracing::info!("invoked");

    let guild_id = ctx.guild_id().unwrap();

    if let Some(vol) = value {
        // Update the volume if there is a currently playing TrackHandle
        let new_vol = GuildVolume::try_from(vol).unwrap();
        let playing_map = ctx.data().playing.read().await;
        if let Some(playing) = playing_map.get(&guild_id)
        {
            playing.track_handle.set_volume(*new_vol)?;
        }
        drop(playing_map);

        // Update the volume setting of guild
        let mut guild_data = ctx.data().guilds.entry(guild_id).or_default();
        guild_data.config.volume = new_vol;
        drop(guild_data);

        tracing::info!("set volume success");

        turto_say(ctx, SetVolume(new_vol)).await?;
        Ok(())
    } else {
        let curr_vol = ctx.data().guilds.entry(guild_id).or_default().config.volume;
        turto_say(ctx, SetVolume(curr_vol)).await?;
        Ok(())
    }
}
