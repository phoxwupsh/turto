use crate::{
    messages::TurtoMessage,
    utils::guild::{GuildUtil, VoiceChannelState},
};
use serenity::{
    client::Context,
    framework::standard::{macros::command, CommandResult},
    model::channel::Message,
};

#[command]
#[bucket = "turto"]
async fn join(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(&ctx.cache).unwrap().clone();
    let bot_id = ctx.cache.current_user().id;

    match guild.cmp_voice_channel(&bot_id, &msg.author.id) {
        VoiceChannelState::Different(bot_vc, _) => {
            msg.reply(ctx, TurtoMessage::DifferentVoiceChannel { bot: bot_vc })
                .await?;
            return Ok(());
        }
        VoiceChannelState::None | VoiceChannelState::OnlyFirst(_) => {
            msg.reply(ctx, TurtoMessage::UserNotInVoiceChannel).await?;
            return Ok(());
        }
        VoiceChannelState::OnlySecond(user_vc) => {
            let success = songbird::get(ctx)
                .await
                .unwrap()
                .join(guild.id, user_vc)
                .await;
            if success.is_ok() {
                msg.reply(ctx, TurtoMessage::Join(user_vc)).await?;
            }
        }
        VoiceChannelState::Same(_) => (),
    }
    Ok(())
}
