use std::time::Duration;

use crate::{
    commands::{
        about::about, autoleave::autoleave, ban::ban, clear::clear, help::help, insert::insert,
        join::join, leave::leave, pause::pause, play::play, playlist::playlist, playwhat::playwhat,
        queue::queue, remove::remove, repeat::repeat, seek::seek, shuffle::shuffle, skip::skip,
        stop::stop, unban::unban, volume::volume,
    },
    config::{
        get_config,
        help::{get_help, locale_list},
    },
    models::alias::Command,
};
use tracing::warn;

pub mod about;
pub mod autoleave;
pub mod ban;
pub mod clear;
pub mod help;
pub mod insert;
pub mod join;
pub mod leave;
pub mod pause;
pub mod play;
pub mod playlist;
pub mod playwhat;
pub mod queue;
pub mod remove;
pub mod repeat;
pub mod seek;
pub mod shuffle;
pub mod skip;
pub mod stop;
pub mod unban;
pub mod volume;

pub fn create_commands() -> Vec<Command> {
    let mut commands = vec![
        about(),
        autoleave(),
        ban(),
        clear(),
        help(),
        insert(),
        join(),
        leave(),
        pause(),
        play(),
        playlist(),
        playwhat(),
        queue(),
        remove(),
        repeat(),
        seek(),
        shuffle(),
        skip(),
        stop(),
        unban(),
        volume(),
    ];

    for command in commands.iter_mut() {
        let help = get_help();
        // add default short description
        if let Some(command_help) = help
            .get("default")
            .and_then(|default_help| default_help.get(&command.name))
        {
            command.description = Some(command_help.short_description.to_string());
            // add default description for each parameter
            for parameter in command.parameters.iter_mut() {
                let Some(parameter_description) = command_help
                    .parameters
                    .as_ref()
                    .and_then(|parameters| parameters.get(&parameter.name))
                else {
                    warn!(
                        "Description of parameter {} of command {} not found",
                        parameter.name, command.name
                    );
                    continue;
                };
                parameter.description = Some(parameter_description.to_string());
            }
        } else {
            warn!("Short description of command {} not found", command.name);
        }
        // add short description for all available locales
        for locale in locale_list() {
            if let Some(command_help) = help
                .get(locale)
                .and_then(|locale_help| locale_help.get(&command.name))
            // .map(|command_help| command_help.short_description.as_str())
            {
                command.description_localizations.insert(
                    locale.to_string(),
                    command_help.short_description.to_string(),
                );
                for parameter in command.parameters.iter_mut() {
                    let Some(parameter_description) = command_help
                        .parameters
                        .as_ref()
                        .and_then(|parameters| parameters.get(&parameter.name))
                    else {
                        warn!(
                            "Description of parameter {} of command {} for locale {} not found",
                            parameter.name, command.name, locale
                        );
                        continue;
                    };
                    parameter
                        .description_localizations
                        .insert(locale.to_string(), parameter_description.to_string());
                }
            } else {
                warn!(
                    "Short description of command {} for locale {} not found",
                    command.name, locale
                )
            }
        }
        // set command cooldown for each command
        let command_cooldown = Duration::from_secs(get_config().command_delay);
        command.cooldown_config.write().unwrap().guild = Some(command_cooldown);
    }
    commands
}
