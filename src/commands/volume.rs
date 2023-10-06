use crate::{
    messages::TurtoMessage,
    models::guild::volume::GuildVolume,
    typemap::{guild_data::GuildDataMap, playing::Playing},
};
use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::Message,
    prelude::Context,
};
use tracing::error;

#[command]
#[bucket = "turto"]
async fn volume(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    if args.rest() == "?" {
        let curr_vol = ctx
            .data
            .read()
            .await
            .get::<GuildDataMap>()
            .unwrap()
            .clone()
            .entry(msg.guild_id.unwrap())
            .or_default()
            .config
            .volume;
        msg.reply(ctx, TurtoMessage::SetVolume(Ok(curr_vol)))
            .await?;
        return Ok(());
    }
    let new_vol_u32 = match args.parse::<usize>() {
        Ok(vol_u32) => vol_u32,
        Err(_) => {
            msg.reply(ctx, TurtoMessage::SetVolume(Err(()))).await?;
            return Ok(());
        }
    };
    let new_vol = match GuildVolume::try_from(new_vol_u32) {
        Ok(vol) => vol,
        Err(_) => {
            msg.reply(ctx, TurtoMessage::SetVolume(Err(()))).await?;
            return Ok(());
        }
    };

    // Update the volume if there is a currently playing TrackHandle
    let playing_lock = ctx.data.read().await.get::<Playing>().unwrap().clone();
    {
        let playing = playing_lock.read().await;
        if let Some(current_track) = playing.get(&msg.guild_id.unwrap()) {
            if let Err(why) = current_track.set_volume(*new_vol) {
                error!(
                    "Failed to set volume for track {}: {}",
                    current_track.uuid(),
                    why
                );
            }
        }
    }

    // Update the volume setting of guild
    let guild_data_map = ctx.data.read().await.get::<GuildDataMap>().unwrap().clone();
    let mut guild_data = guild_data_map.entry(msg.guild_id.unwrap()).or_default();
    guild_data.config.volume = new_vol;
    drop(guild_data);

    msg.reply(ctx, TurtoMessage::SetVolume(Ok(new_vol))).await?;

    Ok(())
}
