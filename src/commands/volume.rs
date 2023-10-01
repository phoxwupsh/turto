use crate::{
    typemap::{playing::Playing, config::GuildConfigs},
    models::guild::{config::GuildConfig, volume::GuildVolume},
    messages::TurtoMessage,
};
use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::Message,
    prelude::Context,
};
use tracing::error;

#[command]
#[bucket = "music"]
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
            .or_insert_with(GuildConfig::default)
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
                    "Error setting volume for track {}: {:?}",
                    current_track.uuid(),
                    why
                );
            }
        }
    }

    // Update the volume setting of guild
    let settings_lock = ctx
        .data
        .read()
        .await
        .get::<GuildConfigs>()
        .unwrap()
        .clone();
    {
        let mut settings = settings_lock.lock().await;
        let setting = settings
            .entry(msg.guild_id.unwrap())
            .or_insert_with(GuildConfig::default);
        setting.volume = new_vol;
    }

    msg.reply(ctx, TurtoMessage::SetVolume(Ok(new_vol))).await?;

    Ok(())
}
