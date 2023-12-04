use crate::{typemap::guild_data::GuildDataMap, utils::misc::ToEmoji};
use serenity::{
    all::ComponentInteractionDataKind,
    builder::{
        CreateActionRow, CreateInteractionResponse, CreateInteractionResponseMessage,
        CreateMessage, CreateSelectMenu, CreateSelectMenuKind, CreateSelectMenuOption,
    },
    framework::standard::{macros::command, CommandResult},
    futures::StreamExt,
    model::prelude::Message,
    prelude::Context,
};
use std::time::Duration;

#[command]
#[bucket = "turto"]
async fn playlist(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_data_map = ctx.data.read().await.get::<GuildDataMap>().unwrap().clone();
    let guild_data = guild_data_map.entry(msg.guild_id.unwrap()).or_default();
    let waiting: Option<Message>;

    if guild_data.playlist.is_empty() {
        drop(guild_data);
        msg.reply(ctx, "🈳").await?;
        return Ok(());
    } else if guild_data.playlist.len() <= 10 {
        // directly display if the playlist has less than 10 items
        let response = guild_data
            .playlist
            .iter()
            .enumerate()
            .map(|(index, playlist_item)| {
                let mut line = (index + 1).to_emoji();
                line.push(' ');
                line.push_str(&playlist_item.title);
                line
            })
            .fold(String::new(), |acc, title| acc + &title + "\n")
            .trim_end()
            .to_owned();
        drop(guild_data);

        msg.reply(ctx, response).await?;
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
            CreateSelectMenu::new("page_select", CreateSelectMenuKind::String { options })
                .placeholder("📖❓"),
        );

        let select_msg = msg
            .channel_id
            .send_message(
                ctx,
                CreateMessage::new()
                    .reference_message(msg)
                    .components(vec![action]),
            )
            .await?;
        waiting = Some(select_msg)
    }

    if let Some(waiting) = waiting {
        let mut interactions = waiting
            .await_component_interactions(ctx)
            .timeout(Duration::from_secs(60))
            .stream();

        let interaction = loop {
            match interactions.next().await {
                Some(interaction) => {
                    if interaction.user == msg.author {
                        // response only if the interaction is send by the user who invoke the help command
                        break interaction;
                    }
                }
                None => {
                    // if there is no interaction sended, delete the select menu
                    waiting.delete(ctx).await?;
                    return Ok(());
                }
            }
        };

        let page: usize = match &interaction.data.kind {
            ComponentInteractionDataKind::StringSelect { values } => values[0].parse().unwrap(),
            _ => panic!("Invalid playlist select"),
        };

        let page_index = page - 1;
        let guild_data = guild_data_map.entry(msg.guild_id.unwrap()).or_default();
        let response_content = guild_data
            .playlist
            .iter()
            .enumerate()
            .filter(|(index, _playlist_item)| index / 10 == page_index)
            .map(|(index, playlist_item)| {
                let mut line = (index + 1).to_emoji();
                line.push(' ');
                line.push_str(&playlist_item.title);
                line
            })
            .fold(String::new(), |acc, title| acc + &title + "\n")
            .trim_end()
            .to_owned();
        drop(guild_data);

        let response = CreateInteractionResponse::UpdateMessage(
            CreateInteractionResponseMessage::new().content(response_content),
        );
        interaction.create_response(ctx, response).await?;
        return Ok(());
    }

    Ok(())
}
