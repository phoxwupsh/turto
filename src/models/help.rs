use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize)]
pub struct CommandHelp {
    pub short_description: String,
    #[serde(default)]
    pub description: String,
    pub parameters: Option<HashMap<String, String>>
}

pub type Help = HashMap<String, HashMap<String, CommandHelp>>;