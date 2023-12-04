use crate::{
    messages::TurtoMessage,
    typemap::playing::PlayingMap,
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
    let guild = msg.guild(&ctx.cache).unwrap().clone();
    let bot_id = ctx.cache.current_user().id;

    match guild.cmp_voice_channel(&bot_id, &msg.author.id) {
        VoiceChannelState::Different(bot_vc, _) | VoiceChannelState::OnlyFirst(bot_vc) => {
            msg.reply(ctx, TurtoMessage::DifferentVoiceChannel { bot: bot_vc })
                .await?;
            return Ok(());
        }
        VoiceChannelState::OnlySecond(_) | VoiceChannelState::None => {
            msg.reply(ctx, TurtoMessage::BotNotInVoiceChannel).await?;
            return Ok(());
        }
        VoiceChannelState::Same(_) => (),
    }

    let playing_lock = ctx.data.read().await.get::<PlayingMap>().unwrap().clone();
    let mut playing_map = playing_lock.write().await;
    let Some(playing) = playing_map.remove(&guild.id) else {
        msg.reply(ctx, TurtoMessage::NotPlaying).await?;
        return Ok(());
    };
    drop(playing_map);

    if let Err(why) = playing.track_handle.stop() {
        let uuid = playing.track_handle.uuid();
        error!("Failed to stop track {uuid}: {why}");
    }

    let title = playing.metadata.title.clone().unwrap();
    msg.reply(ctx, TurtoMessage::Stop { title: &title }).await?;

    Ok(())
}
