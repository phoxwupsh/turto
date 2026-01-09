use crate::{
    models::{alias::Context, error::CommandError},
    utils::misc::sha256_now,
};
use serenity::{
    builder::{CreateEmbed, CreateEmbedAuthor},
    prelude::Mentionable,
};
use tracing::{Span, instrument};

#[poise::command(slash_command, guild_only)]
#[instrument(
    name = "about",
    skip_all,
    parent = ctx.invocation_data::<Span>().await.as_deref().unwrap_or(&Span::none())
)]
pub async fn about(ctx: Context<'_>) -> Result<(), CommandError> {
    tracing::info!("invoked");

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

    ctx.send(poise::CreateReply::default().embed(embed)).await?;
    Ok(())
}
