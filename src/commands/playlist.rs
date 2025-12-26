use crate::{
    message::{
        TurtoMessage,
        TurtoMessageKind::{EmptyPlaylist, InvalidPlaylistPage},
    },
    models::{
        alias::Context, error::CommandError, playlist::Playlist
    },
    utils::{misc::ToEmoji, turto_say},
};
use poise::CreateReply;
use serenity::{
    all::{ComponentInteractionDataKind, EditMessage},
    builder::{
        CreateActionRow, CreateInteractionResponse, CreateSelectMenu, CreateSelectMenuKind,
        CreateSelectMenuOption,
    },
    collector::ComponentInteractionCollector,
};
use tracing::{Span, instrument};
use std::time::Duration;

#[poise::command(slash_command, guild_only)]
#[instrument(
    name = "playlist",
    skip_all,
    parent = ctx.invocation_data::<Span>().await.as_deref().unwrap_or(&Span::none())
    fields(page)
)]
pub async fn playlist(ctx: Context<'_>, #[min = 1] page: Option<usize>) -> Result<(), CommandError> {
    tracing::info!("invoked");

    let guild_id = ctx.guild_id().unwrap();
    let guild_data = ctx.data().guilds.entry(guild_id).or_default();
    let total_pages = guild_data.playlist.total_pages();

    if guild_data.playlist.is_empty() {
        drop(guild_data);
        turto_say(ctx, EmptyPlaylist).await?;
        return Ok(());
    }

    if let Some(page) = page {
        tracing::info!("show playlist page");

        let response = match generate_playlist_str(&guild_data.playlist, page) {
            Some(res) => res,
            None => TurtoMessage::new(ctx, InvalidPlaylistPage { total_pages }).to_string(),
        };
        drop(guild_data);

        ctx.say(response).await?;
        return Ok(());
    }

    if guild_data.playlist.len() <= 10 {
        tracing::info!(item = guild_data.playlist.len(), "show playlist");

        // directly display if the playlist has less than 10 items
        let response = generate_playlist_str(&guild_data.playlist, 1);
        drop(guild_data);

        ctx.say(response.unwrap()).await?;
        Ok(())
    } else if guild_data.playlist.len() <= 250 {
        // show the select menu if the playlist has more than 10 and less than 250 items
        // since discord text message has a length limitation of 2000 unicode chars
        // and select menu has a limitation of 25 options

        tracing::info!("show select menu");

        let custom_id = ctx.id().to_string();
        let select_menu = generate_page_select_menu(&guild_data.playlist, &custom_id);
        drop(guild_data);

        let select_response = CreateReply::default().components(vec![select_menu]);
        let select_msg = ctx.send(select_response).await?;

        let Some(mut mci) = ComponentInteractionCollector::new(ctx)
            .author_id(ctx.author().id)
            .channel_id(ctx.channel_id())
            .timeout(Duration::from_secs(60))
            .filter(move |mci| mci.data.custom_id == custom_id)
            .await
        else {
            // delete the select menu if not selected after timeout
            tracing::info!("select menu timeout");

            select_msg.delete(ctx).await?;
            return Ok(());
        };

        let page = match &mci.data.kind {
            ComponentInteractionDataKind::StringSelect { values } => {
                values[0].parse::<usize>().unwrap()
            }
            _ => unreachable!(),
        };

        tracing::info!(selected = page ,"page selected");

        let guild_data = ctx.data().guilds.entry(guild_id).or_default();
        let response_content = match generate_playlist_str(&guild_data.playlist, page) {
            Some(res) => res,
            // just in case the playlist in changed during the wait
            None => TurtoMessage::new(ctx, InvalidPlaylistPage { total_pages }).to_string(),
        };
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
    } else {
        // when the playlist has more than 250 items just display the total number of pages
        let response = "📖".to_string() + &total_pages.to_emoji();

        ctx.say(response).await?;
        Ok(())
    }
}

fn generate_playlist_str(playlist: &Playlist, page_index: usize) -> Option<String> {
    let mut res = String::new();
    for (index, item) in playlist.page_with_indices(page_index)? {
        let mut line = (index + 1).to_string() + ". ";
        line.push(' ');
        line.push_str(item.title().unwrap_or_default());
        line.push('\n');
        res.push_str(&line);
    }
    Some(res)
}

fn generate_page_select_menu(playlist: &Playlist, custom_id: impl Into<String>) -> CreateActionRow {
    let options = (1..=playlist.total_pages())
        .map(|index| CreateSelectMenuOption::new(index.to_emoji(), index.to_string()).emoji('📄'))
        .collect::<Vec<_>>();
    CreateActionRow::SelectMenu(CreateSelectMenu::new(
        custom_id,
        CreateSelectMenuKind::String { options },
    ))
}
