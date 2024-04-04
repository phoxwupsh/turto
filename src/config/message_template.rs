use crate::utils::template::Template;
use anyhow::{anyhow, Context, Result};
use std::{collections::HashMap, fs, path::Path, sync::OnceLock};

type Templates = HashMap<String, HashMap<String, Template>>;

static TEMPLATES: OnceLock<Templates> = OnceLock::new();
static TEMPLATES_LIST: [&str; 32] = [
    "not_playing",
    "user_not_in_voice_channel",
    "bot_not_in_voice_channel",
    "different_voice_channel",
    "play",
    "pause",
    "skip",
    "stop",
    "join",
    "leave",
    "queue",
    "remove",
    "remove_all",
    "invalid_remove_index",
    "url_not_found",
    "invalid_url",
    "volume",
    "toggle_autoleave",
    "seek_success",
    "invalid_seek",
    "seek_not_allow",
    "backward_seek_not_allow",
    "seek_not_long_enough",
    "administrator_only",
    "user_got_banned",
    "user_already_banned",
    "user_got_unbanned",
    "user_not_banned",
    "banned_user_repsonse",
    "empty_playlist",
    "shuffle",
    "toggle_repeat",
];

pub fn get_template(template_name: &str, locale: Option<&str>) -> &'static Template {
    TEMPLATES
        .get()
        .unwrap()
        .get(locale.unwrap_or("default"))
        .and_then(|templates| templates.get(template_name))
        .unwrap()
}

pub fn load_templates(path: impl AsRef<Path>) -> Result<()> {
    let templates_map = fs::read_to_string(path.as_ref())
        .context(format!(
            "Failed to load message templates from {}",
            path.as_ref().display()
        ))
        .and_then(|templates_toml| {
            toml::from_str::<HashMap<String, HashMap<String, String>>>(&templates_toml)
                .context("Failed to parse message templates")
        })?;

    let templates = templates_map
        .into_iter()
        .map(|(locale, map)| {
            let templates = map
                .into_iter()
                .map(|(template_name, template_str)| {
                    let template = Template::parse(&template_str);
                    (template_name, template)
                })
                .collect::<HashMap<_, _>>();
            (locale, templates)
        })
        .collect::<HashMap<_, _>>();
    
    if let Some(default_templates) = templates.get("default") {
        for template_name in TEMPLATES_LIST {
            if default_templates.get(template_name).is_none() {
                return Err(anyhow!("Missing default language message template: {}", template_name,));
            }
        }
    } else {
        return Err(anyhow!("Missing default message template"));
    }


    TEMPLATES.set(templates).unwrap();

    Ok(())
}
