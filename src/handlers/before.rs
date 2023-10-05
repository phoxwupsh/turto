use serenity::{framework::standard::macros::hook, model::prelude::Message, prelude::Context};

use crate::{messages::TurtoMessage, typemap::guild_data::GuildDataMap};

#[hook]
pub async fn before_hook(ctx: &Context, msg: &Message, _cmd_name: &str) -> bool {
    let guild = msg.guild(ctx).unwrap();
    let guild_data_map_lock = ctx.data.read().await.get::<GuildDataMap>().unwrap().clone();
    {
        let mut guild_data_map = guild_data_map_lock.lock().await;
        let guild_data = guild_data_map.entry(guild.id).or_default();
        if guild_data.config.banned.get(&msg.author.id).is_some() {
            msg.reply(ctx, TurtoMessage::BannedUserResponse)
                .await
                .unwrap_or_else(|err| panic!("Error sending message: {}", err));
            return false;
        }
    }
    true
}
