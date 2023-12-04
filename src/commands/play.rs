use crate::{
    messages::TurtoMessage,
    typemap::playing::PlayingMap,
    utils::{
        guild::{GuildUtil, VoiceChannelState},
        play::{play_next, play_url},
    },
};
use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::Message,
    prelude::Context,
};
use songbird::tracks::PlayMode;
use tracing::error;
use url::Url;

#[command]
#[bucket = "turto"]
async fn play(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let guild = msg.guild(&ctx.cache).unwrap().clone();
    let bot_id = ctx.cache.current_user().id;

    let call = match guild.cmp_voice_channel(&bot_id, &msg.author.id) {
        VoiceChannelState::None | VoiceChannelState::OnlyFirst(_) => {
            msg.reply(ctx, TurtoMessage::UserNotInVoiceChannel).await?;
            return Ok(());
        }
        VoiceChannelState::Different(bot_vc, _) => {
            msg.reply(ctx, TurtoMessage::DifferentVoiceChannel { bot: bot_vc })
                .await?;
            return Ok(());
        }
        VoiceChannelState::OnlySecond(user_vc) => {
            match songbird::get(ctx)
                .await
                .unwrap()
                .join(guild.id, user_vc)
                .await
            {
                Ok(call) => {
                    msg.reply(ctx, TurtoMessage::Join(user_vc)).await?;
                    call
                }
                Err(err) => {
                    error!("Failed to join voice channel {user_vc}: {err}");
                    return Ok(());
                }
            }
        }
        VoiceChannelState::Same(_) => songbird::get(ctx).await.unwrap().get(guild.id).unwrap(),
    };

    let url = args.rest().to_string();

    // Check if url is provided
    if !url.is_empty() {
        // Validate the URL
        if Url::parse(&url).is_err() {
            msg.reply(ctx, TurtoMessage::InvalidUrl(None)).await?;
            return Ok(());
        }

        let meta = play_url(call, ctx.data.clone(), guild.id, url).await?;
        msg.reply(
            ctx,
            TurtoMessage::Play {
                title: meta.title.as_ref().unwrap(),
            },
        )
        .await?;
    } else {
        // If no url provided, check if there is a paused track or there is any song in the playlist
        let playing_lock = ctx.data.read().await.get::<PlayingMap>().unwrap().clone();
        let playing_map = playing_lock.read().await;
        if let Some(playing) = playing_map.get(&guild.id) {
            if let Ok(current_track_state) = playing.track_handle.get_info().await {
                if current_track_state.playing == PlayMode::Pause {
                    // If there is a paused song then play it
                    if let Err(why) = playing.track_handle.play() {
                        let uuid = playing.track_handle.uuid();
                        error!("Failed to play track {uuid}: {why}");
                    }
                    return Ok(());
                }
            }
        }
        drop(playing_map);

        if let Some(Ok(meta)) = play_next(call, ctx.data.clone(), guild.id).await {
            // if there is any song in the play list
            msg.reply(
                ctx,
                TurtoMessage::Play {
                    title: meta.title.as_ref().unwrap(),
                },
            )
            .await?;
        } else {
            // if the playlist is empty
            msg.reply(ctx, TurtoMessage::InvalidUrl(None)).await?;
        }
    }
    Ok(())
}
