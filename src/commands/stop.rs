use crate::{
    messages::TurtoMessage,
    typemap::playing::Playing,
    utils::guild::{GuildUtil, VoiceChannelState},
};
use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::Message,
    prelude::Context,
};
use tracing::error;

#[command]
#[bucket = "turto"]
async fn stop(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let guild = msg.guild(ctx).unwrap();

    match guild.cmp_voice_channel(&ctx.cache.current_user_id(), &msg.author.id) {
        VoiceChannelState::Different(bot_vc, _) | VoiceChannelState::OnlyFirst(bot_vc) => {
            msg.reply(ctx, TurtoMessage::DifferentVoiceChannel { bot: &bot_vc })
                .await?;
            return Ok(());
        }
        VoiceChannelState::OnlySecond(_) | VoiceChannelState::None => {
            msg.reply(ctx, TurtoMessage::BotNotInVoiceChannel).await?;
            return Ok(());
        }
        VoiceChannelState::Same(_) => (),
    }

    let playing_lock = ctx.data.read().await.get::<Playing>().unwrap().clone();
    {
        let mut playing = playing_lock.write().await;
        let current_track = match playing.remove(&guild.id) {
            Some(track) => track,
            None => {
                msg.reply(ctx, TurtoMessage::NotPlaying).await?;
                return Ok(());
            }
        };

        if let Err(why) = current_track.stop() {
            error!("Error stopping track {}: {}", current_track.uuid(), why);
        }

        let title = current_track.metadata().title.clone().unwrap();
        msg.reply(ctx, TurtoMessage::Stop { title: &title }).await?;
    }

    Ok(())
}
