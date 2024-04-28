use crate::utils::template::Template;
use anyhow::{anyhow, Context, Result};
use std::{collections::HashMap, fs, path::Path, sync::OnceLock};

type Templates = HashMap<String, HashMap<String, Template>>;

static TEMPLATES: OnceLock<Templates> = OnceLock::new();
static TEMPLATES_LIST: [&str; 34] = [
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
    "invalid_playlist_page",
    "remove_many",
];

pub fn get_template(template_name: &str, locale: Option<&str>) -> &'static Template {
    let langs = TEMPLATES.get().unwrap();
    match langs.get(&locale.unwrap_or("default").to_ascii_lowercase()) { // case insensitive for locale ID
        Some(templates) => templates.get(template_name),
        None => langs.get("default").unwrap().get(template_name),
    }.unwrap()
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
            (locale.to_ascii_lowercase(), templates) // case insensitive for locale ID
        })
        .collect::<HashMap<_, _>>();

    if let Some(default_templates) = templates.get("default") {
        for template_name in TEMPLATES_LIST {
            if default_templates.get(template_name).is_none() {
                return Err(anyhow!(
                    "Missing default language message template: {}",
                    template_name,
                ));
            }
        }
    } else {
        return Err(anyhow!("Missing default message template"));
    }

    TEMPLATES.set(templates).unwrap();

    Ok(())
}


#[cfg(test)]
mod tests {
    use super::{get_template, load_templates};

    #[test]
    fn test_get_unsupported_lang() {
        load_templates("templates.toml.template").unwrap();
        let cn = get_template("not_playing", Some("zh-CN")).renderer().render();
        let ja = get_template("empty_playlist", Some("ja")).renderer().render();
        let none = get_template("user_not_in_voice_channel", None).renderer().render();
        assert_eq!(cn.as_str(), "Not playing now.");
        assert_eq!(ja.as_str(), "The playlist is empty.");
        assert_eq!(none.as_str(), "You are not in a voice channel.");
    }
    
    #[test]
    fn test_get_supported_lang() {
        load_templates("templates.toml.template").unwrap();
        let upper = get_template("not_playing", Some("ZH-TW")).renderer().render();
        let lower = get_template("not_playing", Some("zh-tw")).renderer().render();
        let mixed = get_template("not_playing", Some("zh-TW")).renderer().render();
        assert_eq!(upper.as_str(), "現在沒有在播放任何東西。");
        assert_eq!(lower.as_str(), "現在沒有在播放任何東西。");
        assert_eq!(mixed.as_str(), "現在沒有在播放任何東西。");
    
    }
}
