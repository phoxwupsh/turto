use std::{collections::HashMap, fmt::Display};

#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq, Eq))]
pub struct Template {
    length: usize,
    tokens: Vec<Token>,
}

pub struct TemplateRenderer<'a> {
    template: &'a Template,
    args: HashMap<&'a str, &'a dyn Display>,
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
        Template {
            length: template.len(),
            tokens,
        }
    }

    pub fn renderer(&self) -> TemplateRenderer {
        TemplateRenderer {
            template: self,
            args: HashMap::<_, _>::new(),
        }
    }
}

impl<'a> TemplateRenderer<'a> {
    pub fn render(&self) -> String {
        let mut res = String::with_capacity(self.template.length);
        self.template.tokens.iter().for_each(|token| match token {
            Token::Text(text) => res.push_str(text),
            Token::Arg(arg) => {
                if let Some(s) = self.args.get(arg.as_str()) {
                    res.push_str(s.to_string().as_str());
                }
            }
        });
        res
    }

    pub fn add_arg(&mut self, key: &'a str, value: &'a dyn Display) -> &mut Self {
        self.args.insert(key, value);
        self
    }
}

#[cfg(test)]
mod test {
    use crate::utils::template::Token;

    use super::Template;

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
                length: 12,
                tokens: vec![Token::Text("Hello World!".to_string())]
            }
        );
        assert_eq!(
            Template::parse(single_arg),
            Template {
                length: 13,
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
                length: 26,
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
                length: 19,
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
                length: 19,
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
