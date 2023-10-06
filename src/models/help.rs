use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug)]
pub struct Help {
    pub placeholder: String,
    pub usage_field: String,
    pub example_field: String,
    pub commands: HashMap<String, CommandHelp>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct HelpFileModel {
    placeholder: String,
    usage_field: String,
    example_field: String,
    commands: HashMap<String, CommandHelpFileModel>,
}

#[derive(Debug)]
pub struct CommandHelp {
    pub description: String,
    pub usage: String,
    pub example: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CommandHelpFileModel {
    description: String,
    usage: String,
    example: Vec<String>,
}

impl From<HelpFileModel> for Help {
    fn from(value: HelpFileModel) -> Self {
        let command_helps = value
            .commands
            .into_iter()
            .map(|(k, v)| (k, CommandHelp::from(v)))
            .collect::<HashMap<_, _>>();
        Help {
            placeholder: value.placeholder,
            usage_field: value.usage_field,
            example_field: value.example_field,
            commands: command_helps,
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
            description: value.description,
            usage: usage_str,
            example: examples_str,
        }
    }
}
