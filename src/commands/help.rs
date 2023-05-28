use std::collections::HashSet;

use serenity::{
    framework::standard::{
        help_commands, macros::help, Args, CommandGroup, CommandResult, HelpOptions,
    },
    model::prelude::{Message, UserId},
    prelude::Context,
};

#[help]
#[individual_command_tip = "以下是所有能用的指令，你可以用`help 指令`來查看個指令的詳細用法。使用任何指令前面記得加上`!`。"]
#[strikethrough_commands_tip_in_guild = ""]
#[usage_label = "用法"]
#[usage_sample_label = "範例"]
#[grouped_label = "類別"]
#[ungrouped_label = "未分類"]
#[available_text = "使用時機"]
#[guild_only_text = "只能在伺服器中使用"]
#[group_prefix = "!"]
async fn my_help(
    context: &Context,
    msg: &Message,
    args: Args,
    help_options: &'static HelpOptions,
    groups: &[&'static CommandGroup],
    owners: HashSet<UserId>,
) -> CommandResult {
    let _ = help_commands::with_embeds(context, msg, args, help_options, groups, owners).await;
    Ok(())
}
