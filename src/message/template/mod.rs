use std::{borrow::Cow, collections::HashMap, fmt::Display, path::Path, str::FromStr};
use strum::IntoEnumIterator;

use crate::message::template::names::TemplateName;

pub mod names;

#[derive(Debug)]
pub struct Templates {
    default: HashMap<TemplateName, Template>,
    locales: HashMap<String, HashMap<TemplateName, Template>>,
}

impl Templates {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, toml::de::Error> {
        let path = path.as_ref();
        let templates_str = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(error) => {
                tracing::warn!(%error, path = %path.display(), "failed to read tempaltes, will use default templates");
                return Ok(Templates::default());
            }
        };
        let templates_map =
            toml::from_str::<HashMap<String, HashMap<String, String>>>(&templates_str)?;

        // transform raw data
        let mut locales = HashMap::new();
        for (locale, map) in templates_map {
            let mut locale_map = HashMap::new();
            for (name, template_str) in map {
                let Ok(template_name) = TemplateName::from_str(&name) else {
                    tracing::warn!(locale ,%name, "unknown template name ignored");
                    continue;
                };
                let template = Template::parse(&template_str);
                locale_map.insert(template_name, template);
            }
            // case insensitive for locale ID
            locales.insert(locale.to_ascii_lowercase(), locale_map);
        }

        Ok(Self {
            default: Self::build_default(),
            locales,
        })
    }

    pub fn get_with_fallback<'a>(
        &'a self,
        template_name: TemplateName,
        locale: Option<&str>,
    ) -> &'a Template {
        if let Some(locale) = locale
            && let Some(template_locale) = self.locales.get(locale)
            && let Some(template) = template_locale.get(&template_name)
        {
            template
        } else {
            self.default.get(&template_name).unwrap()
        }
    }

    fn build_default() -> HashMap<TemplateName, Template> {
        TemplateName::iter()
            .map(|template_name| (template_name, template_name.default_template()))
            .collect()
    }
}

impl Default for Templates {
    fn default() -> Self {
        Self {
            default: Self::build_default(),
            locales: HashMap::new(),
        }
    }
}

#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq, Eq))]
pub struct Template {
    tokens: Vec<Token>,
}

#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq, Eq))]
enum Token {
    Text(String),
    Arg(String),
}

impl Template {
    pub fn parse(template: &str) -> Self {
        let mut start = 0;
        let mut pos = 0;
        let mut last_left = 0;
        let mut tokens = Vec::new();
        let mut has_left = false;
        for c in template.chars() {
            match c {
                '{' => {
                    last_left = pos;
                    has_left = true;
                }
                '}' => {
                    if has_left {
                        let text = template[start..last_left].to_string();
                        let arg = template[last_left + 1..pos].to_string();
                        tokens.push(Token::Text(text));
                        tokens.push(Token::Arg(arg));
                        start = pos + 1;
                        last_left = start;
                        has_left = false;
                    }
                }
                _ => (),
            }
            pos += c.len_utf8();
        }
        let remainder = template[start..].to_string();
        tokens.push(Token::Text(remainder));
        Template { tokens }
    }

    pub fn renderer(&'_ self) -> TemplateRenderer<'_> {
        TemplateRenderer {
            template: self,
            args: HashMap::new(),
        }
    }
}

pub struct TemplateRenderer<'a> {
    template: &'a Template,
    args: HashMap<&'a str, Cow<'a, str>>,
}

impl<'a> TemplateRenderer<'a> {
    pub fn render_iter(&self) -> impl Iterator<Item = &str> {
        self.template.tokens.iter().filter_map(|token| match token {
            Token::Text(text) => Some(text.as_str()),
            Token::Arg(arg) => self.args.get(arg.as_str()).map(AsRef::as_ref),
        })
    }

    pub fn render(&self) -> String {
        self.render_iter().collect()
    }

    pub fn add_arg(&mut self, key: &'a str, value: impl Into<DisplayRef<'a>>) {
        self.args.insert(key, value.into().0);
    }
}

pub struct DisplayRef<'a>(Cow<'a, str>);

impl<'a> From<&'a str> for DisplayRef<'a> {
    fn from(value: &'a str) -> Self {
        Self(Cow::Borrowed(value))
    }
}

impl From<String> for DisplayRef<'_> {
    fn from(value: String) -> Self {
        Self(Cow::Owned(value))
    }
}

impl<T> From<&T> for DisplayRef<'_>
where
    T: Display,
{
    fn from(value: &T) -> Self {
        Self(Cow::Owned(value.to_string()))
    }
}

#[cfg(test)]
mod test {
    use super::{Template, Token};

    #[test]
    fn test_template_parse() {
        let no_arg = "Hello World!";
        let single_arg = "Hello {name}!";
        let multi_arg = "Hello {name1} and {name2}!";
        let brackets = "Hello {{{{name}}}}!";
        let unclosed = "}}{{arg1}{{{arg2}}{";
        assert_eq!(
            Template::parse(no_arg),
            Template {
                tokens: vec![Token::Text("Hello World!".to_string())]
            }
        );
        assert_eq!(
            Template::parse(single_arg),
            Template {
                tokens: vec![
                    Token::Text("Hello ".to_string()),
                    Token::Arg("name".to_string()),
                    Token::Text("!".to_string())
                ]
            }
        );
        assert_eq!(
            Template::parse(multi_arg),
            Template {
                tokens: vec![
                    Token::Text("Hello ".to_string()),
                    Token::Arg("name1".to_string()),
                    Token::Text(" and ".to_string()),
                    Token::Arg("name2".to_string()),
                    Token::Text("!".to_string())
                ]
            }
        );
        assert_eq!(
            Template::parse(brackets),
            Template {
                tokens: vec![
                    Token::Text("Hello {{{".to_string()),
                    Token::Arg("name".to_string()),
                    Token::Text("}}}!".to_string())
                ]
            }
        );
        assert_eq!(
            Template::parse(unclosed),
            Template {
                tokens: vec![
                    Token::Text("}}{".to_string()),
                    Token::Arg("arg1".to_string()),
                    Token::Text("{{".to_string()),
                    Token::Arg("arg2".to_string()),
                    Token::Text("}{".to_string())
                ]
            }
        );
    }
}
