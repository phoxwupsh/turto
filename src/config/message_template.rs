use crate::utils::template::Template;
use std::{collections::HashMap, fs, path::Path, sync::OnceLock};

static TEMPLATES: OnceLock<HashMap<String, HashMap<String, Template>>> = OnceLock::new();

pub fn get_template(template_name: &str, locale: Option<&str>) -> &'static Template {
    TEMPLATES
        .get_or_init(|| load_templates("templates.toml"))
        .get(locale.unwrap_or("default"))
        .and_then(|templates| templates.get(template_name))
        .unwrap_or_else(|| {
            panic!(
                "Can't find the message template \"{}\" in templates.toml",
                template_name
            )
        })
}

fn load_templates(path: impl AsRef<Path>) -> HashMap<String, HashMap<String, Template>> {
    let templates_map = fs::read_to_string(path)
        .map_err(|err| panic!("Error loading templates.toml: {err}"))
        .and_then(|templates_toml| {
            toml::from_str::<HashMap<String, HashMap<String, String>>>(&templates_toml)
        })
        .unwrap_or_else(|err| panic!("Error parsing templates.toml: {err}"));

    templates_map
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
        .collect::<HashMap<_, _>>()
}
