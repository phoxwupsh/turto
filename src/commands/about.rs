use crate::{config::get_config, utils::misc::sha256_now};
use serenity::{
    builder::{CreateEmbed, CreateEmbedAuthor, CreateMessage},
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::Message,
    prelude::{Context, Mentionable},
};

#[command]
#[bucket = "turto"]
async fn about(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let mut embed = CreateEmbed::new()
        .author(
            CreateEmbedAuthor::new("phoxwupsh")
                .url("https://github.com/phoxwupsh")
                .icon_url("https://avatars.githubusercontent.com/u/89735195"),
        )
        .field("Version", env!("CARGO_PKG_VERSION"), true)
        .title(env!("CARGO_PKG_NAME"))
        .description(env!("CARGO_PKG_DESCRIPTION"))
        .url(env!("CARGO_PKG_REPOSITORY"))
        .image(format!(
            "https://opengraph.githubassets.com/{}/phoxwupsh/turto",
            sha256_now()
        ));
    if let Some(owner) = get_config().owner {
        embed = embed.field("Owner of this bot", owner.mention().to_string(), true);
    }

    let response = CreateMessage::new().reference_message(msg).embed(embed);
    msg.channel_id
        .send_message(ctx, response)
        .await?;
    Ok(())
}
