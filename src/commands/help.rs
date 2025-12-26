use crate::models::{alias::Context, error::CommandError};
use poise::{ChoiceParameter, CreateReply};
use serenity::builder::CreateEmbed;
use tracing::{Span, instrument};

#[derive(ChoiceParameter, strum::Display)]
#[strum(serialize_all = "snake_case")]
enum HelpOption {
    #[name = "about"]
    About,
    #[name = "autoleave"]
    Autoleave,
    #[name = "ban"]
    Ban,
    #[name = "join"]
    Join,
    #[name = "leave"]
    Leave,
    #[name = "pause"]
    Pause,
    #[name = "play"]
    Play,
    #[name = "playlist"]
    Playlist,
    #[name = "playwhat"]
    Playwhat,
    #[name = "queue"]
    Queue,
    #[name = "remove"]
    Remove,
    #[name = "repeat"]
    Repeat,
    #[name = "seek"]
    Seek,
    #[name = "shuffle"]
    Shuffle,
    #[name = "skip"]
    Skip,
    #[name = "stop"]
    Stop,
    #[name = "unban"]
    Unban,
    #[name = "volume"]
    Volume,
}

#[poise::command(slash_command, guild_only)]
#[instrument(
    name = "help",
    skip_all,
    parent = ctx.invocation_data::<Span>().await.as_deref().unwrap_or(&Span::none())
    fields(%command)
)]
pub async fn help(ctx: Context<'_>, command: HelpOption) -> Result<(), CommandError> {
    tracing::info!("invoked");

    let helps = &ctx.data().help;
    let command_help = if let Some(locale) = ctx.locale() {
        helps
            .get(locale, command.name())
            .or(helps.get_default(command.name()))
            .unwrap()
    } else {
        helps.get_default(command.name()).unwrap()
    };

    let mut embed = CreateEmbed::new()
        .title(command.name())
        .description(&command_help.description);

    if let Some(parameters) = &command_help.parameters {
        for (name, description) in parameters.iter() {
            embed = embed.field(name, description, false);
        }
    }

    let response = CreateReply::default().embed(embed);

    ctx.send(response).await?;
    Ok(())
}
