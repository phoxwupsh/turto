use serenity::{framework::standard::macros::hook, model::prelude::Message, prelude::Context};

use crate::{guild::setting::GuildSettings, models::guild_setting::GuildSetting, messages::TurtoMessage};

#[hook]
pub async fn before_hook(ctx: &Context, msg: &Message, _cmd_name: &str) -> bool {
    let guild = msg.guild(ctx).unwrap();
    let guild_settings_lock = ctx
        .data
        .read()
        .await
        .get::<GuildSettings>()
        .expect("Expected GuildSettings in TypeMap")
        .clone();
    {
        let mut guild_settings = guild_settings_lock.lock().await;
        let guild_setting = guild_settings
            .entry(guild.id)
            .or_insert_with(GuildSetting::default);
        if guild_setting.banned.get(&msg.author.id).is_some() {
            msg.reply(ctx, TurtoMessage::BannedUserResponse).await.expect("Error Sending message");
            return false
        }
    }
    true
}
