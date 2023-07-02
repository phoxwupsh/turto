use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::Message,
    prelude::Context,
};

use crate::{
    guild::setting::Settings, models::setting::GuildSetting
};


#[command]
async fn autoleave(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let toggle = match args.rest() {
         "on" => true,
         "off" => false,
         _ => {
            msg.reply(ctx, "Please specify to turn on or off autoleave").await?;
            return Ok(());
         }
    };
    let settings_lock = {
        let data_read = ctx.data.read().await;
        data_read
            .get::<Settings>()
            .expect("Expected Playlists in TypeMap.")
            .clone()
    };
    {
        let mut settings = settings_lock.lock().await;
        let setting = settings
            .entry(msg.guild_id.unwrap())
            .or_insert_with(GuildSetting::default);
        setting.auto_leave = toggle;
    }
    if toggle {
        msg.reply(ctx, "Auto leave: ✅").await?;
    }
    else {
        msg.reply(ctx, "Auto leave: ❎").await?;
    }
    Ok(())
}