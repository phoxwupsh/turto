use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::Message,
    prelude::{Context, Mentionable},
};

use tracing::error;

use crate::{
    guild::playing::Playing,
    messages::NOT_PLAYING,
    utils::guild::{GuildUtil, VoiceChannelState},
};

#[command]
#[bucket = "music"]
async fn stop(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let guild = msg.guild(ctx).unwrap();

    match guild.cmp_voice_channel(&ctx.cache.current_user_id(), &msg.author.id) {
        VoiceChannelState::Different(bot_vc, _) | VoiceChannelState::OnlyFirst(bot_vc) => {
            msg.reply(ctx, format!("You are not in {}", bot_vc.mention())).await?;
            return Ok(());
        }
        VoiceChannelState::OnlySecond(_) | VoiceChannelState::None => {
            msg.reply(ctx, "Currently not in a voice channel").await?;
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
        let mut playing = playing_lock.write().await;
        let current_track = match playing.remove(&guild.id) {
            Some(track) => track,
            None => {
                msg.reply(ctx, NOT_PLAYING).await?;
                return Ok(());
            }
        };

        if let Err(why) = current_track.stop() {
            error!("Error stopping track {}: {:?}", current_track.uuid(), why);
        }

        msg.reply(
            ctx,
            format!("⏹️ {}", current_track.metadata().title.clone().unwrap()),
        )
        .await?;
    }

    Ok(())
}
