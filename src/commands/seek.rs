use std::time::Duration;

use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::Message,
    prelude::{Context, Mentionable},
};
use songbird::tracks::PlayMode;
use tracing::error;

use crate::{guild::playing::Playing, messages::NOT_PLAYING, utils::guild::{GuildUtil, VoiceChannelState}};

#[command]
#[bucket = "music"]
async fn seek(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let guild = msg.guild(ctx).unwrap();

    match guild.cmp_voice_channel(&ctx.cache.current_user_id(), &msg.author.id) {
        VoiceChannelState::Different(bot_vc, _) | VoiceChannelState::OnlyFirst(bot_vc) => {
            msg.reply(ctx, format!("You are not in {}", bot_vc.mention())).await?;
            return Ok(());
        },
        VoiceChannelState::OnlySecond(_) | VoiceChannelState::None => {
            msg.reply(ctx, "Currently not in a voice channel").await?;
            return Ok(());
        },
        VoiceChannelState::Same(_) => ()
    }

    let sec = match args.parse::<u64>() {
        Ok(s) => s,
        Err(_) => {
            msg.reply(ctx, "enter a number").await?;
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
            if let Ok(track_state) = current_track.get_info().await {
                if track_state.playing == PlayMode::Stop || track_state.playing == PlayMode::End {
                    msg.reply(ctx, NOT_PLAYING).await?;
                    return Ok(());
                }
                if track_state.position.as_secs() + 600 < sec {
                    // Seeking is expensive, currently set the seek limitation to 10 mins
                    msg.reply(ctx, "Too long to seek.").await?;
                    return Ok(());
                }
                if track_state.position.as_secs() > sec {
                    // Backward seeking is more expensive so currently disable it
                    msg.reply(ctx, "Backward is not support").await?;
                    return Ok(());
                }
            }

            let track_sec = current_track.metadata().duration.unwrap().as_secs();
            if track_sec < sec {
                msg.reply(
                    ctx,
                    format!(
                        "Not long enough, {} is only {} seconds long.",
                        current_track.metadata().title.clone().unwrap(),
                        track_sec
                    ),
                )
                .await?;
                return Ok(());
            }

            if let Err(why) = current_track.seek_time(Duration::from_secs(sec)) {
                error!("Error seeking track {}: {:?}", current_track.uuid(), why);
            }
        }
    }

    Ok(())
}
