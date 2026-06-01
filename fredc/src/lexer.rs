#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Keywords
    Fn,
    Let,
    Return,
    If,
    Else,
    While,
    Loop,
    For,
    From,
    To,
    True,
    False,
    Nil,
    In,
    Switch,
    Case,
    Default,

    // Identifiers and literals
    Id(String),
    Number(f64),
    String(String),
    TemplateString(Vec<TemplateNode>),

    // Operators
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Equal,
    EqualEqual,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    And,
    Or,
    Not,
    Dot,
    Colon,
    Question,

    // Delimiters
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Comma,
    Semicolon,

    Eof,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TemplateNode {
    Text(String),
    Expr(Vec<Token>),
}

pub struct Lexer {
    input: Vec<char>,
    pos: usize,
}

impl Lexer {
    fn new(input: &str) -> Self {
        Lexer {
            input: input.chars().collect(),
            pos: 0,
        }
    }

    fn peek(&self) -> Option<char> {
        if self.pos < self.input.len() {
            Some(self.input[self.pos])
        } else {
            None
        }
    }

    fn peek_ahead(&self, n: usize) -> Option<char> {
        if self.pos + n < self.input.len() {
            Some(self.input[self.pos + n])
        } else {
            None
        }
    }

    fn advance(&mut self) {
        self.pos += 1;
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.advance();
            } else if c == '/' && self.peek_ahead(1) == Some('/') {
                self.advance();
                self.advance();
                while let Some(ch) = self.peek() {
                    if ch == '\n' {
                        break;
                    }
                    self.advance();
                }
            } else {
                break;
            }
        }
    }

    fn read_string(&mut self, quote: char) -> Result<String, String> {
        self.advance();
        let mut result = String::new();
        loop {
            match self.peek() {
                Some(c) if c == quote => {
                    self.advance();
                    return Ok(result);
                }
                Some('\\') => {
                    self.advance();
                    match self.peek() {
                        Some('n') => result.push('\n'),
                        Some('t') => result.push('\t'),
                        Some('r') => result.push('\r'),
                        Some('\\') => result.push('\\'),
                        Some('"') => result.push('"'),
                        Some('\'') => result.push('\''),
                        Some(c) => result.push(c),
                        None => return Err("Unterminated string".to_string()),
                    }
                    self.advance();
                }
                Some(c) => {
                    result.push(c);
                    self.advance();
                }
                None => return Err("Unterminated string".to_string()),
            }
        }
    }

    fn read_number(&mut self) -> f64 {
        let mut num_str = String::new();
        while let Some(c) = self.peek() {
            if c.is_numeric() || c == '.' {
                num_str.push(c);
                self.advance();
            } else {
                break;
            }
        }
        num_str.parse().unwrap_or(0.0)
    }

    fn read_ident(&mut self) -> String {
        let mut ident = String::new();
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' {
                ident.push(c);
                self.advance();
            } else {
                break;
            }
        }
        ident
    }

    fn read_template_string(&mut self) -> Result<Token, String> {
        self.advance();
        let mut nodes = Vec::new();
        let mut text = String::new();

        loop {
            match self.peek() {
                None => return Err("Unterminated template string".to_string()),
                Some('`') => {
                    if !text.is_empty() {
                        nodes.push(TemplateNode::Text(text));
                    }
                    self.advance();
                    return Ok(Token::TemplateString(nodes));
                }
                Some('$') if self.peek_ahead(1) == Some('{') => {
                    if !text.is_empty() {
                        nodes.push(TemplateNode::Text(text.clone()));
                        text.clear();
                    }
                    self.advance();
                    self.advance();
                    let mut expr_str = String::new();
                    let mut depth = 1;
                    while depth > 0 {
                        match self.peek() {
                            None => return Err("Unterminated template expression".to_string()),
                            Some('{') => {
                                expr_str.push('{');
                                depth += 1;
                                self.advance();
                            }
                            Some('}') => {
                                depth -= 1;
                                if depth > 0 {
                                    expr_str.push('}');
                                }
                                self.advance();
                            }
                            Some(c) => {
                                expr_str.push(c);
                                self.advance();
                            }
                        }
                    }
                    let expr_tokens = tokenize(&expr_str)?;
                    nodes.push(TemplateNode::Expr(expr_tokens));
                }
                Some(c) => {
                    text.push(c);
                    self.advance();
                }
            }
        }
    }

    fn next_token(&mut self) -> Result<Token, String> {
        self.skip_whitespace();

        match self.peek() {
            None => Ok(Token::Eof),
            Some(c) if c.is_alphabetic() || c == '_' => {
                let ident = self.read_ident();
                Ok(match ident.as_str() {
                    "fn" => Token::Fn,
                    "let" => Token::Let,
                    "return" => Token::Return,
                    "if" => Token::If,
                    "else" => Token::Else,
                    "while" => Token::While,
                    "loop" => Token::Loop,
                    "for" => Token::For,
                    "from" => Token::From,
                    "to" => Token::To,
                    "true" => Token::True,
                    "false" => Token::False,
                    "nil" => Token::Nil,
                    "and" => Token::And,
                    "or" => Token::Or,
                    "not" => Token::Not,
                    "in" => Token::In,
                    "switch" => Token::Switch,
                    "case" => Token::Case,
                    "default" => Token::Default,
                    _ => Token::Id(ident),
                })
            }
            Some(c) if c.is_numeric() => {
                let num = self.read_number();
                Ok(Token::Number(num))
            }
            Some('"') => self.read_string('"').map(Token::String),
            Some('\'') => self.read_string('\'').map(Token::String),
            Some('`') => self.read_template_string(),
            Some('+') => {
                self.advance();
                Ok(Token::Plus)
            }
            Some('-') => {
                self.advance();
                Ok(Token::Minus)
            }
            Some('*') => {
                self.advance();
                Ok(Token::Star)
            }
            Some('/') => {
                self.advance();
                Ok(Token::Slash)
            }
            Some('%') => {
                self.advance();
                Ok(Token::Percent)
            }
            Some('=') => {
                self.advance();
                if self.peek() == Some('=') {
                    self.advance();
                    Ok(Token::EqualEqual)
                } else {
                    Ok(Token::Equal)
                }
            }
            Some('!') => {
                self.advance();
                if self.peek() == Some('=') {
                    self.advance();
                    Ok(Token::NotEqual)
                } else {
                    Ok(Token::Not)
                }
            }
            Some('<') => {
                self.advance();
                if self.peek() == Some('=') {
                    self.advance();
                    Ok(Token::LessEqual)
                } else {
                    Ok(Token::Less)
                }
            }
            Some('>') => {
                self.advance();
                if self.peek() == Some('=') {
                    self.advance();
                    Ok(Token::GreaterEqual)
                } else {
                    Ok(Token::Greater)
                }
            }
            Some('~') => {
                self.advance();
                if self.peek() == Some('=') {
                    self.advance();
                    Ok(Token::NotEqual)
                } else {
                    Err("Unexpected character: ~".to_string())
                }
            }
            Some('(') => {
                self.advance();
                Ok(Token::LParen)
            }
            Some(')') => {
                self.advance();
                Ok(Token::RParen)
            }
            Some('{') => {
                self.advance();
                Ok(Token::LBrace)
            }
            Some('}') => {
                self.advance();
                Ok(Token::RBrace)
            }
            Some('[') => {
                self.advance();
                Ok(Token::LBracket)
            }
            Some(']') => {
                self.advance();
                Ok(Token::RBracket)
            }
            Some(',') => {
                self.advance();
                Ok(Token::Comma)
            }
            Some(';') => {
                self.advance();
                Ok(Token::Semicolon)
            }
            Some('.') => {
                self.advance();
                Ok(Token::Dot)
            }
            Some(':') => {
                self.advance();
                Ok(Token::Colon)
            }
            Some('?') => {
                self.advance();
                Ok(Token::Question)
            }
            Some(c) => Err(format!("Unexpected character: {}", c)),
        }
    }
}

pub fn tokenize(input: &str) -> Result<Vec<Token>, String> {
    let mut lexer = Lexer::new(input);
    let mut tokens = Vec::new();

    loop {
        let token = lexer.next_token()?;
        let is_eof = token == Token::Eof;
        tokens.push(token);
        if is_eof {
            break;
        }
    }

    Ok(tokens)
}
