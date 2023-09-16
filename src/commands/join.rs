use crate::{utils::guild::{GuildUtil, VoiceChannelState}, messages::TurtoMessage};

use serenity::{
    client::Context,
    framework::standard::{macros::command, CommandResult},
    model::channel::Message
};

#[command]
#[bucket = "music"]
async fn join(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(ctx).unwrap();

    match guild.cmp_voice_channel(&ctx.cache.current_user_id(), &msg.author.id) {
        VoiceChannelState::Different(bot_vc, _) => {
            msg.reply(ctx, TurtoMessage::DifferentVoiceChannel { bot: &bot_vc }).await?;
            return Ok(());
        }
        VoiceChannelState::None | VoiceChannelState::OnlyFirst(_) => {
            msg.reply(ctx, TurtoMessage::UserNotInVoiceChannel).await?;
            return Ok(());
        }
        VoiceChannelState::OnlySecond(user_vc) => {
            let (_handler_lock, success) = songbird::get(ctx)
                .await
                .expect("Songbird Voice client placed in at initialization.")
                .join(guild.id, user_vc)
                .await;
            if success.is_ok() {
                msg.reply(ctx, TurtoMessage::Join(&user_vc)).await?;
            }
        }
        VoiceChannelState::Same(_) => (),
    }
    Ok(())
}
