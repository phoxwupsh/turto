use serenity::{framework::standard::macros::hook, model::prelude::Message, prelude::Context};

use crate::{messages::TurtoMessage, typemap::config::GuildConfigs};

#[hook]
pub async fn before_hook(ctx: &Context, msg: &Message, _cmd_name: &str) -> bool {
    let guild = msg.guild(ctx).unwrap();
    let guild_configs_lock = ctx.data.read().await.get::<GuildConfigs>().unwrap().clone();
    {
        let mut guild_configs = guild_configs_lock.lock().await;
        let guild_config = guild_configs.entry(guild.id).or_default();
        if guild_config.banned.get(&msg.author.id).is_some() {
            msg.reply(ctx, TurtoMessage::BannedUserResponse)
                .await
                .unwrap_or_else(|err| panic!("Error sending message: {}", err));
            return false;
        }
    }
    true
}
