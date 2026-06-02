use crate::ast::{Expr, Stmt};
use crate::lexer::Token;

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, pos: 0 }
    }

    fn peek(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or(&Token::Eof)
    }

    fn advance(&mut self) {
        self.pos += 1;
    }

    fn expect(&mut self, expected: Token) -> Result<(), String> {
        if std::mem::discriminant(self.peek()) == std::mem::discriminant(&expected) {
            self.advance();
            Ok(())
        } else {
            Err(format!("Expected {:?}, got {:?}", expected, self.peek()))
        }
    }

    fn parse_program(&mut self) -> Result<Vec<Stmt>, String> {
        let mut stmts = Vec::new();
        while self.peek() != &Token::Eof {
            stmts.push(self.parse_stmt()?);
        }
        Ok(stmts)
    }

    fn parse_stmt(&mut self) -> Result<Stmt, String> {
        match self.peek() {
            Token::Fn => self.parse_fn_def(),
            Token::Let => self.parse_let(),
            Token::If => self.parse_if(),
            Token::While => self.parse_while(),
            Token::Loop => self.parse_loop(),
            Token::For => self.parse_for_in(),
            Token::Return => self.parse_return(),
            Token::Break => {
                self.advance();
                Ok(Stmt::Break)
            }
            Token::Switch => self.parse_switch(),
            _ => {
                let expr = self.parse_expr()?;
                // Compound assignment operators desugar to `target = target <op> value`
                let compound_op = match self.peek() {
                    Token::PlusEqual => Some("+"),
                    Token::MinusEqual => Some("-"),
                    Token::StarEqual => Some("*"),
                    Token::SlashEqual => Some("/"),
                    _ => None,
                };
                if self.peek() == &Token::Equal || compound_op.is_some() {
                    self.advance();
                    let rhs = self.parse_expr()?;
                    // Build the value: plain rhs, or (target <op> rhs) for compound forms
                    let value = match compound_op {
                        Some(op) => Expr::BinOp {
                            left: Box::new(expr.clone()),
                            op: op.to_string(),
                            right: Box::new(rhs),
                        },
                        None => rhs,
                    };
                    match expr {
                        Expr::Id(name) => Ok(Stmt::Assign { target: name, value }),
                        Expr::Index { obj, index } => Ok(Stmt::AssignIndex {
                            obj: *obj,
                            index: *index,
                            value,
                        }),
                        Expr::Field { obj, field } => Ok(Stmt::AssignField {
                            obj: *obj,
                            field,
                            value,
                        }),
                        _ => Err("Can only assign to identifiers, array elements, or object fields".to_string()),
                    }
                } else {
                    Ok(Stmt::Expr(expr))
                }
            }
        }
    }

    fn parse_fn_def(&mut self) -> Result<Stmt, String> {
        self.expect(Token::Fn)?;
        let name = match self.peek() {
            Token::Id(n) => {
                let name = n.clone();
                self.advance();
                name
            }
            _ => return Err("Expected function name".to_string()),
        };

        self.expect(Token::LParen)?;
        let mut params = Vec::new();
        while self.peek() != &Token::RParen {
            match self.peek() {
                Token::Id(p) => {
                    params.push(p.clone());
                    self.advance();
                }
                _ => return Err("Expected parameter name".to_string()),
            }
            if self.peek() == &Token::Comma {
                self.advance();
            }
        }
        self.expect(Token::RParen)?;

        self.expect(Token::LBrace)?;
        let body = self.parse_block()?;
        self.expect(Token::RBrace)?;

        Ok(Stmt::FnDef { name, params, body })
    }

    fn parse_let(&mut self) -> Result<Stmt, String> {
        self.expect(Token::Let)?;
        let name = match self.peek() {
            Token::Id(n) => {
                let name = n.clone();
                self.advance();
                name
            }
            _ => return Err("Expected variable name".to_string()),
        };

        let value = if self.peek() == &Token::Equal {
            self.advance();
            Some(self.parse_expr()?)
        } else {
            None
        };

        Ok(Stmt::Let { name, value })
    }

    fn parse_if(&mut self) -> Result<Stmt, String> {
        self.expect(Token::If)?;
        self.expect(Token::LParen)?;
        let cond = self.parse_expr()?;
        self.expect(Token::RParen)?;

        self.expect(Token::LBrace)?;
        let then_body = self.parse_block()?;
        self.expect(Token::RBrace)?;

        let else_body = if self.peek() == &Token::Else {
            self.advance();
            self.expect(Token::LBrace)?;
            let body = self.parse_block()?;
            self.expect(Token::RBrace)?;
            Some(body)
        } else {
            None
        };

        Ok(Stmt::If {
            cond,
            then_body,
            else_body,
        })
    }

    fn parse_while(&mut self) -> Result<Stmt, String> {
        self.expect(Token::While)?;
        self.expect(Token::LParen)?;
        let cond = self.parse_expr()?;
        self.expect(Token::RParen)?;

        self.expect(Token::LBrace)?;
        let body = self.parse_block()?;
        self.expect(Token::RBrace)?;

        Ok(Stmt::While { cond, body })
    }

    fn parse_loop(&mut self) -> Result<Stmt, String> {
        self.expect(Token::Loop)?;
        let var = match self.peek() {
            Token::Id(v) => {
                let var = v.clone();
                self.advance();
                var
            }
            _ => return Err("Expected loop variable".to_string()),
        };

        self.expect(Token::From)?;
        let from = self.parse_expr()?;

        self.expect(Token::To)?;
        let to = self.parse_expr()?;

        let step = if self.peek() == &Token::Comma {
            self.advance();
            Some(self.parse_expr()?)
        } else {
            None
        };

        self.expect(Token::LBrace)?;
        let body = self.parse_block()?;
        self.expect(Token::RBrace)?;

        Ok(Stmt::Loop {
            var,
            from,
            to,
            step,
            body,
        })
    }

    fn parse_return(&mut self) -> Result<Stmt, String> {
        self.expect(Token::Return)?;
        let value = if self.peek() == &Token::Semicolon || self.peek() == &Token::RBrace {
            None
        } else {
            Some(self.parse_expr()?)
        };
        Ok(Stmt::Return(value))
    }

    fn parse_for_in(&mut self) -> Result<Stmt, String> {
        self.expect(Token::For)?;
        let var = match self.peek() {
            Token::Id(v) => {
                let var = v.clone();
                self.advance();
                var
            }
            _ => return Err("Expected loop variable".to_string()),
        };

        self.expect(Token::In)?;
        let iter = self.parse_expr()?;

        self.expect(Token::LBrace)?;
        let body = self.parse_block()?;
        self.expect(Token::RBrace)?;

        Ok(Stmt::ForIn { var, iter, body })
    }

    fn parse_switch(&mut self) -> Result<Stmt, String> {
        self.expect(Token::Switch)?;
        self.expect(Token::LParen)?;
        let expr = self.parse_expr()?;
        self.expect(Token::RParen)?;
        self.expect(Token::LBrace)?;

        let mut cases = Vec::new();
        while self.peek() != &Token::RBrace && self.peek() != &Token::Eof {
            match self.peek() {
                Token::Case => {
                    self.advance();
                    let case_expr = self.parse_expr()?;
                    self.expect(Token::Colon)?;
                    let body = self.parse_case_body()?;
                    cases.push((Some(case_expr), body));
                }
                Token::Default => {
                    self.advance();
                    self.expect(Token::Colon)?;
                    let body = self.parse_case_body()?;
                    cases.push((None, body));
                }
                _ => return Err("Expected case or default".to_string()),
            }
        }
        self.expect(Token::RBrace)?;
        Ok(Stmt::Switch { expr, cases })
    }

    fn parse_case_body(&mut self) -> Result<Vec<Stmt>, String> {
        let mut stmts = Vec::new();
        while !matches!(self.peek(), Token::Case | Token::Default | Token::RBrace | Token::Eof) {
            stmts.push(self.parse_stmt()?);
        }
        Ok(stmts)
    }

    fn parse_block(&mut self) -> Result<Vec<Stmt>, String> {
        let mut stmts = Vec::new();
        while self.peek() != &Token::RBrace && self.peek() != &Token::Eof {
            stmts.push(self.parse_stmt()?);
        }
        Ok(stmts)
    }

    fn parse_expr(&mut self) -> Result<Expr, String> {
        self.parse_ternary()
    }

    fn parse_ternary(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_or()?;
        if self.peek() == &Token::Question {
            self.advance();
            let then_expr = self.parse_or()?;
            self.expect(Token::Colon)?;
            let else_expr = self.parse_ternary()?;
            expr = Expr::Ternary {
                cond: Box::new(expr),
                then_expr: Box::new(then_expr),
                else_expr: Box::new(else_expr),
            };
        }
        Ok(expr)
    }

    fn parse_or(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_and()?;
        while self.peek() == &Token::Or {
            self.advance();
            let right = self.parse_and()?;
            left = Expr::BinOp {
                left: Box::new(left),
                op: "or".to_string(),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_comparison()?;
        while self.peek() == &Token::And {
            self.advance();
            let right = self.parse_comparison()?;
            left = Expr::BinOp {
                left: Box::new(left),
                op: "and".to_string(),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_add_sub()?;
        while let Some(op) = self.get_comparison_op() {
            self.advance();
            let right = self.parse_add_sub()?;
            left = Expr::BinOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn get_comparison_op(&self) -> Option<String> {
        match self.peek() {
            Token::EqualEqual => Some("==".to_string()),
            Token::NotEqual => Some("!=".to_string()),
            Token::Less => Some("<".to_string()),
            Token::LessEqual => Some("<=".to_string()),
            Token::Greater => Some(">".to_string()),
            Token::GreaterEqual => Some(">=".to_string()),
            _ => None,
        }
    }

    fn parse_add_sub(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_mul_div()?;
        while let Some(op) = self.get_add_sub_op() {
            self.advance();
            let right = self.parse_mul_div()?;
            left = Expr::BinOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn get_add_sub_op(&self) -> Option<String> {
        match self.peek() {
            Token::Plus => Some("+".to_string()),
            Token::Minus => Some("-".to_string()),
            _ => None,
        }
    }

    fn parse_mul_div(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_unary()?;
        while let Some(op) = self.get_mul_div_op() {
            self.advance();
            let right = self.parse_unary()?;
            left = Expr::BinOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn get_mul_div_op(&self) -> Option<String> {
        match self.peek() {
            Token::Star => Some("*".to_string()),
            Token::Slash => Some("/".to_string()),
            Token::Percent => Some("%".to_string()),
            _ => None,
        }
    }

    fn parse_unary(&mut self) -> Result<Expr, String> {
        match self.peek() {
            Token::Not => {
                self.advance();
                Ok(Expr::UnOp {
                    op: "!".to_string(),
                    expr: Box::new(self.parse_unary()?),
                })
            }
            Token::Minus => {
                self.advance();
                Ok(Expr::UnOp {
                    op: "-".to_string(),
                    expr: Box::new(self.parse_unary()?),
                })
            }
            _ => self.parse_call(),
        }
    }

    fn parse_call(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_primary()?;

        loop {
            match self.peek() {
                Token::LParen => {
                    self.advance();
                    let mut args = Vec::new();
                    while self.peek() != &Token::RParen {
                        args.push(self.parse_expr()?);
                        if self.peek() == &Token::Comma {
                            self.advance();
                        }
                    }
                    self.expect(Token::RParen)?;
                    expr = Expr::Call {
                        func: Box::new(expr),
                        args,
                    };
                }
                Token::Dot => {
                    self.advance();
                    match self.peek() {
                        Token::Id(method) => {
                            let method = method.clone();
                            self.advance();
                            // Check if this is a method call or just field access
                            if self.peek() == &Token::LParen {
                                self.advance();
                                let mut args = Vec::new();
                                while self.peek() != &Token::RParen {
                                    args.push(self.parse_expr()?);
                                    if self.peek() == &Token::Comma {
                                        self.advance();
                                    }
                                }
                                self.expect(Token::RParen)?;
                                expr = Expr::MethodCall {
                                    obj: Box::new(expr),
                                    method,
                                    args,
                                };
                            } else {
                                expr = Expr::Field {
                                    obj: Box::new(expr),
                                    field: method,
                                };
                            }
                        }
                        _ => return Err("Expected method name after dot".to_string()),
                    }
                }
                Token::LBracket => {
                    self.advance();
                    let index = self.parse_expr()?;
                    self.expect(Token::RBracket)?;
                    expr = Expr::Index {
                        obj: Box::new(expr),
                        index: Box::new(index),
                    };
                }
                _ => break,
            }
        }

        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        match self.peek().clone() {
            Token::Number(n) => {
                self.advance();
                Ok(Expr::Number(n))
            }
            Token::Float(n) => {
                self.advance();
                Ok(Expr::Float(n))
            }
            Token::String(s) => {
                self.advance();
                Ok(Expr::String(s))
            }
            Token::TemplateString(nodes) => {
                self.advance();
                let mut out = Vec::new();
                for node in nodes {
                    match node {
                        crate::lexer::TemplateNode::Text(t) => {
                            out.push(crate::ast::TemplateStringNode::Text(t));
                        }
                        crate::lexer::TemplateNode::Expr(tokens) => {
                            let mut p = Parser::new(tokens);
                            let expr = p.parse_expr()?;
                            out.push(crate::ast::TemplateStringNode::Expr(Box::new(expr)));
                        }
                    }
                }
                Ok(Expr::TemplateString(out))
            }
            Token::True => {
                self.advance();
                Ok(Expr::Bool(true))
            }
            Token::False => {
                self.advance();
                Ok(Expr::Bool(false))
            }
            Token::Nil => {
                self.advance();
                Ok(Expr::Nil)
            }
            Token::Id(name) => {
                self.advance();
                Ok(Expr::Id(name))
            }
            Token::Fn => self.parse_closure(),
            Token::LParen => {
                self.advance();
                let expr = self.parse_expr()?;
                self.expect(Token::RParen)?;
                Ok(expr)
            }
            Token::LBracket => self.parse_array(),
            Token::LBrace => self.parse_object(),
            _ => Err(format!("Unexpected token: {:?}", self.peek())),
        }
    }

    fn parse_closure(&mut self) -> Result<Expr, String> {
        self.expect(Token::Fn)?;
        self.expect(Token::LParen)?;
        let mut params = Vec::new();
        while self.peek() != &Token::RParen {
            match self.peek() {
                Token::Id(p) => {
                    params.push(p.clone());
                    self.advance();
                }
                _ => return Err("Expected parameter name".to_string()),
            }
            if self.peek() == &Token::Comma {
                self.advance();
            }
        }
        self.expect(Token::RParen)?;

        self.expect(Token::LBrace)?;
        let body = self.parse_block()?;
        self.expect(Token::RBrace)?;

        Ok(Expr::Closure { params, body })
    }

    fn parse_array(&mut self) -> Result<Expr, String> {
        self.expect(Token::LBracket)?;
        let mut elements = Vec::new();
        while self.peek() != &Token::RBracket {
            elements.push(self.parse_expr()?);
            if self.peek() == &Token::Comma {
                self.advance();
            }
        }
        self.expect(Token::RBracket)?;
        Ok(Expr::Array(elements))
    }

    fn parse_object(&mut self) -> Result<Expr, String> {
        self.expect(Token::LBrace)?;

        // Check if this is actually an array (no colons at top level initially)
        let checkpoint = self.pos;
        let mut is_array = true;
        let mut depth = 0;

        while self.pos < self.tokens.len() {
            match self.peek() {
                Token::LBrace | Token::LBracket | Token::LParen => depth += 1,
                Token::RBrace if depth == 0 => break,
                Token::RBrace | Token::RBracket | Token::RParen => depth -= 1,
                Token::Colon if depth == 0 => {
                    is_array = false;
                    break;
                }
                _ => {}
            }
            self.advance();
        }

        self.pos = checkpoint;

        if is_array {
            self.parse_array_or_object_as_array()
        } else {
            self.parse_object_proper()
        }
    }

    fn parse_array_or_object_as_array(&mut self) -> Result<Expr, String> {
        let mut elements = Vec::new();
        while self.peek() != &Token::RBrace {
            // Skip numeric keys like "0:", "1:", etc.
            if matches!(self.peek(), Token::Number(_) | Token::Float(_)) {
                self.advance();
                if self.peek() == &Token::Colon {
                    self.advance();
                }
            }
            elements.push(self.parse_expr()?);
            if self.peek() == &Token::Comma {
                self.advance();
            }
        }
        self.expect(Token::RBrace)?;
        Ok(Expr::Array(elements))
    }

    fn parse_object_proper(&mut self) -> Result<Expr, String> {
        let mut fields = Vec::new();
        while self.peek() != &Token::RBrace {
            let key = match self.peek() {
                Token::Id(k) => {
                    let k = k.clone();
                    self.advance();
                    k
                }
                Token::String(s) => {
                    let s = s.clone();
                    self.advance();
                    s
                }
                Token::Number(n) => {
                    let n = *n as i64;
                    self.advance();
                    n.to_string()
                }
                Token::Float(n) => {
                    let n = *n;
                    self.advance();
                    n.to_string()
                }
                _ => return Err("Expected object key".to_string()),
            };

            self.expect(Token::Colon)?;
            let value = self.parse_expr()?;
            fields.push((key, value));

            if self.peek() == &Token::Comma {
                self.advance();
            }
        }
        self.expect(Token::RBrace)?;
        Ok(Expr::Object(fields))
    }
}

pub fn parse(tokens: Vec<Token>) -> Result<Vec<Stmt>, String> {
    let mut parser = Parser::new(tokens);
    parser.parse_program()
}
