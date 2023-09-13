use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::Message,
    prelude::Context,
};

use crate::{guild::setting::Settings, messages::TurtoMessage, models::setting::GuildSetting};

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
    let settings_lock = ctx
        .data
        .read()
        .await
        .get::<Settings>()
        .expect("Expected Playlists in TypeMap.")
        .clone();
    {
        let mut settings = settings_lock.lock().await;
        let setting = settings
            .entry(msg.guild_id.unwrap())
            .or_insert_with(GuildSetting::default);
        setting.auto_leave = toggle;
    }
    msg.reply(ctx, TurtoMessage::SetAutoleave(Ok(toggle))).await?;
    Ok(())
}
