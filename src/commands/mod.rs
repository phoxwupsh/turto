use crate::models::{alias::Command, command::registry, config::TurtoConfig, help::HelpConfig};
use std::time::Duration;

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

pub fn create_commands(config: &TurtoConfig, help_config: &HelpConfig) -> Vec<Command> {
    let command_cooldown = Duration::from_secs(config.command_delay);

    registry()
        .iter()
        .map(|kind| {
            let mut command = (kind.entry().build)();

            // Default-locale descriptions for the slash command preview and its parameters.
            let default = help_config.resolve(None, *kind);
            command.description = Some(default.short_description.to_owned());
            for param in command.parameters.iter_mut() {
                if let Some(desc) = default.parameters.get(param.name.as_str()) {
                    param.description = Some(desc.to_string());
                }
            }

            // Localized descriptions, applied only where a locale actually provides them.
            for over in help_config.locale_overrides(*kind) {
                if let Some(short_desc) = over.short_description() {
                    command
                        .description_localizations
                        .insert(over.locale.to_owned(), short_desc.to_owned());
                }
                for param in command.parameters.iter_mut() {
                    if let Some(param_desc) = over.parameter(&param.name) {
                        param
                            .description_localizations
                            .insert(over.locale.to_owned(), param_desc.to_owned());
                    }
                }
            }

            // Per-command cooldown.
            command.cooldown_config.write().unwrap().guild = Some(command_cooldown);

            command
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::models::command::{CommandKind, registry};
    use poise::ChoiceParameter;
    use std::collections::HashSet;

    /// Every command's registration must match its real [`poise::command`] signature:
    /// same command name and same set of parameter names. The macro guarantees this by
    /// construction; this test fails loudly if that ever breaks.
    #[test]
    fn registration_matches_command_signature() {
        for kind in registry() {
            let entry = kind.entry();
            let command = (entry.build)();

            assert_eq!(entry.name, command.name, "command name mismatch");

            let real: HashSet<&str> = command.parameters.iter().map(|p| p.name.as_str()).collect();
            let documented: HashSet<&str> = entry.parameters.iter().map(|p| p.name).collect();
            assert_eq!(
                real, documented,
                "parameter mismatch for command `{}`",
                entry.name
            );
        }
    }

    /// Discord rejects a slash command whose argument offers more than 25 choices, so
    /// `/help` breaks once there are more than 25 documented commands.
    #[test]
    fn help_choices_fit_discord_limit() {
        let choices = CommandKind::list();
        assert!(
            choices.len() <= 25,
            "`/help` offers {} choices, Discord allows at most 25",
            choices.len()
        );
        // `help` itself has no help page, so it must not be selectable.
        assert!(CommandKind::from_name("help").is_none());
    }
}
