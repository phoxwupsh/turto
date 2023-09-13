use crate::utils::guild::{GuildUtil, VoiceChannelState};
use serenity::{
    framework::standard::{macros::command, CommandResult},
    model::prelude::Message,
    prelude::{Context, Mentionable},
};

#[command]
#[bucket = "music"]
async fn leave(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(ctx).unwrap();
    
    match guild.cmp_voice_channel(&ctx.cache.current_user_id(), &msg.author.id) {
        VoiceChannelState::None | VoiceChannelState::OnlySecond(_) => {
            msg.reply(ctx, "Currently not in a voice channel").await?;
            return Ok(())
        },
        VoiceChannelState::Different(bot_vc, _) | VoiceChannelState::OnlyFirst(bot_vc) => {
            msg.reply(ctx, format!("You are not in {}", bot_vc.mention())).await?;
            return Ok(())
        },
        VoiceChannelState::Same(_) => ()
    }

    let guild = msg.guild(ctx).unwrap();

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placing in Resource failed.")
        .clone();

    manager.remove(guild.id).await?;
    Ok(())
}
