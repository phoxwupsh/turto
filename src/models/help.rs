use std::{collections::HashMap, fs, sync::OnceLock};

use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub struct Help {
    pub help_message: String,
    pub placeholder: String,
    pub usage_field: String,
    pub example_field: String,
    pub command_helps: HashMap<String, CommandHelp>,
}

#[derive(Serialize, Deserialize, Debug)]
struct HelpFileModel {
    help_message: String,
    placeholder: String,
    usage_field: String,
    example_field: String,
    command_helps: HashMap<String, CommandHelpFileModel>,
}

#[derive(Debug)]
pub struct CommandHelp {
    pub help_message: String,
    pub command_name: String,
    pub description: String,
    pub usage: String,
    pub example: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct CommandHelpFileModel {
    help_message: String,
    command_name: String,
    description: String,
    usage: String,
    example: Vec<String>,
}

impl Help {
    pub fn get_helps() -> &'static Help {
        static HELP: OnceLock<Help> = OnceLock::new();
        HELP.get_or_init(|| {
            let helps_file = fs::read_to_string("helps.toml")
                .map_err(|err| panic!("Error loading helps.toml: {err}"))
                .and_then(|helps_json| toml::from_str::<HelpFileModel>(&helps_json))
                .unwrap_or_else(|err| panic!("Error parsing helps.toml: {err}"));
            Help::from(helps_file)
        })
    }
    pub fn get_command_list() -> &'static Vec<String> {
        static COMMAND_LIST: OnceLock<Vec<String>> = OnceLock::new();
        COMMAND_LIST.get_or_init(|| {
            let mut command_list = Self::get_helps()
                .command_helps
                .keys()
                .cloned()
                .collect::<Vec<String>>();
            command_list.sort();
            command_list
        })
    }
}

impl From<HelpFileModel> for Help {
    fn from(value: HelpFileModel) -> Self {
        let command_helps = value
            .command_helps
            .into_iter()
            .map(|(k, v)| (k, CommandHelp::from(v)))
            .collect::<HashMap<_, _>>();
        Help {
            help_message: value.help_message,
            placeholder: value.placeholder,
            usage_field: value.usage_field,
            example_field: value.example_field,
            command_helps,
        }
    }
}

impl From<CommandHelpFileModel> for CommandHelp {
    fn from(value: CommandHelpFileModel) -> Self {
        let mut usage_str = String::with_capacity(value.usage.len() + 8);
        usage_str.push('`');
        usage_str.push_str(&value.usage);
        usage_str.push('`');
        let examples_str_len =
            value.example.iter().map(|v| v.len()).sum::<usize>() + (value.example.len() * 8);
        let mut examples_str = String::with_capacity(examples_str_len);
        for example in value.example {
            examples_str.push('`');
            examples_str.push_str(&example);
            examples_str.push('`');
            examples_str.push('\n');
        }
        let _ = examples_str.trim_end();
        CommandHelp {
            help_message: value.help_message,
            command_name: value.command_name,
            description: value.description,
            usage: usage_str,
            example: examples_str,
        }
    }
}
