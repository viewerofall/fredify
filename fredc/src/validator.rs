use crate::ast::{Expr, Stmt};
use std::collections::HashSet;

pub struct Validator {
    defined_vars: HashSet<String>,
    errors: Vec<String>,
}

impl Validator {
    pub fn new() -> Self {
        Validator {
            defined_vars: HashSet::new(),
            errors: Vec::new(),
        }
    }

    pub fn validate(&mut self, stmts: &[Stmt]) -> Result<(), Vec<String>> {
        for stmt in stmts {
            self.validate_stmt(stmt);
        }
        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors.clone())
        }
    }

    fn validate_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::FnDef { name, params, body } => {
                self.defined_vars.insert(name.clone());
                let saved = self.defined_vars.clone();
                for param in params {
                    self.defined_vars.insert(param.clone());
                }
                for s in body {
                    self.validate_stmt(s);
                }
                self.defined_vars = saved;
            }
            Stmt::Let { name, value } => {
                if let Some(v) = value {
                    self.validate_expr(v);
                }
                self.defined_vars.insert(name.clone());
            }
            Stmt::Assign { target, value } => {
                if !self.defined_vars.contains(target) {
                    self.errors.push(format!("Undefined variable: '{}'", target));
                }
                self.validate_expr(value);
            }
            Stmt::If { cond, then_body, else_body } => {
                self.validate_expr(cond);
                for s in then_body {
                    self.validate_stmt(s);
                }
                if let Some(els) = else_body {
                    for s in els {
                        self.validate_stmt(s);
                    }
                }
            }
            Stmt::While { cond, body } => {
                self.validate_expr(cond);
                for s in body {
                    self.validate_stmt(s);
                }
            }
            Stmt::Loop { var, from, to, step, body } => {
                self.validate_expr(from);
                self.validate_expr(to);
                if let Some(s) = step {
                    self.validate_expr(s);
                }
                self.defined_vars.insert(var.clone());
                for s in body {
                    self.validate_stmt(s);
                }
            }
            Stmt::ForIn { var, iter, body } => {
                self.validate_expr(iter);
                self.defined_vars.insert(var.clone());
                for s in body {
                    self.validate_stmt(s);
                }
            }
            Stmt::Switch { expr, cases } => {
                self.validate_expr(expr);
                for (case_expr, body) in cases {
                    if let Some(e) = case_expr {
                        self.validate_expr(e);
                    }
                    for s in body {
                        self.validate_stmt(s);
                    }
                }
            }
            Stmt::Return(Some(e)) => self.validate_expr(e),
            Stmt::Expr(e) => self.validate_expr(e),
            _ => {}
        }
    }

    fn validate_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Id(name) => {
                let builtins = vec!["Math", "table", "os", "io", "string", "print", "to_int", "to_float", "to_string", "to_int_str"];
                if !self.defined_vars.contains(name) && !builtins.contains(&name.as_str()) {
                    self.errors.push(format!("Undefined variable: '{}'", name));
                }
            }
            Expr::BinOp { left, op, right } => {
                self.validate_expr(left);
                self.validate_expr(right);
                if op == "+" {
                    // Could be number or string, both valid
                }
            }
            Expr::UnOp { expr, .. } => self.validate_expr(expr),
            Expr::Call { func, args } => {
                self.validate_expr(func);
                for arg in args {
                    self.validate_expr(arg);
                }
            }
            Expr::MethodCall { obj, method, args } => {
                self.validate_expr(obj);
                for arg in args {
                    self.validate_expr(arg);
                }
                // Check for invalid method combinations
                match (obj.as_ref(), method.as_str()) {
                    (Expr::Id(n), m) if n == "table" => {
                        if !matches!(m, "insert" | "remove" | "concat" | "sort") {
                            self.errors.push(format!("Unknown table method: '{}'", m));
                        }
                    }
                    (Expr::Id(n), m) if n == "os" => {
                        if !matches!(m, "time" | "exit" | "getenv" | "system") {
                            self.errors.push(format!("Unknown os method: '{}'", m));
                        }
                    }
                    (Expr::Id(n), m) if n == "io" => {
                        if !matches!(m, "open" | "close" | "read" | "write") {
                            self.errors.push(format!("Unknown io method: '{}'", m));
                        }
                    }
                    (Expr::Id(n), m) if n == "string" => {
                        if !matches!(m, "find" | "split") {
                            self.errors.push(format!("Unknown string method: '{}'", m));
                        }
                    }
                    _ => {}
                }
            }
            Expr::Index { obj, index } => {
                self.validate_expr(obj);
                self.validate_expr(index);
            }
            Expr::Field { obj, .. } => self.validate_expr(obj),
            Expr::Array(elems) => {
                for e in elems {
                    self.validate_expr(e);
                    if matches!(e, Expr::String(_)) {
                        self.errors.push("Arrays cannot contain strings (only numbers)".to_string());
                    }
                }
            }
            Expr::Closure { params, body } => {
                let saved = self.defined_vars.clone();
                for p in params {
                    self.defined_vars.insert(p.clone());
                }
                for s in body {
                    self.validate_stmt(s);
                }
                self.defined_vars = saved;
            }
            Expr::TemplateString(nodes) => {
                for node in nodes {
                    if let crate::ast::TemplateStringNode::Expr(e) = node {
                        self.validate_expr(e);
                    }
                }
            }
            _ => {}
        }
    }
}
