use crate::{
    message::{
        TurtoMessage,
        TurtoMessageKind::{EmptyPlaylist, InvalidPlaylistPage},
    },
    models::{alias::Context, error::CommandError, playlist::Playlist},
    utils::{misc::ToEmoji, turto_say},
    ytdl::YouTubeDl,
};
use poise::CreateReply;
use serenity::{
    all::{ComponentInteractionDataKind, CreateButton, EditMessage, ReactionType},
    builder::{
        CreateActionRow, CreateInteractionResponse, CreateSelectMenu, CreateSelectMenuKind,
        CreateSelectMenuOption,
    },
    collector::ComponentInteractionCollector,
};
use std::time::Duration;
use tracing::{Span, instrument};

const PREV_CUSTOM_ID: &str = "prev";
const NEXT_CUSTOM_ID: &str = "next";
const SELECT_MENU_PAGE_SIZE: usize = 25;
const PLAYLIST_PAGE_SIZE: usize = 10;

#[poise::command(slash_command, guild_only)]
#[instrument(
    name = "playlist",
    skip_all,
    parent = ctx.invocation_data::<Span>().await.as_deref().unwrap_or(&Span::none())
    fields(page)
)]
pub async fn playlist(
    ctx: Context<'_>,
    #[min = 1] page: Option<usize>,
) -> Result<(), CommandError> {
    tracing::info!("invoked");

    let guild_id = ctx.guild_id().unwrap();
    let guild_data = ctx.data().guilds.entry(guild_id).or_default();
    let total_pages = playlist_page_len(&guild_data.playlist);

    if guild_data.playlist.is_empty() {
        drop(guild_data);
        turto_say(ctx, EmptyPlaylist).await?;
        return Ok(());
    }

    if let Some(page) = page {
        tracing::info!("show playlist page");

        let playlist_str = generate_playlist_str(&guild_data.playlist, page);
        let response = if playlist_str.is_empty() {
            TurtoMessage::new(ctx, InvalidPlaylistPage { total_pages }).to_string()
        } else {
            playlist_str
        };

        drop(guild_data);

        ctx.say(response).await?;
        return Ok(());
    }

    if guild_data.playlist.len() <= PLAYLIST_PAGE_SIZE {
        tracing::info!(item = guild_data.playlist.len(), "show playlist");

        // directly display if the playlist has less than 1 page
        let response = generate_playlist_str(&guild_data.playlist, 1);
        drop(guild_data);

        ctx.say(response).await?;
        Ok(())
    } else {
        // show the select menu if the playlist has more than 10 items
        // since discord text message has a length limitation of 2000 unicode chars

        tracing::info!("show select menu");

        let custom_id = ctx.id().to_string();
        let mut curr_start_page = 1_usize;
        let total_pages = playlist_page_len(&guild_data.playlist);
        let select_menu = generate_page_select_menu(
            &custom_id,
            curr_start_page,
            total_pages.min(SELECT_MENU_PAGE_SIZE),
        );
        let mut components = vec![select_menu];

        // there is a limitation of 25 for items in a select menu
        // show pagination buttons when there's more than 25 pages
        if total_pages > SELECT_MENU_PAGE_SIZE {
            let prev_btn = create_dir_btn(false, true);
            let next_btn = create_dir_btn(true, false);
            components.push(CreateActionRow::Buttons(vec![prev_btn, next_btn]));
        }
        drop(guild_data);

        let mut select_response = CreateReply::default().components(components);
        if total_pages > SELECT_MENU_PAGE_SIZE {
            select_response =
                select_response.content(page_indicator_str(curr_start_page, SELECT_MENU_PAGE_SIZE));
        }

        let select_msg = ctx.send(select_response).await?;

        // read next button or select menu action until some item is select
        // or time out because of no action
        loop {
            let custom_id_inner = custom_id.clone();
            let collector = ComponentInteractionCollector::new(ctx)
                .author_id(ctx.author().id)
                .channel_id(ctx.channel_id())
                .timeout(Duration::from_secs(60))
                .filter(move |mci| {
                    mci.data.custom_id == custom_id_inner
                        || mci.data.custom_id == PREV_CUSTOM_ID
                        || mci.data.custom_id == NEXT_CUSTOM_ID
                });

            match collector.await {
                Some(mut mci) => {
                    match mci.data.kind {
                        ComponentInteractionDataKind::Button => {
                            match mci.data.custom_id.as_str() {
                                PREV_CUSTOM_ID => {
                                    curr_start_page =
                                        curr_start_page.saturating_sub(SELECT_MENU_PAGE_SIZE);
                                    tracing::info!("prev button invoked");
                                }
                                NEXT_CUSTOM_ID => {
                                    curr_start_page =
                                        curr_start_page.saturating_add(SELECT_MENU_PAGE_SIZE);
                                    tracing::info!("next button invoked");
                                }
                                // there must be some problem if the custom_id is not the ids we just set for those buttons
                                _ => {
                                    return Err(CommandError::InvalidOperation {
                                        cause: "non-existent interaction",
                                    });
                                }
                            }

                            let guild_data = ctx.data().guilds.entry(guild_id).or_default();
                            let total_pages = playlist_page_len(&guild_data.playlist);
                            let end_page_idx =
                                total_pages.min(curr_start_page + SELECT_MENU_PAGE_SIZE - 1);
                            let select_menu = generate_page_select_menu(
                                &custom_id,
                                curr_start_page,
                                end_page_idx,
                            );

                            let prev_disable = curr_start_page == 1;
                            let prev_btn = create_dir_btn(false, prev_disable);

                            let next_disable = end_page_idx >= total_pages;
                            let next_btn = create_dir_btn(true, next_disable);

                            let btn_row = CreateActionRow::Buttons(vec![prev_btn, next_btn]);

                            tracing::info!(
                                from = curr_start_page,
                                to = end_page_idx,
                                "show select menu"
                            );

                            mci.message
                                .edit(
                                    ctx,
                                    EditMessage::new()
                                        .content(page_indicator_str(curr_start_page, end_page_idx))
                                        .components(vec![select_menu, btn_row]),
                                )
                                .await?;
                            mci.create_response(ctx, CreateInteractionResponse::Acknowledge)
                                .await?;
                            continue;
                        }
                        ComponentInteractionDataKind::StringSelect { ref values } => {
                            let Some(page) =
                                values.first().and_then(|value| value.parse::<usize>().ok())
                            else {
                                return Err(CommandError::InvalidOperation {
                                    cause: "invalid interaction value",
                                });
                            };

                            let guild_data = ctx.data().guilds.entry(guild_id).or_default();
                            let response_content =
                                generate_playlist_str(&guild_data.playlist, page);
                            drop(guild_data);

                            tracing::info!(selected = page, "page selected");

                            let mut message = mci.message.clone();
                            message
                                .edit(
                                    ctx,
                                    EditMessage::new()
                                        .components(vec![])
                                        .content(response_content),
                                )
                                .await?;
                            mci.create_response(ctx, CreateInteractionResponse::Acknowledge)
                                .await?;
                            break;
                        }
                        // there must be some problem if we receive interactions other than those above kinds
                        _ => {
                            return Err(CommandError::InvalidOperation {
                                cause: "non-existent interaction",
                            });
                        }
                    }
                }
                None => {
                    // delete the select menu if not selected after timeout
                    tracing::info!("select menu timeout");

                    select_msg.delete(ctx).await?;
                    return Ok(());
                }
            }
        }

        tracing::info!("complete");

        Ok(())
    }
}

fn generate_playlist_str(playlist: &Playlist, page_index: usize) -> String {
    let mut res = String::new();
    for (index, item) in page_with_indices(playlist, page_index) {
        res.push_str(&index.to_string());
        res.push_str(". ");
        res.push_str(item.title().unwrap_or_default());
        res.push('\n');
    }
    res
}

fn playlist_page_len(playlist: &Playlist) -> usize {
    playlist.len().div_ceil(PLAYLIST_PAGE_SIZE)
}

fn page_with_indices(
    playlist: &Playlist,
    page_index: usize,
) -> impl Iterator<Item = (usize, &YouTubeDl)> {
    let start = (page_index - 1) * PLAYLIST_PAGE_SIZE;
    playlist
        .iter()
        .enumerate()
        .skip(start)
        .take(PLAYLIST_PAGE_SIZE)
        .map(|(index, ytdl)| (index + 1, ytdl))
}

fn generate_page_select_menu(
    custom_id: impl Into<String>,
    start_idx: usize,
    end_idx: usize,
) -> CreateActionRow {
    let options = (start_idx..=end_idx)
        .map(|index| CreateSelectMenuOption::new(index.to_emoji(), index.to_string()).emoji('📄'))
        .collect::<Vec<_>>();
    CreateActionRow::SelectMenu(CreateSelectMenu::new(
        custom_id,
        CreateSelectMenuKind::String { options },
    ))
}

fn create_dir_btn(next: bool, disabled: bool) -> CreateButton {
    let (custom_id, emoji) = if next {
        (NEXT_CUSTOM_ID, "➡️")
    } else {
        (PREV_CUSTOM_ID, "⬅️")
    };
    CreateButton::new(custom_id)
        .emoji(ReactionType::Unicode(emoji.to_owned()))
        .disabled(disabled)
}

fn page_indicator_str(start: usize, end: usize) -> String {
    format!("📖{}↔️{}", start.to_emoji(), end.to_emoji())
}
