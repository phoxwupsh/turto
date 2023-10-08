use std::{collections::HashMap, fmt::Display};

#[derive(Debug)]
pub struct Template {
    length: usize,
    tokens: Vec<Token>,
}

pub struct TemplateRenderer<'a> {
    template: &'a Template,
    args: HashMap<&'a str, &'a dyn Display>,
}

#[derive(Debug)]
enum Token {
    Text(String),
    Arg(String),
}

impl Template {
    pub fn parse(template: &str) -> Result<Self, TemplateParseError> {
        let mut tokens = Vec::<Token>::new();
        let mut acc = String::new();
        let mut is_arg = false;
        for (i, c) in template.chars().enumerate() {
            match c {
                '{' => {
                    if acc.ends_with('\\') {
                        let _ = acc.pop();
                        acc.push(c);
                    } else if !is_arg {
                        is_arg = true;
                        if !acc.is_empty() {
                            let token = Token::Text(acc.clone());
                            tokens.push(token);
                            acc.clear();
                        }
                    } else {
                        return Err(TemplateParseError { index: i });
                    }
                }
                '}' => {
                    if acc.ends_with('\\') {
                        let _ = acc.pop();
                        acc.push(c);
                    } else if is_arg {
                        let token = Token::Arg(acc.clone());
                        tokens.push(token);
                        acc.clear();
                        is_arg = false
                    } else {
                        return Err(TemplateParseError { index: i });
                    }
                }
                _ => {
                    acc.push(c);
                }
            }
        }
        if !acc.is_empty() {
            tokens.push(Token::Text(acc));
        }
        Ok(Template {
            length: template.len(),
            tokens,
        })
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

#[derive(Debug)]
pub struct TemplateParseError {
    pub index: usize,
}

impl Display for TemplateParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("Unexpected token at position {}", self.index))
    }
}
