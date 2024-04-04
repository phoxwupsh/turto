use crate::models::help::{CommandHelp, Help};
use anyhow::{anyhow, Context, Result};
use std::{collections::HashMap, path::Path, sync::OnceLock};

static HELP: OnceLock<Help> = OnceLock::new();
static COMMAND_LIST: [&str; 19] = [
    "about",
    "autoleave",
    "ban",
    "help",
    "join",
    "leave",
    "pause",
    "play",
    "playlist",
    "playwhat",
    "queue",
    "remove",
    "repeat",
    "seek",
    "shuffle",
    "skip",
    "stop",
    "unban",
    "volume",
];

pub fn get_locale_help(locale: Option<&str>) -> &HashMap<String, CommandHelp> {
    let help = get_help();
    if let Some(res) = locale.and_then(|locale| help.get(locale)) {
        res
    } else {
        // fallback to default if the locale is not available
        help.get("default").unwrap()
    }
}

pub fn get_help() -> &'static Help {
    HELP.get().unwrap()
}

pub fn locale_list() -> Vec<&'static str> {
    get_help()
        .keys()
        .map(|key| key.as_str())
        .filter(|key| key != &"default") // default doesn't count
        .collect()
}

pub fn load_help(help_path: impl AsRef<Path>) -> Result<()> {
    let help = std::fs::read_to_string(help_path.as_ref())
        .context(format!(
            "Failed to load help info from {}",
            help_path.as_ref().display()
        ))
        .and_then(|help_str| {
            toml::from_str::<Help>(&help_str).context("Failed to parse help info")
        })?;

    if let Some(default) = help.get("default") {
        for command_name in COMMAND_LIST {
            if !default.contains_key(command_name) {
                return Err(anyhow!(
                    "Missing default language of help info of command: {}",
                    command_name
                ));
            }
        }
    } else {
        return Err(anyhow!("Missing default language of help info"));
    }

    HELP.set(help).unwrap();

    Ok(())
}
