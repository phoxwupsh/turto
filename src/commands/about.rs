use crate::{config::TurtoConfigProvider, utils::misc::sha256_now};
use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::Message,
    prelude::{Context, Mentionable},
};

#[command]
#[bucket = "turto"]
async fn about(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    msg.channel_id
        .send_message(ctx, |response| {
            response.reference_message(msg).embed(|embed| {
                if let Some(owner) = TurtoConfigProvider::get().owner {
                    embed.field("Owner of this bot", owner.mention(), true);
                }
                embed
                    .author(|author| {
                        author
                            .name("phoxwupsh")
                            .url("https://github.com/phoxwupsh")
                            .icon_url("https://avatars.githubusercontent.com/u/89735195")
                    })
                    .field("Version", env!("CARGO_PKG_VERSION"), true)
                    .title(env!("CARGO_PKG_NAME"))
                    .description(env!("CARGO_PKG_DESCRIPTION"))
                    .url(env!("CARGO_PKG_REPOSITORY"))
                    .image(format!(
                        "https://opengraph.githubassets.com/{}/phoxwupsh/turto",
                        sha256_now()
                    ));
                embed
            });
            response
        })
        .await?;
    Ok(())
}
