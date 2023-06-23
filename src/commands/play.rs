use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::Message,
    prelude::{Context, Mentionable},
};
use songbird::tracks::PlayMode;
use url::Url;

use crate::{
    guild::playing::Playing,
    utils::{
        bot_in_voice_channel, play_next, play_song, same_voice_channel, user_in_voice_channel,
    },
};

use tracing::error;

#[command]
#[description = "開始播放，如果turto沒有在其他語音頻道的話就會進入你所在的語音頻道，依照狀況不同有以下幾種可能：\n**1** 有輸入`網址`的話，會停止目前正在播放的項目(如果有的話)，並開始播放`網址`，`網址`目前只支援YouTube的影片(直播不行)。。\n**2** 如果沒有輸入網址，且當目前有正在播放的項目被暫停時，會繼續播放該項目。\n**3** 如果沒有輸入網址，目前也沒有暫停的項目，會開始播放播放清單。"]
#[usage = "網址"]
#[example = ""]
#[example = "https://youtu.be/dQw4w9WgXcQ"]
#[example = "https://www.youtube.com/watch?v=dQw4w9WgXcQ"]
async fn play(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let guild = msg.guild(&ctx.cache).unwrap();
    let guild_id = guild.id;

    // Get the Songbird instance
    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialization.");

    // Check if the user is in a voice channel
    let user_voice_channel_id = match user_in_voice_channel(ctx, msg).await {
        Some(channel_id) => channel_id,
        None => {
            msg.reply(ctx, "You are not in a voice channel").await?;
            return Ok(());
        }
    };

    // Check if the bot is in a voice channel or not, if not join the voice channel
    if let Some(current_bot_voice_channel) = bot_in_voice_channel(ctx, msg).await {
        if !same_voice_channel(ctx, msg).await {
            // Notify th user if they are in different voice channel
            msg.reply(
                ctx,
                format!("I'm currently in {}.", current_bot_voice_channel.mention()),
            )
            .await?;
            return Ok(());
        }
    } else {
        let (_handler_lock, success) = manager.join(guild_id, user_voice_channel_id).await;
        if let Ok(_) = success {
            msg.channel_id
                .say(ctx, format!("🐢{}", user_voice_channel_id.mention()))
                .await?;
        }
    }

    let url = args.rest().to_string();

    // Check if url is provided
    if !url.is_empty() {
        // Validate the URL
        if !Url::parse(&url).is_ok() {
            msg.reply(ctx, "You must provide a valid YouTube URL.")
                .await?;
            return Ok(());
        }

        let meta = play_song(ctx, guild_id, url).await?;

        // Inform the user about the song being played
        msg.reply(ctx, format!("▶️ {}", meta.title.unwrap())).await?;
    } else {
        // If no url provided, check if there is a paused track or there is any song in the playlist
        let playing_lock = {
            let data_read = ctx.data.read().await;
            data_read
                .get::<Playing>()
                .expect("Expected Playing in TypeMap")
                .clone()
        };
        {
            let playing = playing_lock.read().await;
            // Get the current track handle

            if let Some(current_track) = playing.get(&guild_id) {
                if let Ok(current_track_state) = current_track.get_info().await {
                    if current_track_state.playing == PlayMode::Pause {
                        if let Err(why) = current_track.play() {
                            error!("Error playing song: {:?}", why);
                            return Ok(());
                        }
                        return Ok(()); // If there is a paused song then play it
                    }
                }
            } // return the lock
        }

        if let Ok(meta) = play_next(ctx, guild_id).await {
            // if there is any song in the play list
            msg.reply(ctx, format!("▶️ {}", meta.title.unwrap())).await?;
        } else {
            // if the playlist is empty
            msg.reply(ctx, "You have to provide a url.").await?;
        }
    }
    Ok(())
}
