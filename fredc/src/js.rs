//! Hand-written JavaScript -> .fred transpiler.
//!
//! Replaces the old node/CASTL dependency. Parses a modern-JS subset and emits
//! .fred *source* (brace syntax), which then flows through the normal
//! lexer -> parser -> validator -> codegen pipeline. We emit fred text (rather
//! than the AST directly) so `--to-fred` can show the translation and it stays
//! easy to eyeball.
//!
//! Supported subset: let/const/var, function declarations, arrow & function
//! expressions, if/else-if/else, while, C-style for (lowered to while),
//! for...of (-> for-in), return, break, the usual operators (=== -> ==,
//! && -> and, etc.), template literals, arrays, method/'.' calls, ternary,
//! console.log -> print, Math.*, and common String/Array methods.
//!
//! NOT supported (errors or won't behave): objects/dicts, classes, regex,
//! continue, destructuring, spread, async/generators, labelled loops.

// ----------------------------- Lexer ---------------------------------------

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    // keywords
    Let,
    Function,
    Return,
    If,
    Else,
    While,
    For,
    Of,
    Break,
    True,
    False,
    Null,
    // literals / ids
    Id(String),
    Num(f64),
    Str(String),
    Template(Vec<TmplTok>),
    // operators
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Assign,
    PlusEq,
    MinusEq,
    StarEq,
    SlashEq,
    EqEq,  // == or ===
    NotEq, // != or !==
    Lt,
    Le,
    Gt,
    Ge,
    AndAnd,
    OrOr,
    Bang,
    Question,
    Colon,
    Dot,
    Comma,
    Semi,
    Arrow, // =>
    Inc,   // ++
    Dec,   // --
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Eof,
}

#[derive(Debug, Clone, PartialEq)]
enum TmplTok {
    Text(String),
    Expr(Vec<Tok>),
}

struct JsLexer {
    src: Vec<char>,
    pos: usize,
}

impl JsLexer {
    fn new(s: &str) -> Self {
        JsLexer {
            src: s.chars().collect(),
            pos: 0,
        }
    }
    fn peek(&self) -> Option<char> {
        self.src.get(self.pos).copied()
    }
    fn peek2(&self) -> Option<char> {
        self.src.get(self.pos + 1).copied()
    }
    fn bump(&mut self) -> Option<char> {
        let c = self.peek();
        self.pos += 1;
        c
    }

    fn skip_trivia(&mut self) {
        loop {
            match self.peek() {
                Some(c) if c.is_whitespace() => {
                    self.pos += 1;
                }
                Some('/') if self.peek2() == Some('/') => {
                    while let Some(c) = self.peek() {
                        if c == '\n' {
                            break;
                        }
                        self.pos += 1;
                    }
                }
                Some('/') if self.peek2() == Some('*') => {
                    self.pos += 2;
                    while let Some(c) = self.peek() {
                        if c == '*' && self.peek2() == Some('/') {
                            self.pos += 2;
                            break;
                        }
                        self.pos += 1;
                    }
                }
                _ => break,
            }
        }
    }

    fn read_string(&mut self, q: char) -> Result<String, String> {
        self.pos += 1; // opening quote
        let mut out = String::new();
        loop {
            match self.bump() {
                Some(c) if c == q => return Ok(out),
                Some('\\') => match self.bump() {
                    Some('n') => out.push('\n'),
                    Some('t') => out.push('\t'),
                    Some('r') => out.push('\r'),
                    Some('\\') => out.push('\\'),
                    Some('\'') => out.push('\''),
                    Some('"') => out.push('"'),
                    Some('`') => out.push('`'),
                    Some(c) => out.push(c),
                    None => return Err("Unterminated string".into()),
                },
                Some(c) => out.push(c),
                None => return Err("Unterminated string".into()),
            }
        }
    }

    fn read_template(&mut self) -> Result<Tok, String> {
        self.pos += 1; // opening backtick
        let mut parts = Vec::new();
        let mut text = String::new();
        loop {
            match self.peek() {
                None => return Err("Unterminated template literal".into()),
                Some('`') => {
                    self.pos += 1;
                    if !text.is_empty() {
                        parts.push(TmplTok::Text(text));
                    }
                    return Ok(Tok::Template(parts));
                }
                Some('\\') => {
                    self.pos += 1;
                    if let Some(c) = self.bump() {
                        match c {
                            'n' => text.push('\n'),
                            't' => text.push('\t'),
                            _ => text.push(c),
                        }
                    }
                }
                Some('$') if self.peek2() == Some('{') => {
                    if !text.is_empty() {
                        parts.push(TmplTok::Text(std::mem::take(&mut text)));
                    }
                    self.pos += 2;
                    let mut depth = 1;
                    let mut sub = String::new();
                    while depth > 0 {
                        match self.bump() {
                            None => return Err("Unterminated template expression".into()),
                            Some('{') => {
                                depth += 1;
                                sub.push('{');
                            }
                            Some('}') => {
                                depth -= 1;
                                if depth > 0 {
                                    sub.push('}');
                                }
                            }
                            Some(c) => sub.push(c),
                        }
                    }
                    parts.push(TmplTok::Expr(tokenize(&sub)?));
                }
                Some(c) => {
                    text.push(c);
                    self.pos += 1;
                }
            }
        }
    }

    fn read_number(&mut self) -> f64 {
        let mut s = String::new();
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() || c == '.' {
                s.push(c);
                self.pos += 1;
            } else {
                break;
            }
        }
        s.parse().unwrap_or(0.0)
    }

    fn read_ident(&mut self) -> String {
        let mut s = String::new();
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' || c == '$' {
                s.push(c);
                self.pos += 1;
            } else {
                break;
            }
        }
        s
    }

    fn next(&mut self) -> Result<Tok, String> {
        self.skip_trivia();
        let c = match self.peek() {
            None => return Ok(Tok::Eof),
            Some(c) => c,
        };
        if c.is_alphabetic() || c == '_' || c == '$' {
            let id = self.read_ident();
            return Ok(match id.as_str() {
                "let" | "const" | "var" => Tok::Let,
                "function" => Tok::Function,
                "return" => Tok::Return,
                "if" => Tok::If,
                "else" => Tok::Else,
                "while" => Tok::While,
                "for" => Tok::For,
                "of" => Tok::Of,
                "break" => Tok::Break,
                "true" => Tok::True,
                "false" => Tok::False,
                "null" | "undefined" => Tok::Null,
                _ => Tok::Id(id),
            });
        }
        if c.is_ascii_digit() {
            return Ok(Tok::Num(self.read_number()));
        }
        match c {
            '"' => return self.read_string('"').map(Tok::Str),
            '\'' => return self.read_string('\'').map(Tok::Str),
            '`' => return self.read_template(),
            _ => {}
        }
        // operators / punctuation
        self.pos += 1;
        let two = self.peek();
        let tok = match c {
            '+' => match two {
                Some('+') => {
                    self.pos += 1;
                    Tok::Inc
                }
                Some('=') => {
                    self.pos += 1;
                    Tok::PlusEq
                }
                _ => Tok::Plus,
            },
            '-' => match two {
                Some('-') => {
                    self.pos += 1;
                    Tok::Dec
                }
                Some('=') => {
                    self.pos += 1;
                    Tok::MinusEq
                }
                _ => Tok::Minus,
            },
            '*' => {
                if two == Some('=') {
                    self.pos += 1;
                    Tok::StarEq
                } else {
                    Tok::Star
                }
            }
            '/' => {
                if two == Some('=') {
                    self.pos += 1;
                    Tok::SlashEq
                } else {
                    Tok::Slash
                }
            }
            '%' => Tok::Percent,
            '=' => match two {
                Some('=') => {
                    self.pos += 1;
                    if self.peek() == Some('=') {
                        self.pos += 1;
                    }
                    Tok::EqEq
                }
                Some('>') => {
                    self.pos += 1;
                    Tok::Arrow
                }
                _ => Tok::Assign,
            },
            '!' => {
                if two == Some('=') {
                    self.pos += 1;
                    if self.peek() == Some('=') {
                        self.pos += 1;
                    }
                    Tok::NotEq
                } else {
                    Tok::Bang
                }
            }
            '<' => {
                if two == Some('=') {
                    self.pos += 1;
                    Tok::Le
                } else {
                    Tok::Lt
                }
            }
            '>' => {
                if two == Some('=') {
                    self.pos += 1;
                    Tok::Ge
                } else {
                    Tok::Gt
                }
            }
            '&' => {
                if two == Some('&') {
                    self.pos += 1;
                    Tok::AndAnd
                } else {
                    return Err("Unexpected '&' (bitwise ops unsupported)".into());
                }
            }
            '|' => {
                if two == Some('|') {
                    self.pos += 1;
                    Tok::OrOr
                } else {
                    return Err("Unexpected '|' (bitwise ops unsupported)".into());
                }
            }
            '?' => Tok::Question,
            ':' => Tok::Colon,
            '.' => Tok::Dot,
            ',' => Tok::Comma,
            ';' => Tok::Semi,
            '(' => Tok::LParen,
            ')' => Tok::RParen,
            '{' => Tok::LBrace,
            '}' => Tok::RBrace,
            '[' => Tok::LBracket,
            ']' => Tok::RBracket,
            other => return Err(format!("Unexpected character: {}", other)),
        };
        Ok(tok)
    }
}

fn tokenize(s: &str) -> Result<Vec<Tok>, String> {
    let mut lx = JsLexer::new(s);
    let mut out = Vec::new();
    loop {
        let t = lx.next()?;
        let eof = t == Tok::Eof;
        out.push(t);
        if eof {
            break;
        }
    }
    Ok(out)
}

// ------------------------------- AST ---------------------------------------

enum Expr {
    Num(f64),
    Str(String),
    Tmpl(Vec<TmplPart>),
    Bool(bool),
    Null,
    Ident(String),
    Array(Vec<Expr>),
    Object(Vec<(String, Expr)>),
    Unary(String, Box<Expr>),
    Bin(String, Box<Expr>, Box<Expr>),
    Ternary(Box<Expr>, Box<Expr>, Box<Expr>),
    Call(Box<Expr>, Vec<Expr>),
    Member(Box<Expr>, String),
    Index(Box<Expr>, Box<Expr>),
    Func(Vec<String>, Vec<Stmt>),
}

enum TmplPart {
    Text(String),
    Expr(Box<Expr>),
}

enum Stmt {
    // (name, value). value=None means bare `let x`.
    Var(String, Option<Expr>),
    // named function definition -> fred `fn`
    Func(String, Vec<String>, Vec<Stmt>),
    Return(Option<Expr>),
    If(Expr, Vec<Stmt>, Option<Vec<Stmt>>),
    While(Expr, Vec<Stmt>),
    // classic for: init ; cond ; update ; body  (lowered to while on emit)
    For(
        Option<Box<Stmt>>,
        Option<Expr>,
        Option<Box<Stmt>>,
        Vec<Stmt>,
    ),
    ForOf(String, Expr, Vec<Stmt>),
    Break,
    // assignment statement: target, op ("=","+=",...), value
    Assign(Expr, String, Expr),
    Expr(Expr),
}

// ------------------------------ Parser -------------------------------------

struct Parser {
    toks: Vec<Tok>,
    pos: usize,
}

impl Parser {
    fn new(toks: Vec<Tok>) -> Self {
        Parser { toks, pos: 0 }
    }
    fn peek(&self) -> &Tok {
        self.toks.get(self.pos).unwrap_or(&Tok::Eof)
    }
    fn peek_at(&self, n: usize) -> &Tok {
        self.toks.get(self.pos + n).unwrap_or(&Tok::Eof)
    }
    fn bump(&mut self) -> Tok {
        let t = self.peek().clone();
        self.pos += 1;
        t
    }
    fn eat(&mut self, t: &Tok) -> Result<(), String> {
        if std::mem::discriminant(self.peek()) == std::mem::discriminant(t) {
            self.pos += 1;
            Ok(())
        } else {
            Err(format!("Expected {:?}, got {:?}", t, self.peek()))
        }
    }
    fn opt_semi(&mut self) {
        while self.peek() == &Tok::Semi {
            self.pos += 1;
        }
    }

    fn program(&mut self) -> Result<Vec<Stmt>, String> {
        let mut out = Vec::new();
        self.opt_semi();
        while self.peek() != &Tok::Eof {
            out.push(self.stmt()?);
            self.opt_semi();
        }
        Ok(out)
    }

    fn block(&mut self) -> Result<Vec<Stmt>, String> {
        self.eat(&Tok::LBrace)?;
        let mut out = Vec::new();
        self.opt_semi();
        while self.peek() != &Tok::RBrace && self.peek() != &Tok::Eof {
            out.push(self.stmt()?);
            self.opt_semi();
        }
        self.eat(&Tok::RBrace)?;
        Ok(out)
    }

    fn stmt(&mut self) -> Result<Stmt, String> {
        match self.peek() {
            Tok::Let => self.var_decl(),
            Tok::Function => self.func_decl(),
            Tok::Return => {
                self.bump();
                if matches!(self.peek(), Tok::Semi | Tok::RBrace | Tok::Eof) {
                    Ok(Stmt::Return(None))
                } else {
                    Ok(Stmt::Return(Some(self.expr()?)))
                }
            }
            Tok::If => self.if_stmt(),
            Tok::While => {
                self.bump();
                self.eat(&Tok::LParen)?;
                let cond = self.expr()?;
                self.eat(&Tok::RParen)?;
                let body = self.block()?;
                Ok(Stmt::While(cond, body))
            }
            Tok::For => self.for_stmt(),
            Tok::Break => {
                self.bump();
                Ok(Stmt::Break)
            }
            _ => self.expr_or_assign_stmt(),
        }
    }

    fn var_decl(&mut self) -> Result<Stmt, String> {
        self.bump(); // let/const/var
                     // First declarator becomes the returned stmt; extra ones are folded in
                     // by re-emitting — but to keep stmt() single-valued we only support one
                     // declarator per statement (the common case). Multiple are rare.
        let name = self.ident()?;
        let value = if self.peek() == &Tok::Assign {
            self.bump();
            Some(self.assignment()?)
        } else {
            None
        };
        if self.peek() == &Tok::Comma {
            return Err("multiple declarators in one `let` are unsupported; split them".into());
        }
        Ok(Stmt::Var(name, value))
    }

    fn func_decl(&mut self) -> Result<Stmt, String> {
        self.bump(); // function
        let name = self.ident()?;
        let params = self.param_list()?;
        let body = self.block()?;
        Ok(Stmt::Func(name, params, body))
    }

    fn if_stmt(&mut self) -> Result<Stmt, String> {
        self.bump(); // if
        self.eat(&Tok::LParen)?;
        let cond = self.expr()?;
        self.eat(&Tok::RParen)?;
        let then_body = self.block()?;
        let else_body = if self.peek() == &Tok::Else {
            self.bump();
            if self.peek() == &Tok::If {
                // else-if: wrap the nested if as a one-statement else block.
                Some(vec![self.if_stmt()?])
            } else {
                Some(self.block()?)
            }
        } else {
            None
        };
        Ok(Stmt::If(cond, then_body, else_body))
    }

    fn for_stmt(&mut self) -> Result<Stmt, String> {
        self.bump(); // for
        self.eat(&Tok::LParen)?;

        // Detect `for (let x of iter)` / `for (x of iter)`.
        let save = self.pos;
        let is_for_of = {
            if self.peek() == &Tok::Let {
                matches!(self.peek_at(1), Tok::Id(_)) && self.peek_at(2) == &Tok::Of
            } else {
                matches!(self.peek(), Tok::Id(_)) && self.peek_at(1) == &Tok::Of
            }
        };
        if is_for_of {
            if self.peek() == &Tok::Let {
                self.bump();
            }
            let var = self.ident()?;
            self.eat(&Tok::Of)?;
            let iter = self.expr()?;
            self.eat(&Tok::RParen)?;
            let body = self.block()?;
            return Ok(Stmt::ForOf(var, iter, body));
        }
        self.pos = save;

        // Classic C-style for(init; cond; update)
        let init = if self.peek() == &Tok::Semi {
            None
        } else {
            Some(Box::new(self.simple_for_clause()?))
        };
        self.eat(&Tok::Semi)?;
        let cond = if self.peek() == &Tok::Semi {
            None
        } else {
            Some(self.expr()?)
        };
        self.eat(&Tok::Semi)?;
        let update = if self.peek() == &Tok::RParen {
            None
        } else {
            Some(Box::new(self.simple_for_clause()?))
        };
        self.eat(&Tok::RParen)?;
        let body = self.block()?;
        Ok(Stmt::For(init, cond, update, body))
    }

    // init / update clause inside a for-header: a var decl or an assign/expr,
    // with no trailing semicolon consumed.
    fn simple_for_clause(&mut self) -> Result<Stmt, String> {
        if self.peek() == &Tok::Let {
            self.bump();
            let name = self.ident()?;
            self.eat(&Tok::Assign)?;
            let value = self.assignment()?;
            Ok(Stmt::Var(name, Some(value)))
        } else {
            self.expr_or_assign_stmt()
        }
    }

    fn expr_or_assign_stmt(&mut self) -> Result<Stmt, String> {
        let lhs = self.expr()?;
        let op = match self.peek() {
            Tok::Assign => Some("="),
            Tok::PlusEq => Some("+="),
            Tok::MinusEq => Some("-="),
            Tok::StarEq => Some("*="),
            Tok::SlashEq => Some("/="),
            _ => None,
        };
        if let Some(op) = op {
            self.bump();
            let rhs = self.assignment()?;
            return Ok(Stmt::Assign(lhs, op.to_string(), rhs));
        }
        // postfix ++/-- as a statement -> i = i +/- 1
        match self.peek() {
            Tok::Inc => {
                self.bump();
                return Ok(Stmt::Assign(
                    clone_lvalue(&lhs)?,
                    "+=".into(),
                    Expr::Num(1.0),
                ));
            }
            Tok::Dec => {
                self.bump();
                return Ok(Stmt::Assign(
                    clone_lvalue(&lhs)?,
                    "-=".into(),
                    Expr::Num(1.0),
                ));
            }
            _ => {}
        }
        Ok(Stmt::Expr(lhs))
    }

    fn ident(&mut self) -> Result<String, String> {
        match self.bump() {
            Tok::Id(s) => Ok(s),
            other => Err(format!("Expected identifier, got {:?}", other)),
        }
    }

    fn param_list(&mut self) -> Result<Vec<String>, String> {
        self.eat(&Tok::LParen)?;
        let mut params = Vec::new();
        while self.peek() != &Tok::RParen {
            params.push(self.ident()?);
            if self.peek() == &Tok::Comma {
                self.bump();
            }
        }
        self.eat(&Tok::RParen)?;
        Ok(params)
    }

    // ---- expressions ----

    fn expr(&mut self) -> Result<Expr, String> {
        self.assignment()
    }

    // We don't support assignment-as-expression in fred, but `=` may appear via
    // simple_for_clause/assign stmt which call assignment() for the RHS only.
    fn assignment(&mut self) -> Result<Expr, String> {
        self.ternary()
    }

    fn ternary(&mut self) -> Result<Expr, String> {
        let cond = self.logic_or()?;
        if self.peek() == &Tok::Question {
            self.bump();
            let a = self.assignment()?;
            self.eat(&Tok::Colon)?;
            let b = self.assignment()?;
            return Ok(Expr::Ternary(Box::new(cond), Box::new(a), Box::new(b)));
        }
        Ok(cond)
    }

    fn logic_or(&mut self) -> Result<Expr, String> {
        let mut left = self.logic_and()?;
        while self.peek() == &Tok::OrOr {
            self.bump();
            let right = self.logic_and()?;
            left = Expr::Bin("or".into(), Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn logic_and(&mut self) -> Result<Expr, String> {
        let mut left = self.equality()?;
        while self.peek() == &Tok::AndAnd {
            self.bump();
            let right = self.equality()?;
            left = Expr::Bin("and".into(), Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn equality(&mut self) -> Result<Expr, String> {
        let mut left = self.relational()?;
        loop {
            let op = match self.peek() {
                Tok::EqEq => "==",
                Tok::NotEq => "!=",
                _ => break,
            };
            self.bump();
            let right = self.relational()?;
            left = Expr::Bin(op.into(), Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn relational(&mut self) -> Result<Expr, String> {
        let mut left = self.additive()?;
        loop {
            let op = match self.peek() {
                Tok::Lt => "<",
                Tok::Le => "<=",
                Tok::Gt => ">",
                Tok::Ge => ">=",
                _ => break,
            };
            self.bump();
            let right = self.additive()?;
            left = Expr::Bin(op.into(), Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn additive(&mut self) -> Result<Expr, String> {
        let mut left = self.multiplicative()?;
        loop {
            let op = match self.peek() {
                Tok::Plus => "+",
                Tok::Minus => "-",
                _ => break,
            };
            self.bump();
            let right = self.multiplicative()?;
            left = Expr::Bin(op.into(), Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn multiplicative(&mut self) -> Result<Expr, String> {
        let mut left = self.unary()?;
        loop {
            let op = match self.peek() {
                Tok::Star => "*",
                Tok::Slash => "/",
                Tok::Percent => "%",
                _ => break,
            };
            self.bump();
            let right = self.unary()?;
            left = Expr::Bin(op.into(), Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn unary(&mut self) -> Result<Expr, String> {
        match self.peek() {
            Tok::Bang => {
                self.bump();
                Ok(Expr::Unary("!".into(), Box::new(self.unary()?)))
            }
            Tok::Minus => {
                self.bump();
                Ok(Expr::Unary("-".into(), Box::new(self.unary()?)))
            }
            Tok::Plus => {
                self.bump();
                self.unary()
            }
            _ => self.postfix(),
        }
    }

    fn postfix(&mut self) -> Result<Expr, String> {
        let mut expr = self.primary()?;
        loop {
            match self.peek() {
                Tok::Dot => {
                    self.bump();
                    let name = self.ident()?;
                    expr = Expr::Member(Box::new(expr), name);
                }
                Tok::LBracket => {
                    self.bump();
                    let idx = self.expr()?;
                    self.eat(&Tok::RBracket)?;
                    expr = Expr::Index(Box::new(expr), Box::new(idx));
                }
                Tok::LParen => {
                    let args = self.arg_list()?;
                    expr = Expr::Call(Box::new(expr), args);
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn arg_list(&mut self) -> Result<Vec<Expr>, String> {
        self.eat(&Tok::LParen)?;
        let mut args = Vec::new();
        while self.peek() != &Tok::RParen {
            args.push(self.assignment()?);
            if self.peek() == &Tok::Comma {
                self.bump();
            }
        }
        self.eat(&Tok::RParen)?;
        Ok(args)
    }

    fn primary(&mut self) -> Result<Expr, String> {
        match self.peek().clone() {
            Tok::Num(n) => {
                self.bump();
                Ok(Expr::Num(n))
            }
            Tok::Str(s) => {
                self.bump();
                Ok(Expr::Str(s))
            }
            Tok::True => {
                self.bump();
                Ok(Expr::Bool(true))
            }
            Tok::False => {
                self.bump();
                Ok(Expr::Bool(false))
            }
            Tok::Null => {
                self.bump();
                Ok(Expr::Null)
            }
            Tok::Template(parts) => {
                self.bump();
                let mut out = Vec::new();
                for p in parts {
                    match p {
                        TmplTok::Text(t) => out.push(TmplPart::Text(t)),
                        TmplTok::Expr(toks) => {
                            let mut sub = Parser::new(toks);
                            out.push(TmplPart::Expr(Box::new(sub.expr()?)));
                        }
                    }
                }
                Ok(Expr::Tmpl(out))
            }
            Tok::Function => {
                self.bump();
                // optional name (ignored for expression form)
                if let Tok::Id(_) = self.peek() {
                    self.bump();
                }
                let params = self.param_list()?;
                let body = self.block()?;
                Ok(Expr::Func(params, body))
            }
            Tok::Id(name) => {
                // single-param arrow:  x => ...
                if self.peek_at(1) == &Tok::Arrow {
                    self.bump(); // id
                    self.bump(); // =>
                    let body = self.arrow_body()?;
                    Ok(Expr::Func(vec![name], body))
                } else {
                    self.bump();
                    Ok(Expr::Ident(name))
                }
            }
            Tok::LParen => {
                if self.paren_is_arrow() {
                    let params = self.param_list()?;
                    self.eat(&Tok::Arrow)?;
                    let body = self.arrow_body()?;
                    Ok(Expr::Func(params, body))
                } else {
                    self.bump();
                    let e = self.expr()?;
                    self.eat(&Tok::RParen)?;
                    Ok(e)
                }
            }
            Tok::LBracket => {
                self.bump();
                let mut elems = Vec::new();
                while self.peek() != &Tok::RBracket {
                    elems.push(self.assignment()?);
                    if self.peek() == &Tok::Comma {
                        self.bump();
                    }
                }
                self.eat(&Tok::RBracket)?;
                Ok(Expr::Array(elems))
            }
            Tok::LBrace => {
                // object literal: { key: value, ..., shorthand }
                self.bump();
                let mut fields = Vec::new();
                while self.peek() != &Tok::RBrace && self.peek() != &Tok::Eof {
                    let key = match self.peek().clone() {
                        Tok::Id(k) => {
                            self.bump();
                            k
                        }
                        Tok::Str(k) => {
                            self.bump();
                            k
                        }
                        Tok::Num(n) => {
                            self.bump();
                            emit_num(n)
                        }
                        other => return Err(format!("Expected object key, got {:?}", other)),
                    };
                    if self.peek() == &Tok::Colon {
                        self.bump();
                        let val = self.assignment()?;
                        fields.push((key, val));
                    } else {
                        // shorthand { x } -> { x: x }
                        fields.push((key.clone(), Expr::Ident(key)));
                    }
                    if self.peek() == &Tok::Comma {
                        self.bump();
                    }
                }
                self.eat(&Tok::RBrace)?;
                Ok(Expr::Object(fields))
            }
            other => Err(format!("Unexpected token in expression: {:?}", other)),
        }
    }

    // body of an arrow: either a block `{...}` or a single expression (wrapped
    // into `return expr`).
    fn arrow_body(&mut self) -> Result<Vec<Stmt>, String> {
        if self.peek() == &Tok::LBrace {
            self.block()
        } else {
            Ok(vec![Stmt::Return(Some(self.assignment()?))])
        }
    }

    // Lookahead from a `(` to decide if it begins an arrow parameter list,
    // i.e. the matching `)` is immediately followed by `=>`.
    fn paren_is_arrow(&self) -> bool {
        let mut depth = 0;
        let mut i = self.pos;
        loop {
            match self.toks.get(i) {
                None | Some(Tok::Eof) => return false,
                Some(Tok::LParen) => depth += 1,
                Some(Tok::RParen) => {
                    depth -= 1;
                    if depth == 0 {
                        return self.toks.get(i + 1) == Some(&Tok::Arrow);
                    }
                }
                _ => {}
            }
            i += 1;
        }
    }
}

fn clone_lvalue(e: &Expr) -> Result<Expr, String> {
    match e {
        Expr::Ident(n) => Ok(Expr::Ident(n.clone())),
        Expr::Index(o, i) => Ok(Expr::Index(
            Box::new(clone_lvalue(o)?),
            Box::new(clone_lvalue(i)?),
        )),
        Expr::Member(o, n) => Ok(Expr::Member(Box::new(clone_lvalue(o)?), n.clone())),
        Expr::Num(n) => Ok(Expr::Num(*n)),
        _ => Err("invalid target for ++/--".into()),
    }
}

// ------------------------------ Emitter ------------------------------------

fn indent(level: usize) -> String {
    "    ".repeat(level)
}

fn map_method(name: &str) -> &str {
    match name {
        "toUpperCase" => "uppercase",
        "toLowerCase" => "lowercase",
        "charAt" => "char_at",
        // these line up 1:1 with fred and pass through:
        // substring, trim, replace, push, pop, map, filter, reduce, slice,
        // includes, join, length(handled separately)
        other => other,
    }
}

fn emit_block(stmts: &[Stmt], level: usize, out: &mut String) -> Result<(), String> {
    for s in stmts {
        emit_stmt(s, level, out)?;
    }
    Ok(())
}

fn emit_stmt(s: &Stmt, level: usize, out: &mut String) -> Result<(), String> {
    let pad = indent(level);
    match s {
        Stmt::Var(name, value) => {
            match value {
                // const f = (a,b) => ...  /  const f = function(){}  ->  fred fn def
                Some(Expr::Func(params, body)) => {
                    out.push_str(&format!("{}fn {}({}) {{\n", pad, name, params.join(", ")));
                    emit_block(body, level + 1, out)?;
                    out.push_str(&format!("{}}}\n", pad));
                }
                Some(v) => {
                    out.push_str(&format!("{}let {} = {}\n", pad, name, emit_expr(v)?));
                }
                None => out.push_str(&format!("{}let {}\n", pad, name)),
            }
        }
        Stmt::Func(name, params, body) => {
            out.push_str(&format!("{}fn {}({}) {{\n", pad, name, params.join(", ")));
            emit_block(body, level + 1, out)?;
            out.push_str(&format!("{}}}\n", pad));
        }
        Stmt::Return(v) => match v {
            Some(e) => out.push_str(&format!("{}return {}\n", pad, emit_expr(e)?)),
            None => out.push_str(&format!("{}return\n", pad)),
        },
        Stmt::If(cond, then_b, else_b) => {
            out.push_str(&format!("{}if ({}) {{\n", pad, emit_expr(cond)?));
            emit_block(then_b, level + 1, out)?;
            out.push_str(&format!("{}}}", pad));
            if let Some(eb) = else_b {
                out.push_str(" else {\n");
                emit_block(eb, level + 1, out)?;
                out.push_str(&format!("{}}}\n", pad));
            } else {
                out.push('\n');
            }
        }
        Stmt::While(cond, body) => {
            out.push_str(&format!("{}while ({}) {{\n", pad, emit_expr(cond)?));
            emit_block(body, level + 1, out)?;
            out.push_str(&format!("{}}}\n", pad));
        }
        Stmt::ForOf(var, iter, body) => {
            out.push_str(&format!("{}for {} in {} {{\n", pad, var, emit_expr(iter)?));
            emit_block(body, level + 1, out)?;
            out.push_str(&format!("{}}}\n", pad));
        }
        Stmt::For(init, cond, update, body) => {
            // lower to: init ; while (cond) { body ; update }
            if let Some(init) = init {
                emit_stmt(init, level, out)?;
            }
            let cond_s = match cond {
                Some(c) => emit_expr(c)?,
                None => "true".to_string(),
            };
            out.push_str(&format!("{}while ({}) {{\n", pad, cond_s));
            emit_block(body, level + 1, out)?;
            if let Some(update) = update {
                emit_stmt(update, level + 1, out)?;
            }
            out.push_str(&format!("{}}}\n", pad));
        }
        Stmt::Break => out.push_str(&format!("{}break\n", pad)),
        Stmt::Assign(target, op, value) => {
            out.push_str(&format!(
                "{}{} {} {}\n",
                pad,
                emit_expr(target)?,
                op,
                emit_expr(value)?
            ));
        }
        Stmt::Expr(e) => out.push_str(&format!("{}{}\n", pad, emit_expr(e)?)),
    }
    Ok(())
}

fn emit_num(n: f64) -> String {
    if n.fract() == 0.0 && n.abs() < 1e15 {
        format!("{}", n as i64)
    } else {
        format!("{}", n)
    }
}

fn emit_expr(e: &Expr) -> Result<String, String> {
    Ok(match e {
        Expr::Num(n) => emit_num(*n),
        Expr::Str(s) => format!("\"{}\"", escape_str(s)),
        Expr::Bool(b) => b.to_string(),
        Expr::Null => "nil".to_string(),
        Expr::Ident(n) => n.clone(),
        Expr::Tmpl(parts) => {
            let mut s = String::from("`");
            for p in parts {
                match p {
                    TmplPart::Text(t) => s.push_str(&escape_template(t)),
                    TmplPart::Expr(e) => {
                        s.push_str("${");
                        s.push_str(&emit_expr(e)?);
                        s.push('}');
                    }
                }
            }
            s.push('`');
            s
        }
        Expr::Array(elems) => {
            let parts: Result<Vec<_>, _> = elems.iter().map(emit_expr).collect();
            format!("[{}]", parts?.join(", "))
        }
        Expr::Object(fields) => {
            let mut parts = Vec::new();
            for (k, v) in fields {
                parts.push(format!("{}: {}", k, emit_expr(v)?));
            }
            format!("{{{}}}", parts.join(", "))
        }
        Expr::Unary(op, e) => {
            // fred uses `!`/`-` prefix; keep parens for safety
            format!("{}({})", op, emit_expr(e)?)
        }
        Expr::Bin(op, l, r) => format!("({} {} {})", emit_expr(l)?, op, emit_expr(r)?),
        Expr::Ternary(c, a, b) => {
            format!(
                "({} ? {} : {})",
                emit_expr(c)?,
                emit_expr(a)?,
                emit_expr(b)?
            )
        }
        Expr::Index(o, i) => format!("{}[{}]", emit_expr(o)?, emit_expr(i)?),
        Expr::Member(o, name) => {
            // bare `.length` -> fred `.len()`
            if name == "length" {
                format!("{}.len()", emit_expr(o)?)
            } else {
                format!("{}.{}", emit_expr(o)?, name)
            }
        }
        Expr::Func(params, body) => {
            // inline closure (used as a callback)
            let mut s = format!("fn({}) {{ ", params.join(", "));
            let mut inner = String::new();
            emit_block(body, 0, &mut inner)?;
            // collapse the block onto one line-ish; fred is whitespace-tolerant
            s.push_str(inner.trim());
            s.push_str(" }");
            s
        }
        Expr::Call(callee, args) => emit_call(callee, args)?,
    })
}

fn emit_call(callee: &Expr, args: &[Expr]) -> Result<String, String> {
    // console.log(...) -> print(...)
    if let Expr::Member(obj, name) = callee {
        if let Expr::Ident(o) = obj.as_ref() {
            if o == "console" && (name == "log" || name == "error" || name == "info") {
                return emit_print(args);
            }
        }
        // generic method call: obj.method(args)
        let method = map_method(name);
        let arg_s = emit_args(args)?;
        return Ok(format!("{}.{}({})", emit_expr(obj)?, method, arg_s));
    }

    if let Expr::Ident(name) = callee {
        let mapped = match name.as_str() {
            "parseInt" => Some("to_int_str"),
            "String" => Some("to_string"),
            _ => None,
        };
        let nm = mapped.unwrap_or(name.as_str());
        return Ok(format!("{}({})", nm, emit_args(args)?));
    }

    Ok(format!("{}({})", emit_expr(callee)?, emit_args(args)?))
}

fn emit_args(args: &[Expr]) -> Result<String, String> {
    let parts: Result<Vec<_>, _> = args.iter().map(emit_expr).collect();
    Ok(parts?.join(", "))
}

// console.log(a, b, c) -> print(`${a} ${b} ${c}`); single arg -> print(a)
fn emit_print(args: &[Expr]) -> Result<String, String> {
    if args.is_empty() {
        return Ok("print(\"\")".to_string());
    }
    if args.len() == 1 {
        return Ok(format!("print({})", emit_expr(&args[0])?));
    }
    let mut s = String::from("print(`");
    for (i, a) in args.iter().enumerate() {
        if i > 0 {
            s.push(' ');
        }
        s.push_str("${");
        s.push_str(&emit_expr(a)?);
        s.push('}');
    }
    s.push_str("`)");
    Ok(s)
}

fn escape_str(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            _ => out.push(c),
        }
    }
    out
}

fn escape_template(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        match c {
            '`' => out.push_str("\\`"),
            '\\' => out.push_str("\\\\"),
            _ => out.push(c),
        }
    }
    out
}

/// Transpile a JavaScript source string into `.fred` source.
pub fn transpile(js_source: &str) -> Result<String, String> {
    let toks = tokenize(js_source)?;
    let mut parser = Parser::new(toks);
    let program = parser.program()?;
    let mut out = String::new();
    emit_block(&program, 0, &mut out)?;
    Ok(out)
}
