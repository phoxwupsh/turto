use std::{collections::HashMap, fmt::Display};

#[derive(Debug)]
pub struct Template {
    total_len: usize,
    tokens: Vec<Token>,
}

pub struct Renderer<'a> {
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
            total_len: template.len(),
            tokens,
        })
    }

    pub fn get_renderer(&self) -> Renderer {
        Renderer {
            template: self,
            args: HashMap::<_, _>::new(),
        }
    }
}

impl<'a> Renderer<'a> {
    pub fn render_string(&self) -> String {
        let mut res = String::with_capacity(self.template.total_len);
        self.template.tokens.iter().for_each(|token| match token {
            Token::Text(t) => res.push_str(t),
            Token::Arg(a) => {
                if let Some(s) = self.args.get(a.as_str()) {
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
