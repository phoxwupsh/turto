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
        if success.is_ok() {
            msg.channel_id
                .say(ctx, format!("🐢{}", user_voice_channel_id.mention()))
                .await?;
        }
    }

    let url = args.rest().to_string();

    // Check if url is provided
    if !url.is_empty() {
        // Validate the URL
        if Url::parse(&url).is_err() {
            msg.reply(ctx, "You must provide a valid YouTube URL.")
                .await?;
            return Ok(());
        }

        let meta = play_song(ctx, guild_id, url).await?;

        // Inform the user about the song being played
        msg.reply(ctx, format!("▶️ {}", meta.title.unwrap())).await?;
    } else {
        // If no url provided, check if there is a paused track or there is any song in the playlist
        let playing_lock = ctx
            .data
            .read()
            .await
            .get::<Playing>()
            .expect("Expected Playing in TypeMap")
            .clone();
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
