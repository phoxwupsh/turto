use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::Path};

#[derive(Debug, Serialize, Deserialize)]
pub struct CommandHelp {
    pub short_description: String,
    #[serde(default)]
    pub description: String,
    pub parameters: Option<HashMap<String, String>>,
}

#[derive(Debug)]
pub struct Help(HashMap<String, HashMap<String, CommandHelp>>);

impl Help {
    const COMMAND_LIST: [&str; 20] = [
        "about",
        "autoleave",
        "ban",
        "help",
        "insert",
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
    const DEFAULT_KEY: &str = "default";

    pub fn get(&self, locale: &str, command: &str) -> Option<&CommandHelp> {
        self.0.get(locale).and_then(|loc| loc.get(command))
    }

    pub fn get_default(&self, command: &str) -> Option<&CommandHelp> {
        self.0
            .get(Self::DEFAULT_KEY)
            .and_then(|loc| loc.get(command))
    }

    pub fn load(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let help_str = std::fs::read_to_string(path.as_ref())?;
        let map = toml::from_str::<HashMap<String, HashMap<String, CommandHelp>>>(&help_str)?;
        Ok(Self(map))
    }

    pub fn check_default(&self) -> Vec<&str> {
        Self::COMMAND_LIST
            .into_iter()
            .filter(|command| self.get_default(command).is_none())
            .collect()
    }

    pub fn available_locale(&self) -> Vec<&str> {
        self.0
            .keys()
            .map(String::as_str)
            .filter(|key| key != &Self::DEFAULT_KEY) // default doesn't count
            .collect()
    }
}
