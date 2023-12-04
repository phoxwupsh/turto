use crate::models::help::{Help, HelpFileModel};
use std::{fs, sync::OnceLock};

pub fn get_help() -> &'static Help {
    static HELP: OnceLock<Help> = OnceLock::new();
    HELP.get_or_init(|| {
        let helps_file = fs::read_to_string("help.toml")
            .map_err(|err| panic!("Error loading help.toml: {err}"))
            .and_then(|helps_json| toml::from_str::<HelpFileModel>(&helps_json))
            .unwrap_or_else(|err| panic!("Error parsing help.toml: {err}"));
        Help::from(helps_file)
    })
}
pub fn get_command_list() -> &'static Vec<String> {
    static COMMAND_LIST: OnceLock<Vec<String>> = OnceLock::new();
    COMMAND_LIST.get_or_init(|| {
        let mut command_list = get_help()
            .commands
            .keys()
            .cloned()
            .collect::<Vec<String>>();
        command_list.sort();
        command_list
    })
}
