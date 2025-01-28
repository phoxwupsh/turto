use crate::{
    messages::TurtoMessageKind::SetVolume,
    models::{
        alias::{Context, Error},
        guild::volume::GuildVolume,
    },
    utils::turto_say,
};
use tracing::error;

#[poise::command(slash_command, guild_only)]
pub async fn volume(
    ctx: Context<'_>,
    #[min = 0]
    #[max = 100]
    value: Option<usize>,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();

    if let Some(vol) = value {
        // Update the volume if there is a currently playing TrackHandle
        let new_vol = GuildVolume::try_from(vol).unwrap();
        let playing_map = ctx.data().playing.read().await;
        if let Some(playing) = playing_map.get(&guild_id) {
            if let Err(why) = playing.track_handle.set_volume(*new_vol) {
                let uuid = playing.track_handle.uuid();
                error!("Failed to set volume for track {uuid}: {why}");
            }
        }
        drop(playing_map);

        // Update the volume setting of guild
        let mut guild_data = ctx.data().guilds.entry(guild_id).or_default();
        guild_data.config.volume = new_vol;
        drop(guild_data);

        turto_say(ctx, SetVolume(new_vol)).await?;
        Ok(())
    } else {
        let curr_vol = ctx.data().guilds.entry(guild_id).or_default().config.volume;
        turto_say(ctx, SetVolume(curr_vol)).await?;
        Ok(())
    }
}
