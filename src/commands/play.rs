use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::Message,
    prelude::Context,
};
use songbird::tracks::PlayMode;
use url::Url;

use crate::{
    guild::playing::Playing,
    utils::{
        guild::{GuildUtil, VoiceChannelState},
        play::{play_next, play_url},
    }, messages::TurtoMessage,
};

use tracing::error;

#[command]
#[bucket = "music"]
async fn play(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let guild = msg.guild(ctx).unwrap();

    match guild.cmp_voice_channel(&ctx.cache.current_user_id(), &msg.author.id) {
        VoiceChannelState::None | VoiceChannelState::OnlyFirst(_) => {
            msg.reply(ctx, TurtoMessage::UserNotInVoiceChannel).await?;
            return Ok(());
        }
        VoiceChannelState::Different(bot_vc, _) => {
            msg.reply(ctx, TurtoMessage::DifferentVoiceChannel { bot: &bot_vc }).await?;
            return Ok(());
        }
        VoiceChannelState::OnlySecond(user_vc) => {
            let (_handler_lock, success) = songbird::get(ctx)
                .await
                .expect("Songbird Voice client placed in at initialization.")
                .join(guild.id, user_vc)
                .await;
            if success.is_ok() {
                msg.channel_id
                    .say(ctx, TurtoMessage::Join(&user_vc))
                    .await?;
            }
        }
        VoiceChannelState::Same(_) => (),
    }

    let url = args.rest().to_string();

    // Check if url is provided
    if !url.is_empty() {
        // Validate the URL
        if Url::parse(&url).is_err() {
            msg.reply(ctx, TurtoMessage::InvalidUrl(None)).await?;
            return Ok(());
        }

        let meta = play_url(ctx, guild.id, url).await?;
        msg.reply(ctx, TurtoMessage::Play { title: meta.title.as_ref().unwrap() }).await?;
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
            msg.reply(ctx, TurtoMessage::Play { title: meta.title.as_ref().unwrap() }).await?;
        } else {
            // if the playlist is empty
            msg.reply(ctx, TurtoMessage::InvalidUrl(None)).await?;
        }
    }
    Ok(())
}
