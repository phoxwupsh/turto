use crate::utils::template::Template;
use std::{collections::HashMap, fs, sync::OnceLock};
use toml::Table;

pub struct MessageTemplateProvider;

impl MessageTemplateProvider {
    pub fn get_template(template_name: &str) -> &Template {
        static TEMPLATES: OnceLock<HashMap<String, Template>> = OnceLock::new();
        TEMPLATES
            .get_or_init(|| {
                let templates_map = fs::read_to_string("templates.toml")
                    .map_err(|err| panic!("Error loading templates.toml: {err}"))
                    .and_then(|templates_toml| toml::from_str::<Table>(&templates_toml))
                    .unwrap_or_else(|err| panic!("Error parsing templates.toml: {err}"));

                templates_map
                    .into_iter()
                    .map(|(k, v)| {
                        let template_str = v.as_str().unwrap_or_else(|| {
                            panic!("The value of template \"{k}\" should be a string")
                        });
                        let template = Template::parse(template_str)
                            .map_err(|err| panic!("Error parsing template \"{k}\": {err}"))
                            .unwrap();
                        (k, template)
                    })
                    .collect::<HashMap<_, _>>()
            })
            .get(template_name)
            .unwrap_or_else(|| {
                panic!(
                    "Can't find the message template \"{}\" in templates.toml",
                    template_name
                )
            })
    }
}
