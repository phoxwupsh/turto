use crate::utils::template::{Template, TemplateRenderer};
use std::{collections::HashMap, fs, path::Path, sync::OnceLock};

static TEMPLATES: OnceLock<HashMap<String, Template>> = OnceLock::new();

pub fn get_template(template_name: impl AsRef<str>) -> &'static Template {
    TEMPLATES
        .get_or_init(|| load_templates("templates.toml"))
        .get(template_name.as_ref())
        .unwrap_or_else(|| {
            panic!(
                "Can't find the message template \"{}\" in templates.toml",
                template_name.as_ref()
            )
        })
}
pub fn get_renderer(template_name: impl AsRef<str>) -> TemplateRenderer<'static> {
    get_template(template_name.as_ref()).renderer()
}

fn load_templates(path: impl AsRef<Path>) -> HashMap<String, Template> {
    let templates_map = fs::read_to_string(path)
        .map_err(|err| panic!("Error loading templates.toml: {err}"))
        .and_then(|templates_toml| toml::from_str::<HashMap<String, String>>(&templates_toml))
        .unwrap_or_else(|err| panic!("Error parsing templates.toml: {err}"));

    templates_map
        .into_iter()
        .map(|(k, v)| {
            let template = Template::parse(&v)
                .unwrap_or_else(|err| panic!("Error parsing template \"{k}\": {err}"));
            (k, template)
        })
        .collect::<HashMap<_, _>>()
}
