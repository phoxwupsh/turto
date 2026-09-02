use crate::models::alias::Command;
use std::{cmp::Ordering, collections::HashMap, fmt, str::FromStr, sync::OnceLock};

/// One command's registration, submitted next to the command itself by the
/// [`turto_command`](turto_macros::turto_command) attribute macro. It is the source of
/// the default (unlocalized) help text; locale files in
/// [`HelpConfig`](crate::models::help::HelpConfig) only override it.
pub struct CommandEntry {
    pub name: &'static str,
    pub short_description: &'static str,
    /// The `/help` body.
    pub description: &'static str,
    /// Whether `/help` leaves this command out of the commands it offers.
    pub hide_in_help: bool,
    pub parameters: &'static [ParamMeta],
    /// The command constructor `#[poise::command]` generates from the function.
    pub build: fn() -> Command,
}

inventory::collect!(CommandEntry);

/// Default help text for a single command parameter, see [`CommandEntry`].
#[derive(Debug)]
pub struct ParamMeta {
    pub name: &'static str,
    pub description: &'static str,
}

impl CommandEntry {
    /// Whether this command has a parameter named `name`.
    pub fn has_param(&self, name: &str) -> bool {
        self.parameters.iter().any(|p| p.name == name)
    }
}

/// A registered command.
///
/// A copy-cheap handle to the command's [`CommandEntry`]. Equality and ordering are by
/// name, so it is a stable map key whatever order the registrations were collected in.
#[derive(Clone, Copy)]
pub struct CommandKind(&'static CommandEntry);

impl CommandKind {
    /// This command's registration.
    pub fn entry(self) -> &'static CommandEntry {
        self.0
    }

    pub fn name(self) -> &'static str {
        self.0.name
    }
}

/// Every registered command, ordered by name.
///
/// The linker decides what order registrations are collected in, so they are sorted
/// once here: this order is what Discord shows and what tests iterate.
///
/// # Example
///
/// ```
/// let names: Vec<&str> = turto::models::command::registry()
///     .iter()
///     .map(|kind| kind.name())
///     .collect();
/// assert!(names.contains(&"play"));
/// ```
pub fn registry() -> &'static [CommandKind] {
    static REGISTRY: OnceLock<Vec<CommandKind>> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        let mut all = inventory::iter::<CommandEntry>
            .into_iter()
            .map(CommandKind)
            .collect::<Vec<_>>();
        all.sort_unstable();
        all
    })
}

/// The commands `/help` offers as a choice, in the order Discord shows them.
fn help_choices() -> &'static [CommandKind] {
    static HELP_CHOICES: OnceLock<Vec<CommandKind>> = OnceLock::new();
    HELP_CHOICES.get_or_init(|| {
        registry()
            .iter()
            .copied()
            .filter(|kind| !kind.entry().hide_in_help)
            .collect()
    })
}

/// A name that is not a registered command.
#[derive(Debug, thiserror::Error)]
#[error("`{0}` is not a command")]
pub struct UnknownCommand(String);

impl FromStr for CommandKind {
    type Err = UnknownCommand;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let all = registry();
        all.binary_search_by(|kind| kind.name().cmp(s))
            .map(|index| all[index])
            .map_err(|_| UnknownCommand(s.to_owned()))
    }
}

/// A `hide_in_help` command must not be reachable as a `/help` argument, including
/// from a hand-built interaction, so every lookup here goes through help_choices.
impl poise::ChoiceParameter for CommandKind {
    fn list() -> Vec<poise::CommandParameterChoice> {
        help_choices()
            .iter()
            .map(|kind| poise::CommandParameterChoice {
                name: kind.name().to_owned(),
                localizations: HashMap::new(),
                __non_exhaustive: (),
            })
            .collect()
    }

    fn from_index(index: usize) -> Option<Self> {
        help_choices().get(index).copied()
    }

    fn from_name(name: &str) -> Option<Self> {
        help_choices()
            .iter()
            .copied()
            .find(|kind| kind.name().eq_ignore_ascii_case(name))
    }

    fn name(&self) -> &'static str {
        self.0.name
    }

    fn localized_name(&self, _locale: &str) -> Option<&'static str> {
        None
    }
}

impl PartialEq for CommandKind {
    fn eq(&self, other: &Self) -> bool {
        self.0.name == other.0.name
    }
}

impl Eq for CommandKind {}

impl PartialOrd for CommandKind {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CommandKind {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.name.cmp(other.0.name)
    }
}

impl fmt::Display for CommandKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0.name)
    }
}

impl fmt::Debug for CommandKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("CommandKind").field(&self.0.name).finish()
    }
}
