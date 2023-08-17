use crate::{
    guild::{playing::Playing, setting::Settings},
    models::{setting::GuildSetting, volume::GuildVolume},
    utils::i32_to_emoji,
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
    let new_vol_u32 = match args.parse::<u32>() {
        Ok(vol_u32) => vol_u32,
        Err(_) => {
            msg.reply(ctx, "enter a number 0 ~ 100").await?;
            return Ok(());
        }
    };
    let new_vol = match GuildVolume::try_from(new_vol_u32) {
        Ok(vol) => vol,
        Err(_) => {
            msg.reply(ctx, "enter a number 0 ~ 100").await?;
            return Ok(());
        }
    };

    // Update the volume if there is a currently playing TrackHandle
    let playing_lock = ctx
        .data
        .read()
        .await
        .get::<Playing>()
        .expect("Expected Playing in TypeMap")
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
        .get::<Settings>()
        .expect("Expected Playing in TypeMap")
        .clone();
    {
        let mut settings = settings_lock.lock().await;
        let setting = settings
            .entry(msg.guild_id.unwrap())
            .or_insert_with(GuildSetting::default);
        setting.volume = new_vol;
    }

    msg.reply(ctx, format!("🔊{}", i32_to_emoji(new_vol.into())))
        .await?;

    Ok(())
}
