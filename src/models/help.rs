use std::{collections::HashMap, fs, sync::OnceLock};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct Help {
    pub help_msg: String,
    pub placeholder: String,
    pub usage_field: String,
    pub example_field: String,
    pub command_helps: HashMap<String, CommandHelp>,
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct CommandHelp {
    pub help_msg: String,
    pub command_name: String,
    pub description: String,
    pub usage: String,
    pub example: Vec<String>,
}

impl Help {
    pub fn get_helps() -> &'static Help {
        static HELP: OnceLock<Help> = OnceLock::new();
        HELP.get_or_init(||{
            let helps_json = fs::read_to_string("helps.json").expect("Error loading helps.json");
            serde_json::from_str::<Help>(&helps_json).expect("Error parsing helps.json")
        })
    }
    pub fn get_command_list() -> &'static Vec<String> {
        static COMMAND_LIST: OnceLock<Vec<String>> = OnceLock::new();
        COMMAND_LIST.get_or_init(||{
            let mut command_list = Self::get_helps().command_helps.keys().cloned().collect::<Vec<String>>();
            command_list.sort();
            command_list
        })
    }
}