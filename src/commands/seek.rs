use std::time::Duration;

use serenity::{framework::standard::{macros::command, Args, CommandResult}, prelude::Context, model::prelude::Message};
use songbird::tracks::PlayMode;

use crate::{guild::playing::Playing, messages::NOT_PLAYING};

#[command]
#[description = "如果目前有正在播放或暫停中的的項目，跳轉到第`秒數`秒，`秒數`應該要是一個大於0的數字。"]
#[usage = "秒數"]
#[example = "42"]
async fn seek(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let sec = match args.parse::<u64>() {
        Ok(s) => s,
        Err(_) => {
            msg.reply(ctx, "enter a number").await?;
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

            if let Ok(track_state) = current_track.get_info().await {
                if track_state.playing == PlayMode::Stop || track_state.playing == PlayMode::End {
                    msg.reply(ctx, NOT_PLAYING).await?;
                    return Ok(())
                }
            }

            let track_sec = current_track.metadata().duration.unwrap().as_secs();
            if track_sec < sec {
                msg.reply(ctx, format!("Not long enough, {} is only {} seconds long.", current_track.metadata().title.clone().unwrap(), track_sec)).await?;
                return Ok(())
            }

            match current_track.seek_time(Duration::from_secs(sec)) {
                Ok(_) => (),
                Err(why) => println!("{:?}", why)
            }
        }
    }

    Ok(())
}