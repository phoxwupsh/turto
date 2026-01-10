use crate::models::{alias::Command, config::TurtoConfig, help::HelpConfig};
use std::{str::FromStr, time::Duration};
use strum::{AsRefStr, Display, EnumIter, EnumString};

/// THe macro for anything that requires all of the command names
///
/// # Why
///
/// Use this method because maintaining several lists of command names
/// scatter everywhere and depending on them without any alignment or check
/// is kinda error prone.
///
/// # Usage
/// Define a macro that accepts comma separated tokens and put the macro in this
///
/// # Example
///
/// ```rust
/// macro_rules! create_cmd {
///     ($($cmd:ident),* $(,)?) => {
///         $(println!(stringify!($cmd));)*
///     };
/// }
///
/// for_each_cmd!(create_cmd);
/// ```
#[macro_export]
macro_rules! for_each_cmd {
    ($macro:ident) => {
        $macro! {
            about,
            autoleave,
            ban,
            clear,
            help,
            insert,
            join,
            leave,
            pause,
            play,
            playlist,
            playwhat,
            queue,
            remove,
            repeat,
            seek,
            shuffle,
            skip,
            stop,
            unban,
            volume,
        }
    };
}

/// Define mod for all commands
///
/// Generates code like below
/// ```rust
/// mod about;
/// ```
macro_rules! define_cmd_mod {
    ($($cmd:ident),* $(,)?) => {
        $(mod $cmd;)*
    };
}

for_each_cmd!(define_cmd_mod);

/// Define enum for all commands
///
/// Generates code like below
/// ```rust
/// #[allow(non_camel_case_types)]
/// #[derive(Debug, poise::ChoiceParameter, Display, AsRefStr, EnumIter, EnumString)]
/// #[strum(serialize_all = "snake_case")]
/// pub enum CommandKind {
///     about
/// }
/// ```
macro_rules! define_cmd_kind {
    ($($cmd:ident),* $(,)?) => {
        #[allow(non_camel_case_types)]
        #[derive(Debug, poise::ChoiceParameter, Display, AsRefStr, EnumIter, EnumString)]
        #[strum(serialize_all = "snake_case")]
        pub enum CommandKind {
            $($cmd,)*
        }
    };
}

for_each_cmd!(define_cmd_kind);

pub fn create_commands(config: &TurtoConfig, help_config: &HelpConfig) -> Vec<Command> {
    macro_rules! create_cmd {
        ($($cmd:ident),* $(,)?) => {
            vec![
                $(
                    $cmd::$cmd(),
                )*
            ]
        };
    }

    let mut commands = for_each_cmd!(create_cmd);

    for command in commands.iter_mut() {
        let kind = CommandKind::from_str(&command.name).unwrap();
        let cmd_help = help_config.view_default_locale_command(kind);

        command.description = Some(cmd_help.short_description.into_owned());

        for param in command.parameters.iter_mut() {
            // it must not fail because this is the default one
            // if it failed there must be some problem is our code
            let param_desc = cmd_help.parameters.get(param.name.as_str()).unwrap();
            param.description = Some(param_desc.to_string());
        }
    }

    for (locale, help_locale) in help_config.iter_locale() {
        let view = help_locale.view();
        for command in commands.iter_mut() {
            if let Some(command_help) = view.get(command.name.as_str())
                && let Some(short_desc) = command_help.short_description.as_deref()
            {
                command
                    .description_localizations
                    .insert(locale.to_owned(), short_desc.to_owned());

                if let Some(ref param_help) = command_help.params {
                    for param in command.parameters.iter_mut() {
                        if let Some(param_desc) = param_help.get(param.name.as_str()) {
                            param
                                .description_localizations
                                .insert(locale.to_owned(), param_desc.to_string());
                        }
                    }
                }
            }
        }
    }

    for command in commands.iter_mut() {
        let command_cooldown = Duration::from_secs(config.command_delay);
        command.cooldown_config.write().unwrap().guild = Some(command_cooldown);
    }
    commands
}
