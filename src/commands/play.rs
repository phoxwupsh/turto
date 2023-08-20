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
        guild::GuildUtil,
        play::{play_next, play_url},
    },
};

use tracing::error;

#[command]
#[bucket = "music"]
async fn play(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let guild = msg.guild(ctx).unwrap();

    // Get the Songbird instance
    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialization.");

    // Check if the user is in a voice channel
    let user_voice_channel = match guild.get_user_voice_channel(&msg.author.id) {
        Some(channel_id) => channel_id,
        None => {
            msg.reply(ctx, "You are not in a voice channel").await?;
            return Ok(());
        }
    };

    // Check if the bot is in a voice channel or not, if not join the voice channel
    if let Some(bot_voice_channel) = guild.get_user_voice_channel(&ctx.cache.current_user_id()) {
        if bot_voice_channel != user_voice_channel {
            // Notify th user if they are in different voice channel
            msg.reply(
                ctx,
                format!("I'm currently in {}.", bot_voice_channel.mention()),
            )
            .await?;
            return Ok(());
        }
    } else {
        let (_handler_lock, success) = manager.join(guild.id, user_voice_channel).await;
        if success.is_ok() {
            msg.channel_id
                .say(ctx, format!("🐢{}", user_voice_channel.mention()))
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

        let meta = play_url(ctx, guild.id, url).await?;

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

            if let Some(current_track) = playing.get(&guild.id) {
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

        if let Ok(meta) = play_next(ctx, guild.id).await {
            // if there is any song in the play list
            msg.reply(ctx, format!("▶️ {}", meta.title.unwrap())).await?;
        } else {
            // if the playlist is empty
            msg.reply(ctx, "You have to provide a url.").await?;
        }
    }
    Ok(())
}
