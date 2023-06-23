use serenity::{framework::standard::{macros::command, Args, CommandResult}, prelude::Context, model::prelude::Message};
use tracing::error;
use crate::{guild::{playing::Playing, volume::{Volume, GuildVolume}}, utils::{convert_to_emoji}};

#[command]
#[description = "調整音量，`音量`要界於0到100之間，整個伺服器共用同一個音量。"]
#[usage = "音量"]
#[example = "50"]
async fn volume(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let new_vol_u32 = match args.parse::<u32>() {
        Ok(vol_u32) => vol_u32,
        Err(_) => {
            msg.reply(ctx, "enter a number 0 ~ 100").await?;
            return Ok(());
        }
    };
    let new_vol = match GuildVolume::try_from(new_vol_u32) {
        Ok(vol) => vol,
        Err(_) =>{
            msg.reply(ctx, "enter a number 0 ~ 100").await?;
            return Ok(());
        }
    };

    // Update the volume if there is a currently playing TrackHandle
    let playing_lock = {
        let data_read = ctx.data.read().await;
        data_read
            .get::<Playing>()
            .expect("Expected Playing in TypeMap")
            .clone()
    };
    {
        let playing = playing_lock.read().await;
        if let Some(current_track) = playing.get(&msg.guild_id.unwrap()) {
            if let Err(why) = current_track.set_volume(*new_vol) {
                error!("Error setting volume for track {}: {:?}", current_track.uuid(), why);
            }
        }
    }

    // Update the volume setting of guild
    let volume_lock = {
        let data_read = ctx.data.read().await;
        data_read.get::<Volume>().expect("Expected Playing in TypeMap").clone()
    };
    {
        let mut volume = volume_lock.lock().await;
        let _ = volume.insert(msg.guild_id.unwrap(), new_vol);
    }

    msg.reply(ctx, format!("🔊{}", convert_to_emoji(new_vol.into()))).await?;

    Ok(())
}