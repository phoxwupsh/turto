use crate::{messages::TurtoMessage, typemap::guild_data::GuildDataMap};
use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::Message,
    prelude::Context,
};

#[command]
#[bucket = "turto"]
async fn repeat(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let toggle = match args.rest() {
        "on" => true,
        "off" => false,
        _ => {
            msg.reply(ctx, TurtoMessage::SetRepeat(Err(()))).await?;
            return Ok(());
        }
    };

    let guild_data_map = ctx.data.read().await.get::<GuildDataMap>().unwrap().clone();
    let mut guild_data = guild_data_map.entry(msg.guild_id.unwrap()).or_default();
    guild_data.config.repeat = toggle;
    drop(guild_data);

    msg.reply(ctx, TurtoMessage::SetRepeat(Ok(toggle)))
        .await?;
    Ok(())
}
