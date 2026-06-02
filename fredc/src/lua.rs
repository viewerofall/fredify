//! Hand-written Lua -> .fred transpiler.
//!
//! Parses a practical Lua subset and emits `.fred` source (which then flows
//! through the normal lexer -> parser -> validator -> codegen pipeline). No
//! external interpreter — like the JS frontend, this is self-contained.
//!
//! Supported: local/global vars, function declarations, anonymous functions,
//! if/elseif/else, while, numeric `for i=a,b[,step]`, generic `for v in
//! ipairs(t)`, repeat/until, return, break, the operators (`..`->`+`, `~=`->
//! `!=`, `^`->Math.pow, `#`->`.len()`, and/or/not pass through), array-style
//! table constructors, method calls (`obj:m()` and `string.upper(s)`),
//! math.*/table.*/os.*, print, tostring/tonumber.
//!
//! NOT supported (errors): key/value tables (dicts), metatables, multiple
//! assignment/return, varargs, coroutines, goto, modules/require. Arrays are
//! int-only (fred limit) and indexing is passed through verbatim — remember
//! Lua is 1-based while fred/C is 0-based.

use std::collections::HashSet;

// ----------------------------- Lexer ---------------------------------------

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    // keywords
    And,
    Break,
    Do,
    Else,
    Elseif,
    End,
    False,
    For,
    Function,
    If,
    In,
    Local,
    Nil,
    Not,
    Or,
    Repeat,
    Return,
    Then,
    True,
    Until,
    While,
    // literals / names
    Name(String),
    Num(f64),
    Str(String),
    // operators / punctuation
    Plus,
    Minus,
    Star,
    Slash,
    DSlash, // //
    Percent,
    Caret, // ^
    Hash,  // #
    Eq,    // =
    EqEq,  // ==
    Ne,    // ~=
    Lt,
    Le,
    Gt,
    Ge,
    Concat, // ..
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Semi,
    Colon,
    Comma,
    Dot,
    Eof,
}

struct Lexer {
    s: Vec<char>,
    i: usize,
}

impl Lexer {
    fn new(src: &str) -> Self {
        Lexer {
            s: src.chars().collect(),
            i: 0,
        }
    }
    fn peek(&self) -> Option<char> {
        self.s.get(self.i).copied()
    }
    fn peek2(&self) -> Option<char> {
        self.s.get(self.i + 1).copied()
    }

    fn skip_trivia(&mut self) {
        loop {
            match self.peek() {
                Some(c) if c.is_whitespace() => self.i += 1,
                Some('-') if self.peek2() == Some('-') => {
                    self.i += 2;
                    // long comment --[[ ... ]]
                    if self.peek() == Some('[') && self.peek2() == Some('[') {
                        self.i += 2;
                        while self.i < self.s.len() {
                            if self.peek() == Some(']') && self.peek2() == Some(']') {
                                self.i += 2;
                                break;
                            }
                            self.i += 1;
                        }
                    } else {
                        while let Some(c) = self.peek() {
                            if c == '\n' {
                                break;
                            }
                            self.i += 1;
                        }
                    }
                }
                _ => break,
            }
        }
    }

    fn read_string(&mut self, q: char) -> Result<String, String> {
        self.i += 1;
        let mut out = String::new();
        loop {
            match self.peek() {
                None => return Err("Unterminated string".into()),
                Some(c) if c == q => {
                    self.i += 1;
                    return Ok(out);
                }
                Some('\\') => {
                    self.i += 1;
                    match self.peek() {
                        Some('n') => out.push('\n'),
                        Some('t') => out.push('\t'),
                        Some('r') => out.push('\r'),
                        Some('\\') => out.push('\\'),
                        Some('"') => out.push('"'),
                        Some('\'') => out.push('\''),
                        Some(c) => out.push(c),
                        None => return Err("Unterminated string".into()),
                    }
                    self.i += 1;
                }
                Some(c) => {
                    out.push(c);
                    self.i += 1;
                }
            }
        }
    }

    fn read_long_string(&mut self) -> Result<String, String> {
        // assumes at "[["
        self.i += 2;
        let mut out = String::new();
        // skip a leading newline (Lua convention)
        if self.peek() == Some('\n') {
            self.i += 1;
        }
        loop {
            match self.peek() {
                None => return Err("Unterminated long string".into()),
                Some(']') if self.peek2() == Some(']') => {
                    self.i += 2;
                    return Ok(out);
                }
                Some(c) => {
                    out.push(c);
                    self.i += 1;
                }
            }
        }
    }

    fn read_number(&mut self) -> f64 {
        let mut s = String::new();
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() || c == '.' {
                s.push(c);
                self.i += 1;
            } else {
                break;
            }
        }
        s.parse().unwrap_or(0.0)
    }

    fn read_name(&mut self) -> String {
        let mut s = String::new();
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' {
                s.push(c);
                self.i += 1;
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
        if c.is_alphabetic() || c == '_' {
            let w = self.read_name();
            return Ok(match w.as_str() {
                "and" => Tok::And,
                "break" => Tok::Break,
                "do" => Tok::Do,
                "else" => Tok::Else,
                "elseif" => Tok::Elseif,
                "end" => Tok::End,
                "false" => Tok::False,
                "for" => Tok::For,
                "function" => Tok::Function,
                "if" => Tok::If,
                "in" => Tok::In,
                "local" => Tok::Local,
                "nil" => Tok::Nil,
                "not" => Tok::Not,
                "or" => Tok::Or,
                "repeat" => Tok::Repeat,
                "return" => Tok::Return,
                "then" => Tok::Then,
                "true" => Tok::True,
                "until" => Tok::Until,
                "while" => Tok::While,
                _ => Tok::Name(w),
            });
        }
        if c.is_ascii_digit() {
            return Ok(Tok::Num(self.read_number()));
        }
        match c {
            '"' => return self.read_string('"').map(Tok::Str),
            '\'' => return self.read_string('\'').map(Tok::Str),
            '[' if self.peek2() == Some('[') => return self.read_long_string().map(Tok::Str),
            _ => {}
        }
        self.i += 1;
        let t = match c {
            '+' => Tok::Plus,
            '-' => Tok::Minus,
            '*' => Tok::Star,
            '/' => {
                if self.peek() == Some('/') {
                    self.i += 1;
                    Tok::DSlash
                } else {
                    Tok::Slash
                }
            }
            '%' => Tok::Percent,
            '^' => Tok::Caret,
            '#' => Tok::Hash,
            '=' => {
                if self.peek() == Some('=') {
                    self.i += 1;
                    Tok::EqEq
                } else {
                    Tok::Eq
                }
            }
            '~' => {
                if self.peek() == Some('=') {
                    self.i += 1;
                    Tok::Ne
                } else {
                    return Err("Unexpected '~'".into());
                }
            }
            '<' => {
                if self.peek() == Some('=') {
                    self.i += 1;
                    Tok::Le
                } else {
                    Tok::Lt
                }
            }
            '>' => {
                if self.peek() == Some('=') {
                    self.i += 1;
                    Tok::Ge
                } else {
                    Tok::Gt
                }
            }
            '.' => {
                if self.peek() == Some('.') {
                    self.i += 1;
                    if self.peek() == Some('.') {
                        self.i += 1;
                        return Err("varargs (...) are unsupported".into());
                    }
                    Tok::Concat
                } else {
                    Tok::Dot
                }
            }
            '(' => Tok::LParen,
            ')' => Tok::RParen,
            '{' => Tok::LBrace,
            '}' => Tok::RBrace,
            '[' => Tok::LBracket,
            ']' => Tok::RBracket,
            ';' => Tok::Semi,
            ':' => Tok::Colon,
            ',' => Tok::Comma,
            other => return Err(format!("Unexpected character: {}", other)),
        };
        Ok(t)
    }
}

fn tokenize(src: &str) -> Result<Vec<Tok>, String> {
    let mut lx = Lexer::new(src);
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
    Bool(bool),
    Nil,
    Name(String),
    Table(Vec<Expr>),
    Object(Vec<(String, Expr)>),
    Unary(String, Box<Expr>),
    Bin(String, Box<Expr>, Box<Expr>),
    Concat(Vec<Expr>),
    Index(Box<Expr>, Box<Expr>),
    Field(Box<Expr>, String),
    Call(Box<Expr>, Vec<Expr>),
    Method(Box<Expr>, String, Vec<Expr>),
    Func(Vec<String>, Vec<Stmt>),
}

enum Stmt {
    Local(String, Option<Expr>),
    // function name(params) ... end  /  local function ...
    Func(String, Vec<String>, Vec<Stmt>),
    Assign(Expr, Expr),
    Call(Expr),
    // if cond then .. (elseif cond then ..)* (else ..)? end
    If(Vec<(Expr, Vec<Stmt>)>, Option<Vec<Stmt>>),
    While(Expr, Vec<Stmt>),
    NumFor(String, Expr, Expr, Option<Expr>, Vec<Stmt>),
    GenFor(String, Expr, Vec<Stmt>),
    Repeat(Vec<Stmt>, Expr),
    Return(Option<Expr>),
    Break,
    Do(Vec<Stmt>),
}

// ------------------------------ Parser -------------------------------------

struct Parser {
    t: Vec<Tok>,
    i: usize,
}

impl Parser {
    fn new(t: Vec<Tok>) -> Self {
        Parser { t, i: 0 }
    }
    fn peek(&self) -> &Tok {
        self.t.get(self.i).unwrap_or(&Tok::Eof)
    }
    fn peek2(&self) -> &Tok {
        self.t.get(self.i + 1).unwrap_or(&Tok::Eof)
    }
    fn bump(&mut self) -> Tok {
        let t = self.peek().clone();
        self.i += 1;
        t
    }
    fn eat(&mut self, t: &Tok) -> Result<(), String> {
        if std::mem::discriminant(self.peek()) == std::mem::discriminant(t) {
            self.i += 1;
            Ok(())
        } else {
            Err(format!("Expected {:?}, got {:?}", t, self.peek()))
        }
    }
    fn name(&mut self) -> Result<String, String> {
        match self.bump() {
            Tok::Name(n) => Ok(n),
            other => Err(format!("Expected name, got {:?}", other)),
        }
    }

    fn is_block_end(&self) -> bool {
        matches!(
            self.peek(),
            Tok::End | Tok::Else | Tok::Elseif | Tok::Until | Tok::Eof
        )
    }

    fn program(&mut self) -> Result<Vec<Stmt>, String> {
        let mut out = Vec::new();
        while !self.is_block_end() {
            if self.peek() == &Tok::Semi {
                self.i += 1;
                continue;
            }
            out.push(self.stmt()?);
        }
        Ok(out)
    }

    fn block(&mut self) -> Result<Vec<Stmt>, String> {
        self.program()
    }

    fn stmt(&mut self) -> Result<Stmt, String> {
        match self.peek() {
            Tok::Local => self.local_stmt(),
            Tok::Function => self.func_stmt(),
            Tok::If => self.if_stmt(),
            Tok::While => {
                self.bump();
                let cond = self.expr()?;
                self.eat(&Tok::Do)?;
                let body = self.block()?;
                self.eat(&Tok::End)?;
                Ok(Stmt::While(cond, body))
            }
            Tok::For => self.for_stmt(),
            Tok::Repeat => {
                self.bump();
                let body = self.block()?;
                self.eat(&Tok::Until)?;
                let cond = self.expr()?;
                Ok(Stmt::Repeat(body, cond))
            }
            Tok::Return => {
                self.bump();
                if self.is_block_end() || self.peek() == &Tok::Semi {
                    Ok(Stmt::Return(None))
                } else {
                    let e = self.expr()?;
                    if self.peek() == &Tok::Comma {
                        return Err("multiple return values are unsupported".into());
                    }
                    Ok(Stmt::Return(Some(e)))
                }
            }
            Tok::Break => {
                self.bump();
                Ok(Stmt::Break)
            }
            Tok::Do => {
                self.bump();
                let body = self.block()?;
                self.eat(&Tok::End)?;
                Ok(Stmt::Do(body))
            }
            _ => self.expr_stmt(),
        }
    }

    fn local_stmt(&mut self) -> Result<Stmt, String> {
        self.bump(); // local
        if self.peek() == &Tok::Function {
            self.bump();
            let n = self.name()?;
            let (params, body) = self.func_body()?;
            return Ok(Stmt::Func(n, params, body));
        }
        let n = self.name()?;
        if self.peek() == &Tok::Comma {
            return Err("multiple assignment (local a, b = ...) is unsupported; split it".into());
        }
        let val = if self.peek() == &Tok::Eq {
            self.bump();
            Some(self.expr()?)
        } else {
            None
        };
        // `local f = function() ... end` is folded into a fred fn def on emit.
        Ok(Stmt::Local(n, val))
    }

    fn func_stmt(&mut self) -> Result<Stmt, String> {
        self.bump(); // function
        let n = self.name()?;
        if matches!(self.peek(), Tok::Dot | Tok::Colon) {
            return Err("table/method function definitions (a.b / a:b) are unsupported".into());
        }
        let (params, body) = self.func_body()?;
        Ok(Stmt::Func(n, params, body))
    }

    // parse `(params) <block> end`
    fn func_body(&mut self) -> Result<(Vec<String>, Vec<Stmt>), String> {
        self.eat(&Tok::LParen)?;
        let mut params = Vec::new();
        while self.peek() != &Tok::RParen {
            params.push(self.name()?);
            if self.peek() == &Tok::Comma {
                self.bump();
            }
        }
        self.eat(&Tok::RParen)?;
        let body = self.block()?;
        self.eat(&Tok::End)?;
        Ok((params, body))
    }

    fn if_stmt(&mut self) -> Result<Stmt, String> {
        self.bump(); // if
        let mut arms = Vec::new();
        let cond = self.expr()?;
        self.eat(&Tok::Then)?;
        let body = self.block()?;
        arms.push((cond, body));
        let mut else_body = None;
        loop {
            match self.peek() {
                Tok::Elseif => {
                    self.bump();
                    let c = self.expr()?;
                    self.eat(&Tok::Then)?;
                    let b = self.block()?;
                    arms.push((c, b));
                }
                Tok::Else => {
                    self.bump();
                    else_body = Some(self.block()?);
                    break;
                }
                _ => break,
            }
        }
        self.eat(&Tok::End)?;
        Ok(Stmt::If(arms, else_body))
    }

    fn for_stmt(&mut self) -> Result<Stmt, String> {
        self.bump(); // for
        let first = self.name()?;
        match self.peek() {
            Tok::Eq => {
                // numeric for: i = a, b [, step]
                self.bump();
                let start = self.expr()?;
                self.eat(&Tok::Comma)?;
                let end = self.expr()?;
                let step = if self.peek() == &Tok::Comma {
                    self.bump();
                    Some(self.expr()?)
                } else {
                    None
                };
                self.eat(&Tok::Do)?;
                let body = self.block()?;
                self.eat(&Tok::End)?;
                Ok(Stmt::NumFor(first, start, end, step, body))
            }
            Tok::Comma | Tok::In => {
                // generic for: namelist in explist do
                let mut last = first;
                while self.peek() == &Tok::Comma {
                    self.bump();
                    last = self.name()?; // keep the last name (the "value" var)
                }
                self.eat(&Tok::In)?;
                let iter = self.expr()?;
                // unwrap ipairs(t) / pairs(t) -> t
                let iter = unwrap_iter(iter);
                self.eat(&Tok::Do)?;
                let body = self.block()?;
                self.eat(&Tok::End)?;
                Ok(Stmt::GenFor(last, iter, body))
            }
            other => Err(format!("Expected '=' or 'in' in for, got {:?}", other)),
        }
    }

    fn expr_stmt(&mut self) -> Result<Stmt, String> {
        let e = self.suffixed_expr()?;
        if self.peek() == &Tok::Eq {
            self.bump();
            let rhs = self.expr()?;
            if self.peek() == &Tok::Comma {
                return Err("multiple assignment (a, b = ...) is unsupported; split it".into());
            }
            match &e {
                Expr::Name(_) | Expr::Index(_, _) | Expr::Field(_, _) => {}
                _ => return Err("invalid assignment target".into()),
            }
            return Ok(Stmt::Assign(e, rhs));
        }
        match e {
            Expr::Call(_, _) | Expr::Method(_, _, _) => Ok(Stmt::Call(e)),
            _ => Err("syntax error: expression statement must be a call or assignment".into()),
        }
    }

    // ---- expressions (precedence climbing) ----

    fn expr(&mut self) -> Result<Expr, String> {
        self.or_expr()
    }

    fn or_expr(&mut self) -> Result<Expr, String> {
        let mut l = self.and_expr()?;
        while self.peek() == &Tok::Or {
            self.bump();
            let r = self.and_expr()?;
            l = Expr::Bin("or".into(), Box::new(l), Box::new(r));
        }
        Ok(l)
    }

    fn and_expr(&mut self) -> Result<Expr, String> {
        let mut l = self.cmp_expr()?;
        while self.peek() == &Tok::And {
            self.bump();
            let r = self.cmp_expr()?;
            l = Expr::Bin("and".into(), Box::new(l), Box::new(r));
        }
        Ok(l)
    }

    fn cmp_expr(&mut self) -> Result<Expr, String> {
        let mut l = self.concat_expr()?;
        loop {
            let op = match self.peek() {
                Tok::EqEq => "==",
                Tok::Ne => "!=",
                Tok::Lt => "<",
                Tok::Le => "<=",
                Tok::Gt => ">",
                Tok::Ge => ">=",
                _ => break,
            };
            self.bump();
            let r = self.concat_expr()?;
            l = Expr::Bin(op.into(), Box::new(l), Box::new(r));
        }
        Ok(l)
    }

    // Lua `..` stringifies its operands. fred's `+` does not coerce, so we
    // gather the whole chain and emit a template string (`${}` auto-converts).
    fn concat_expr(&mut self) -> Result<Expr, String> {
        let first = self.add_expr()?;
        if self.peek() != &Tok::Concat {
            return Ok(first);
        }
        let mut parts = vec![first];
        while self.peek() == &Tok::Concat {
            self.bump();
            parts.push(self.add_expr()?);
        }
        Ok(Expr::Concat(parts))
    }

    fn add_expr(&mut self) -> Result<Expr, String> {
        let mut l = self.mul_expr()?;
        loop {
            let op = match self.peek() {
                Tok::Plus => "+",
                Tok::Minus => "-",
                _ => break,
            };
            self.bump();
            let r = self.mul_expr()?;
            l = Expr::Bin(op.into(), Box::new(l), Box::new(r));
        }
        Ok(l)
    }

    fn mul_expr(&mut self) -> Result<Expr, String> {
        let mut l = self.unary_expr()?;
        loop {
            let op = match self.peek() {
                Tok::Star => "*",
                Tok::Slash => "/",
                Tok::DSlash => "/", // floor div ~ fred int div
                Tok::Percent => "%",
                _ => break,
            };
            self.bump();
            let r = self.unary_expr()?;
            l = Expr::Bin(op.into(), Box::new(l), Box::new(r));
        }
        Ok(l)
    }

    fn unary_expr(&mut self) -> Result<Expr, String> {
        match self.peek() {
            Tok::Not => {
                self.bump();
                Ok(Expr::Unary("not".into(), Box::new(self.unary_expr()?)))
            }
            Tok::Minus => {
                self.bump();
                Ok(Expr::Unary("-".into(), Box::new(self.unary_expr()?)))
            }
            Tok::Hash => {
                self.bump();
                Ok(Expr::Unary("#".into(), Box::new(self.unary_expr()?)))
            }
            _ => self.pow_expr(),
        }
    }

    // `^` binds tighter than unary in Lua, but for our int-only world the
    // simpler precedence is fine; map to Math.pow.
    fn pow_expr(&mut self) -> Result<Expr, String> {
        let l = self.suffixed_expr()?;
        if self.peek() == &Tok::Caret {
            self.bump();
            let r = self.unary_expr()?;
            return Ok(Expr::Bin("^".into(), Box::new(l), Box::new(r)));
        }
        Ok(l)
    }

    fn suffixed_expr(&mut self) -> Result<Expr, String> {
        let mut e = self.primary()?;
        loop {
            match self.peek() {
                Tok::Dot => {
                    self.bump();
                    let n = self.name()?;
                    e = Expr::Field(Box::new(e), n);
                }
                Tok::LBracket => {
                    self.bump();
                    let idx = self.expr()?;
                    self.eat(&Tok::RBracket)?;
                    e = Expr::Index(Box::new(e), Box::new(idx));
                }
                Tok::Colon => {
                    self.bump();
                    let m = self.name()?;
                    let args = self.call_args()?;
                    e = Expr::Method(Box::new(e), m, args);
                }
                Tok::LParen => {
                    let args = self.call_args()?;
                    e = Expr::Call(Box::new(e), args);
                }
                // string call sugar:  f"x"  — rare, support f("x")-style only
                _ => break,
            }
        }
        Ok(e)
    }

    fn call_args(&mut self) -> Result<Vec<Expr>, String> {
        // single string argument: print "hi"
        if let Tok::Str(s) = self.peek() {
            let s = s.clone();
            self.bump();
            return Ok(vec![Expr::Str(s)]);
        }
        // table argument: f{...}
        if self.peek() == &Tok::LBrace {
            return Ok(vec![self.primary()?]);
        }
        self.eat(&Tok::LParen)?;
        let mut args = Vec::new();
        while self.peek() != &Tok::RParen {
            args.push(self.expr()?);
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
            Tok::Nil => {
                self.bump();
                Ok(Expr::Nil)
            }
            Tok::Name(n) => {
                self.bump();
                Ok(Expr::Name(n))
            }
            Tok::Function => {
                self.bump();
                let (params, body) = self.func_body()?;
                Ok(Expr::Func(params, body))
            }
            Tok::LParen => {
                self.bump();
                let e = self.expr()?;
                self.eat(&Tok::RParen)?;
                Ok(e)
            }
            Tok::LBrace => self.table(),
            other => Err(format!("Unexpected token in expression: {:?}", other)),
        }
    }

    fn table(&mut self) -> Result<Expr, String> {
        self.eat(&Tok::LBrace)?;

        // Decide array vs dict by looking at the first entry: `name =` or `[k] =`
        // means a key/value table (dict); otherwise it's an array.
        let is_dict = matches!(self.peek(), Tok::LBracket)
            || (matches!(self.peek(), Tok::Name(_)) && self.peek2() == &Tok::Eq);

        if is_dict {
            let mut fields = Vec::new();
            while self.peek() != &Tok::RBrace {
                let key = match self.peek().clone() {
                    Tok::Name(k) => {
                        self.bump();
                        k
                    }
                    Tok::LBracket => {
                        self.bump();
                        let k = match self.peek().clone() {
                            Tok::Str(s) => {
                                self.bump();
                                s
                            }
                            Tok::Num(n) => {
                                self.bump();
                                emit_num(n)
                            }
                            other => {
                                return Err(format!(
                                    "table key must be a string/number literal, got {:?}",
                                    other
                                ))
                            }
                        };
                        self.eat(&Tok::RBracket)?;
                        k
                    }
                    other => {
                        return Err(format!(
                            "mixed array/dict tables are unsupported; got {:?}",
                            other
                        ))
                    }
                };
                self.eat(&Tok::Eq)?;
                let val = self.expr()?;
                fields.push((key, val));
                if self.peek() == &Tok::Comma || self.peek() == &Tok::Semi {
                    self.bump();
                }
            }
            self.eat(&Tok::RBrace)?;
            return Ok(Expr::Object(fields));
        }

        let mut elems = Vec::new();
        while self.peek() != &Tok::RBrace {
            // a key/value entry appearing after array entries = mixed table (unsupported)
            if self.peek() == &Tok::LBracket
                || (matches!(self.peek(), Tok::Name(_)) && self.peek2() == &Tok::Eq)
            {
                return Err("mixed array/dict tables are unsupported".into());
            }
            elems.push(self.expr()?);
            if self.peek() == &Tok::Comma || self.peek() == &Tok::Semi {
                self.bump();
            }
        }
        self.eat(&Tok::RBrace)?;
        Ok(Expr::Table(elems))
    }
}

// ipairs(t) / pairs(t) -> t  (so generic-for iterates the array directly)
fn unwrap_iter(e: Expr) -> Expr {
    if let Expr::Call(callee, mut args) = e {
        if let Expr::Name(n) = callee.as_ref() {
            if (n == "ipairs" || n == "pairs") && args.len() == 1 {
                return args.remove(0);
            }
        }
        return Expr::Call(callee, args);
    }
    e
}

// ------------------------------ Emitter ------------------------------------

fn indent(level: usize) -> String {
    "    ".repeat(level)
}

fn map_math(name: &str) -> String {
    // Lua math.* -> fred Math.*  (names line up; capitalize the module)
    format!("Math.{}", name)
}

fn map_method(name: &str) -> &str {
    match name {
        "upper" => "uppercase",
        "lower" => "lowercase",
        "sub" => "substring",
        "len" => "length",
        other => other,
    }
}

fn emit_block(
    stmts: &[Stmt],
    level: usize,
    decl: &mut HashSet<String>,
    out: &mut String,
) -> Result<(), String> {
    for s in stmts {
        emit_stmt(s, level, decl, out)?;
    }
    Ok(())
}

fn emit_stmt(
    s: &Stmt,
    level: usize,
    decl: &mut HashSet<String>,
    out: &mut String,
) -> Result<(), String> {
    let pad = indent(level);
    match s {
        Stmt::Local(name, value) => {
            decl.insert(name.clone());
            match value {
                Some(Expr::Func(params, body)) => {
                    out.push_str(&format!("{}fn {}({}) {{\n", pad, name, params.join(", ")));
                    emit_block(body, level + 1, decl, out)?;
                    out.push_str(&format!("{}}}\n", pad));
                }
                Some(v) => out.push_str(&format!("{}let {} = {}\n", pad, name, emit_expr(v)?)),
                None => out.push_str(&format!("{}let {}\n", pad, name)),
            }
        }
        Stmt::Func(name, params, body) => {
            decl.insert(name.clone());
            out.push_str(&format!("{}fn {}({}) {{\n", pad, name, params.join(", ")));
            emit_block(body, level + 1, decl, out)?;
            out.push_str(&format!("{}}}\n", pad));
        }
        Stmt::Assign(target, value) => {
            // a bare global `x = ...` becomes `let x = ...` on first sight
            if let Expr::Name(n) = target {
                if !decl.contains(n) {
                    decl.insert(n.clone());
                    out.push_str(&format!("{}let {} = {}\n", pad, n, emit_expr(value)?));
                    return Ok(());
                }
            }
            out.push_str(&format!(
                "{}{} = {}\n",
                pad,
                emit_expr(target)?,
                emit_expr(value)?
            ));
        }
        Stmt::Call(e) => out.push_str(&format!("{}{}\n", pad, emit_expr(e)?)),
        Stmt::If(arms, else_body) => emit_if(arms, else_body, level, decl, out)?,
        Stmt::While(cond, body) => {
            out.push_str(&format!("{}while ({}) {{\n", pad, emit_expr(cond)?));
            emit_block(body, level + 1, decl, out)?;
            out.push_str(&format!("{}}}\n", pad));
        }
        Stmt::NumFor(var, start, end, step, body) => {
            decl.insert(var.clone());
            let step_s = match step {
                Some(s) => format!(", {}", emit_expr(s)?),
                None => String::new(),
            };
            out.push_str(&format!(
                "{}loop {} from {} to {}{} {{\n",
                pad,
                var,
                emit_expr(start)?,
                emit_expr(end)?,
                step_s
            ));
            emit_block(body, level + 1, decl, out)?;
            out.push_str(&format!("{}}}\n", pad));
        }
        Stmt::GenFor(var, iter, body) => {
            decl.insert(var.clone());
            out.push_str(&format!("{}for {} in {} {{\n", pad, var, emit_expr(iter)?));
            emit_block(body, level + 1, decl, out)?;
            out.push_str(&format!("{}}}\n", pad));
        }
        Stmt::Repeat(body, cond) => {
            // repeat B until C  ->  while (true) { B if (C) { break } }
            out.push_str(&format!("{}while (true) {{\n", pad));
            emit_block(body, level + 1, decl, out)?;
            out.push_str(&format!(
                "{}if ({}) {{ break }}\n",
                indent(level + 1),
                emit_expr(cond)?
            ));
            out.push_str(&format!("{}}}\n", pad));
        }
        Stmt::Return(v) => match v {
            Some(e) => out.push_str(&format!("{}return {}\n", pad, emit_expr(e)?)),
            None => out.push_str(&format!("{}return\n", pad)),
        },
        Stmt::Break => out.push_str(&format!("{}break\n", pad)),
        Stmt::Do(body) => {
            // fred has no bare block scope; just inline it
            emit_block(body, level, decl, out)?;
        }
    }
    Ok(())
}

fn emit_if(
    arms: &[(Expr, Vec<Stmt>)],
    else_body: &Option<Vec<Stmt>>,
    level: usize,
    decl: &mut HashSet<String>,
    out: &mut String,
) -> Result<(), String> {
    let pad = indent(level);
    // first arm
    out.push_str(&format!("{}if ({}) {{\n", pad, emit_expr(&arms[0].0)?));
    emit_block(&arms[0].1, level + 1, decl, out)?;
    out.push_str(&format!("{}}}", pad));
    // remaining arms become nested else { if ... }
    emit_if_rest(&arms[1..], else_body, level, decl, out)?;
    out.push('\n');
    Ok(())
}

fn emit_if_rest(
    arms: &[(Expr, Vec<Stmt>)],
    else_body: &Option<Vec<Stmt>>,
    level: usize,
    decl: &mut HashSet<String>,
    out: &mut String,
) -> Result<(), String> {
    let pad = indent(level);
    if arms.is_empty() {
        if let Some(eb) = else_body {
            out.push_str(" else {\n");
            emit_block(eb, level + 1, decl, out)?;
            out.push_str(&format!("{}}}", pad));
        }
        return Ok(());
    }
    out.push_str(" else {\n");
    let ipad = indent(level + 1);
    out.push_str(&format!("{}if ({}) {{\n", ipad, emit_expr(&arms[0].0)?));
    emit_block(&arms[0].1, level + 2, decl, out)?;
    out.push_str(&format!("{}}}", ipad));
    emit_if_rest(&arms[1..], else_body, level + 1, decl, out)?;
    out.push('\n');
    out.push_str(&format!("{}}}", pad));
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
        Expr::Nil => "nil".to_string(),
        Expr::Name(n) => n.clone(),
        Expr::Table(elems) => {
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
        Expr::Unary(op, e) => match op.as_str() {
            "#" => format!("{}.len()", emit_expr(e)?),
            "not" => format!("not ({})", emit_expr(e)?),
            _ => format!("{}({})", op, emit_expr(e)?),
        },
        Expr::Bin(op, l, r) => match op.as_str() {
            "^" => format!("Math.pow({}, {})", emit_expr(l)?, emit_expr(r)?),
            _ => format!("({} {} {})", emit_expr(l)?, op, emit_expr(r)?),
        },
        Expr::Concat(parts) => {
            let mut s = String::from("`");
            for p in parts {
                s.push_str("${");
                s.push_str(&emit_expr(p)?);
                s.push('}');
            }
            s.push('`');
            s
        }
        Expr::Index(o, i) => format!("{}[{}]", emit_expr(o)?, emit_expr(i)?),
        Expr::Field(o, name) => format!("{}.{}", emit_expr(o)?, name),
        Expr::Method(o, m, args) => {
            format!("{}.{}({})", emit_expr(o)?, map_method(m), emit_args(args)?)
        }
        Expr::Func(params, body) => {
            let mut decl = HashSet::new();
            let mut inner = String::new();
            emit_block(body, 0, &mut decl, &mut inner)?;
            format!("fn({}) {{ {} }}", params.join(", "), inner.trim())
        }
        Expr::Call(callee, args) => emit_call(callee, args)?,
    })
}

fn emit_call(callee: &Expr, args: &[Expr]) -> Result<String, String> {
    if let Expr::Field(obj, name) = callee {
        if let Expr::Name(o) = obj.as_ref() {
            match o.as_str() {
                "math" => return Ok(format!("{}({})", map_math(name), emit_args(args)?)),
                "table" | "os" => return Ok(format!("{}.{}({})", o, name, emit_args(args)?)),
                "string" => return emit_string_lib(name, args),
                _ => {}
            }
        }
        // value.method(args) called with dot -> fred method call
        return Ok(format!(
            "{}.{}({})",
            emit_expr(obj)?,
            map_method(name),
            emit_args(args)?
        ));
    }

    if let Expr::Name(name) = callee {
        match name.as_str() {
            "print" => return emit_print(args),
            "tostring" => return Ok(format!("to_string({})", emit_args(args)?)),
            "tonumber" => return Ok(format!("to_int_str({})", emit_args(args)?)),
            _ => return Ok(format!("{}({})", name, emit_args(args)?)),
        }
    }

    Ok(format!("{}({})", emit_expr(callee)?, emit_args(args)?))
}

// string.upper(s) -> s.uppercase(); string.find/split stay as fred string.*
fn emit_string_lib(name: &str, args: &[Expr]) -> Result<String, String> {
    match name {
        "upper" | "lower" | "len" if args.len() == 1 => {
            Ok(format!("{}.{}()", emit_expr(&args[0])?, map_method(name)))
        }
        "sub" if args.len() >= 2 => {
            let rest: Result<Vec<_>, _> = args[1..].iter().map(emit_expr).collect();
            Ok(format!(
                "{}.substring({})",
                emit_expr(&args[0])?,
                rest?.join(", ")
            ))
        }
        "find" | "split" => Ok(format!("string.{}({})", name, emit_args(args)?)),
        other => Err(format!(
            "string.{} is unsupported (try s:upper()/s:sub() or string.find/split)",
            other
        )),
    }
}

fn emit_args(args: &[Expr]) -> Result<String, String> {
    let parts: Result<Vec<_>, _> = args.iter().map(emit_expr).collect();
    Ok(parts?.join(", "))
}

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

/// Transpile a Lua source string into `.fred` source.
pub fn transpile(lua_source: &str) -> Result<String, String> {
    let toks = tokenize(lua_source)?;
    let mut parser = Parser::new(toks);
    let program = parser.program()?;
    if parser.peek() != &Tok::Eof {
        return Err(format!("Unexpected trailing token: {:?}", parser.peek()));
    }
    let mut decl = HashSet::new();
    let mut out = String::new();
    emit_block(&program, 0, &mut decl, &mut out)?;
    Ok(out)
}
