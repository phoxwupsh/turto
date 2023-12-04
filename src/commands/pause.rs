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
async fn pause(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let guild = msg.guild(&ctx.cache).unwrap().clone();
    let bot_id = ctx.cache.current_user().id;

    match guild.cmp_voice_channel(&bot_id, &msg.author.id) {
        VoiceChannelState::None | VoiceChannelState::OnlySecond(_) => {
            msg.reply(ctx, TurtoMessage::BotNotInVoiceChannel).await?;
            return Ok(());
        }
        VoiceChannelState::Different(bot_vc, _) | VoiceChannelState::OnlyFirst(bot_vc) => {
            msg.reply(ctx, TurtoMessage::DifferentVoiceChannel { bot: bot_vc })
                .await?;
            return Ok(());
        }
        VoiceChannelState::Same(_) => (),
    }

    let playing_lock = ctx.data.read().await.get::<PlayingMap>().unwrap().clone();

    let playing_map = playing_lock.read().await;
    let Some(playing) = playing_map.get(&guild.id) else {
        msg.reply(ctx, TurtoMessage::NotPlaying).await?;
        return Ok(());
    };

    if let Err(why) = playing.track_handle.pause() {
        let uuid = playing.track_handle.uuid();
        error!("Failed to pause track {uuid}: {why}");
    }
    let title = playing.metadata.title.as_ref().unwrap();
    msg.reply(ctx, TurtoMessage::Pause { title }).await?;
    drop(playing_map);

    Ok(())
}
