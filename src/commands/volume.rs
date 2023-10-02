use crate::{
    typemap::{playing::Playing, config::GuildConfigs},
    models::guild::volume::GuildVolume,
    messages::TurtoMessage,
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
            .get::<GuildConfigs>()
            .unwrap()
            .clone()
            .lock()
            .await
            .entry(msg.guild_id.unwrap())
            .or_default()
            .volume;
        msg.reply(ctx, TurtoMessage::SetVolume(Ok(curr_vol))).await?;
        return Ok(())
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
    let playing_lock = ctx
        .data
        .read()
        .await
        .get::<Playing>()
        .unwrap()
        .clone();
    {
        let playing = playing_lock.read().await;
        if let Some(current_track) = playing.get(&msg.guild_id.unwrap()) {
            if let Err(why) = current_track.set_volume(*new_vol) {
                error!(
                    "Error setting volume for track {}: {}",
                    current_track.uuid(),
                    why
                );
            }
        }
    }

    // Update the volume setting of guild
    let guild_configs_lock = ctx
        .data
        .read()
        .await
        .get::<GuildConfigs>()
        .unwrap()
        .clone();
    {
        let mut guild_configs = guild_configs_lock.lock().await;
        let guild_config = guild_configs
            .entry(msg.guild_id.unwrap())
            .or_default();
        guild_config.volume = new_vol;
    }

    msg.reply(ctx, TurtoMessage::SetVolume(Ok(new_vol))).await?;

    Ok(())
}
