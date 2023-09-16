use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::Message,
    prelude::Context,
};

use tracing::error;

use crate::{
    guild::playing::Playing,
    messages::TurtoMessage,
    utils::guild::{GuildUtil, VoiceChannelState},
};

#[command]
#[bucket = "music"]
async fn pause(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let guild = msg.guild(ctx).unwrap();

    match guild.cmp_voice_channel(&ctx.cache.current_user_id(), &msg.author.id) {
        VoiceChannelState::None | VoiceChannelState::OnlySecond(_) => {
            msg.reply(ctx, TurtoMessage::BotNotInVoiceChannel).await?;
            return Ok(());
        }
        VoiceChannelState::Different(bot_vc, _) | VoiceChannelState::OnlyFirst(bot_vc) => {
            msg.reply(ctx, TurtoMessage::DifferentVoiceChannel { bot: &bot_vc })
                .await?;
            return Ok(());
        }
        VoiceChannelState::Same(_) => (),
    }

    let playing_lock = ctx
        .data
        .read()
        .await
        .get::<Playing>()
        .expect("Expected Playing in TypeMap")
        .clone();
    {
        let playing = playing_lock.read().await;
        let current_track = match playing.get(&guild.id) {
            Some(track) => track,
            None => {
                msg.reply(ctx, TurtoMessage::NotPlaying).await?;
                return Ok(());
            }
        };

        if let Err(why) = current_track.pause() {
            error!("Error pausing a track {}: {:?}", current_track.uuid(), why);
        }

        msg.reply(
            ctx,
            TurtoMessage::Pause {
                title: current_track.metadata().title.as_ref().unwrap(),
            },
        )
        .await?;
    }
    Ok(())
}
