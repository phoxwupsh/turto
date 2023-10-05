use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::Message,
    prelude::Context,
};

use crate::{
    typemap::guild_data::GuildDataMap, messages::TurtoMessage,
};

#[command]
#[bucket = "turto"]
async fn autoleave(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let toggle = match args.rest() {
        "on" => true,
        "off" => false,
        _ => {
            msg.reply(ctx, TurtoMessage::SetAutoleave(Err(()))).await?;
            return Ok(());
        }
    };
    let guild_data_map_lock = ctx
        .data
        .read()
        .await
        .get::<GuildDataMap>()
        .unwrap()
        .clone();
    {
        let mut guild_data_map = guild_data_map_lock.lock().await;
        let guild_data = guild_data_map
            .entry(msg.guild_id.unwrap())
            .or_default();
        guild_data.config.auto_leave = toggle;
    }
    msg.reply(ctx, TurtoMessage::SetAutoleave(Ok(toggle)))
        .await?;
    Ok(())
}
