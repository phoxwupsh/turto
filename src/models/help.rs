use std::{collections::HashMap, fs};

use lazy_static::lazy_static;
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

lazy_static! {
    pub static ref HELPS: Help = {
        let helps_json = fs::read_to_string("helps.json").expect("Error loading helps.json");
        let helps: Help = serde_json::from_str(&helps_json).expect("Error parsing helps.json");
        helps
    };
    pub static ref COMMAND_LIST: Vec<String> = {
        let mut command_list = HELPS.command_helps.keys().cloned().collect::<Vec<String>>();
        command_list.sort();
        command_list
    };
}
