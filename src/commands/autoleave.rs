use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::Message,
    prelude::Context,
};

use crate::{
    typemap::config::GuildConfigs, messages::TurtoMessage, models::guild::config::GuildConfig,
};

#[command]
#[bucket = "music"]
async fn autoleave(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let toggle = match args.rest() {
        "on" => true,
        "off" => false,
        _ => {
            msg.reply(ctx, TurtoMessage::SetAutoleave(Err(()))).await?;
            return Ok(());
        }
    };
    let guild_configs_lock = ctx
        .data
        .read()
        .await
        .get::<GuildConfigs>()
        .unwrap()
        .clone();
    {
        let mut guild_configs = guild_configs_lock.lock().await;
        let guild_config = guild_configs
            .entry(msg.guild_id.unwrap())
            .or_insert_with(GuildConfig::default);
        guild_config.auto_leave = toggle;
    }
    msg.reply(ctx, TurtoMessage::SetAutoleave(Ok(toggle)))
        .await?;
    Ok(())
}
