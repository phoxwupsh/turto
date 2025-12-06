use crate::{
    models::alias::{Context, Error},
    utils::misc::sha256_now,
};
use serenity::{
    builder::{CreateEmbed, CreateEmbedAuthor},
    prelude::Mentionable,
};

#[poise::command(slash_command, guild_only)]
pub async fn about(ctx: Context<'_>) -> Result<(), Error> {
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
    if let Some(owner) = ctx.data().config.owner {
        embed = embed.field("Owner of this bot", owner.mention().to_string(), true);
    }

    if let Some(locale) = ctx.locale() {
        println!("locale: {}", locale);
    }

    ctx.send(poise::CreateReply::default().embed(embed)).await?;
    Ok(())
}
