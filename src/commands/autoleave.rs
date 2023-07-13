use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::Message,
    prelude::Context,
};

use crate::{guild::setting::Settings, models::setting::GuildSetting};

#[command]
#[bucket = "music"]
async fn autoleave(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let repl: String;
    let toggle = match args.rest() {
        "on" => {
            repl = "✅".to_string();
            true
        }
        "off" => {
            repl = "❎".to_string();
            false
        }
        _ => {
            msg.reply(ctx, "Please specify to turn on or off autoleave")
                .await?;
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
    msg.reply(ctx, format!("Auto leave: {}", repl)).await?;
    Ok(())
}
