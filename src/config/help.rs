use crate::models::help::{CommandHelp, Help};
use std::{collections::HashMap, sync::OnceLock};

static NEW_HELP: OnceLock<Help> = OnceLock::new();

pub fn get_locale_help(locale: Option<&str>) -> &HashMap<String, CommandHelp> {
    let help = get_help();
    if let Some(res) = locale.and_then(|locale| help.get(locale)) {
        res
    } else {
        // fallback to default if the locale is not available
        help.get("default")
            .unwrap_or_else(|| panic!("unable to read default help info"))
    }
}

pub fn get_help() -> &'static Help {
    NEW_HELP.get_or_init(|| {
        load_help().unwrap_or_else(|err| panic!("Error loading help.toml: {}", err))
    })
}

pub fn locale_list() -> Vec<&'static str> {
    get_help()
        .keys()
        .map(|key| key.as_str())
        .filter(|key| key != &"default") // default doesn't count
        .collect()
}

fn load_help() -> Result<Help, Box<dyn std::error::Error>> {
    let file = std::fs::read_to_string("help.toml")?;
    let res = toml::from_str::<Help>(&file)?;
    Ok(res)
}
