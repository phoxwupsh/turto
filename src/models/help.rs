use std::{collections::HashMap, fs};

use serde::{Serialize, Deserialize};
use lazy_static::lazy_static;

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct Help {
    pub help_msg: String,
    pub placeholder: String,
    pub usage_field: String,
    pub example_field: String,
    pub command_helps: HashMap<String, CommandHelp>
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct CommandHelp {
    pub help_msg: String,
    pub command_name: String,
    pub description: String,
    pub usage: String,
    pub example: Vec<String>
}

lazy_static!{
    pub static ref HELPS: Help = {
        let helps_json = fs::read_to_string("helps.json").expect("Error loading helps.json");
        let helps: Help = serde_json::from_str(&helps_json).expect("Error parsing helps.json");
        helps
    };
}