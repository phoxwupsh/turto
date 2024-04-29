use crate::{
    messages::{TurtoMessage, TurtoMessageKind::Join},
    models::alias::{Context, Error},
};
use reqwest::Client;
use serenity::all::{ChannelId, GuildId};
use songbird::Call;
use std::sync::{Arc, OnceLock};
use tokio::sync::Mutex;

pub mod guild;
pub mod json;
pub mod misc;
pub mod play;
pub mod queue;
pub mod template;
pub mod ytdl;
pub mod url;

pub fn get_http_client() -> Client {
    static HTTP_CLIENT: OnceLock<Client> = OnceLock::new();
    HTTP_CLIENT.get_or_init(Client::new).clone()
}

pub async fn join_voice_channel(
    ctx: Context<'_>,
    locale: Option<&str>,
    guild_id: GuildId,
    channel_id: ChannelId,
) -> Result<Arc<Mutex<Call>>, Error> {
    // there is some time limit of a command to be response,
    // joining a voice can take time and cause timeout
    // so use defer to prevent timeout
    ctx.defer().await?;
    let success = songbird::get(ctx.serenity_context())
        .await
        .unwrap()
        .join(guild_id, channel_id)
        .await;
    match success {
        Ok(call) => {
            ctx.say(TurtoMessage {
                locale,
                kind: Join(channel_id),
            })
            .await?;
            Ok(call)
        }
        Err(err) => Err(Box::new(err)),
    }
}
