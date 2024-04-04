use crate::{
    messages::{TurtoMessage, TurtoMessageKind::EmptyPlaylist},
    models::alias::{Context, Error},
    utils::misc::ToEmoji,
};
use poise::{CreateReply, ReplyHandle};
use serenity::{
    all::{ComponentInteractionDataKind, EditMessage},
    builder::{
        CreateActionRow, CreateInteractionResponse, CreateSelectMenu, CreateSelectMenuKind,
        CreateSelectMenuOption,
    },
    collector::ComponentInteractionCollector,
};
use std::time::Duration;

#[poise::command(slash_command, guild_only)]
pub async fn playlist(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();
    let guild_data = ctx.data().guilds.entry(guild_id).or_default();
    let waiting: Option<ReplyHandle>;
    let custom_id = ctx.id();

    if guild_data.playlist.is_empty() {
        drop(guild_data);
        ctx.say(TurtoMessage {
            locale: ctx.locale(),
            kind: EmptyPlaylist,
        })
        .await?;
        return Ok(());
    } else if guild_data.playlist.len() <= 10 {
        // directly display if the playlist has less than 10 items
        let response = guild_data
            .playlist
            .iter()
            .enumerate()
            .map(|(index, playlist_item)| {
                let mut line = (index + 1).to_string() + ". ";
                line.push(' ');
                line.push_str(&playlist_item.title);
                line
            })
            .fold(String::new(), |acc, title| acc + &title + "\n")
            .trim_end()
            .to_owned();
        drop(guild_data);

        ctx.say(response).await?;
        return Ok(());
    } else {
        // show the select menu if the playlist has more than 10 items (discord text message has a length limitation of 2000 unicode chars)
        let page_num = (guild_data.playlist.len() / 10) + 1;
        drop(guild_data);

        let options = (1..=page_num)
            .map(|index| {
                CreateSelectMenuOption::new("📄".to_string() + &index.to_emoji(), index.to_string())
            })
            .collect::<Vec<_>>();
        let action = CreateActionRow::SelectMenu(
            CreateSelectMenu::new(
                custom_id.to_string(),
                CreateSelectMenuKind::String { options },
            )
            .placeholder("📖❓"),
        );

        let select_response = CreateReply::default().components(vec![action]);
        let select_msg = ctx.send(select_response).await?;
        waiting = Some(select_msg)
    }

    let Some(mut mci) = ComponentInteractionCollector::new(ctx)
        .author_id(ctx.author().id)
        .channel_id(ctx.channel_id())
        .timeout(Duration::from_secs(60))
        .filter(move |mci| mci.data.custom_id == custom_id.to_string())
        .await
    else {
        // delete the select menu if not selected after timeout
        if let Some(waiting) = waiting {
            waiting.delete(ctx).await?;
        }
        return Ok(());
    };

    let page = match &mci.data.kind {
        ComponentInteractionDataKind::StringSelect { values } => {
            values[0].parse::<usize>().unwrap()
        }
        _ => unreachable!(),
    };

    let page_index = page - 1;
    let guild_data = ctx.data().guilds.entry(guild_id).or_default();
    let response_content = guild_data
        .playlist
        .iter()
        .enumerate()
        .filter(|(index, _playlist_item)| index / 10 == page_index)
        .map(|(index, playlist_item)| {
            let mut line = (index + 1).to_string() + ". ";
            line.push_str(&playlist_item.title);
            line
        })
        .fold(String::new(), |acc, title| acc + &title + "\n")
        .trim_end()
        .to_owned();
    drop(guild_data);

    mci.message
        .edit(
            ctx,
            EditMessage::new()
                .components(vec![])
                .content(response_content),
        )
        .await?;
    mci.create_response(ctx, CreateInteractionResponse::Acknowledge)
        .await?;
    Ok(())
}
