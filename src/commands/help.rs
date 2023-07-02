use std::time::Duration;

use serenity::{
    framework::standard::{macros::command, CommandResult},
    model::{application::interaction::InteractionResponseType, prelude::Message},
    prelude::Context,
};

use crate::models::help::HELPS;

#[command]
async fn help(ctx: &Context, msg: &Message) -> CommandResult {
    let waiting = msg.channel_id.send_message(ctx, |m| {
        m.content("以下是所有能用的指令，你可以用`help 指令`來查看個指令的詳細用法。使用任何指令前面記得加上`!`。");
        m.components(|c| {
            c.create_action_row(|r| {
                r.create_select_menu(|menu| {
                    menu.custom_id("help");
                    menu.placeholder("選一個指令");
                    menu.options(|options| {
                        for k in HELPS.keys() {
                            options.create_option(|o| o.label(k).value(k));
                        }
                        options
                    })
                })
            })
        })
    }).await?;

    let interaction = match waiting
        .await_component_interaction(ctx)
        .timeout(Duration::from_secs(60 * 3))
        .await
    {
        Some(x) => x,
        None => {
            msg.reply(ctx, "Timed out").await?;
            return Ok(());
        }
    };

    let command_name = &interaction.data.values[0];
    let target_help = HELPS.get(command_name).expect("Error loading command help");

    interaction
        .create_interaction_response(ctx, |resp| {
            resp.kind(InteractionResponseType::UpdateMessage)
                .interaction_response_data(|data| {
                    data.content(format!("以下是`{}`的使用方式：", command_name));
                    data.components(|c| c);
                    data.embed(|e| {
                        e.title(target_help.command_name.clone());
                        e.description(target_help.description.clone());
                        e.field("用法", target_help.usage.clone(), true);
                        e.field("範例", target_help.example.join("\n"), true)
                    })
                })
        })
        .await?;

    Ok(())
}
