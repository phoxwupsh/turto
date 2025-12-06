use std::time::Duration;

use crate::{
    commands::{
        about::about, autoleave::autoleave, ban::ban, clear::clear, help::help, insert::insert,
        join::join, leave::leave, pause::pause, play::play, playlist::playlist, playwhat::playwhat,
        queue::queue, remove::remove, repeat::repeat, seek::seek, shuffle::shuffle, skip::skip,
        stop::stop, unban::unban, volume::volume,
    },
    models::{alias::Command, config::TurtoConfig, help::Help},
};
use tracing::warn;

mod about;
mod autoleave;
mod ban;
mod clear;
mod help;
mod insert;
mod join;
mod leave;
mod pause;
mod play;
mod playlist;
mod playwhat;
mod queue;
mod remove;
mod repeat;
mod seek;
mod shuffle;
mod skip;
mod stop;
mod unban;
mod volume;

pub fn create_commands(config: &TurtoConfig, help_config: &Help) -> Vec<Command> {
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
        // add default short description
        if let Some(command_help) = help_config.get_default(&command.name) {
            command.description = Some(command_help.short_description.clone());
            // add default description for each parameter
            for parameter in command.parameters.iter_mut() {
                let Some(parameter_description) = command_help
                    .parameters
                    .as_ref()
                    .and_then(|parameters| parameters.get(&parameter.name))
                else {
                    warn!(
                        parameter = parameter.name,
                        command = command.name,
                        "default description of parameter not found",
                    );
                    continue;
                };
                parameter.description = Some(parameter_description.to_string());
            }
        } else {
            warn!(
                command = command.name,
                "default short description not found"
            );
        }
        // add short description for all available locales
        for locale in help_config.available_locale() {
            if let Some(command_help) = help_config.get(locale, &command.name) {
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
                            parameter = parameter.name,
                            command = command.name,
                            locale = locale,
                            "description of parameter not found",
                        );
                        continue;
                    };
                    parameter
                        .description_localizations
                        .insert(locale.to_string(), parameter_description.to_string());
                }
            } else {
                warn!(
                    command = command.name,
                    locale, "short description not found",
                )
            }
        }
        // set command cooldown for each command
        let command_cooldown = Duration::from_secs(config.command_delay);
        command.cooldown_config.write().unwrap().guild = Some(command_cooldown);
    }
    commands
}
