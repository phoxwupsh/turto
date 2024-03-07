use crate::{
    config::help::get_locale_help,
    models::alias::{Context, Error},
};
use poise::{ChoiceParameter, CreateReply};
use serenity::builder::CreateEmbed;

#[derive(ChoiceParameter)]
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
pub async fn help(ctx: Context<'_>, command: HelpOption) -> Result<(), Error> {
    let helps = get_locale_help(ctx.locale());
    let command_name = command.name();

    let target_help = helps
        .get(command_name)
        .unwrap_or(get_locale_help(None).get(command_name).unwrap()); // fallback to default language

    let mut embed = CreateEmbed::new()
        .title(command_name)
        .description(&target_help.description);

    if let Some(parameters) = &target_help.parameters {
        for (name, description) in parameters.iter() {
            embed = embed.field(name, description, false);
        }
    }

    let response = CreateReply::default().embed(embed);

    ctx.send(response).await?;
    Ok(())
}
