use crate::ast::{Expr, Stmt};
use std::collections::HashSet;

pub struct Validator {
    defined_vars: HashSet<String>,
    errors: Vec<String>,
    allow_nuke: bool,
}

impl Validator {
    pub fn new() -> Self {
        Validator {
            defined_vars: HashSet::new(),
            errors: Vec::new(),
            allow_nuke: false,
        }
    }

    // nuke() is only permitted in genuine .fred sources, not .lua/.js inputs.
    pub fn set_allow_nuke(&mut self, allow: bool) {
        self.allow_nuke = allow;
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
            Stmt::AssignIndex { obj, index, value } => {
                self.validate_expr(obj);
                self.validate_expr(index);
                self.validate_expr(value);
            }
            Stmt::AssignField { obj, value, .. } => {
                self.validate_expr(obj);
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
            Stmt::Break => {}, // Break is always valid in loops
            Stmt::Expr(e) => self.validate_expr(e),
            _ => {}
        }
    }

    fn validate_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Id(name) => {
                let builtins = vec!["Math", "table", "os", "io", "http", "string", "print", "to_int", "to_float", "to_string", "to_int_str", "input_key", "read_line", "nuke"];
                if !self.defined_vars.contains(name) && !builtins.contains(&name.as_str()) {
                    self.errors.push(format!("Undefined variable: '{}'. Did you forget to declare it with 'let'?", name));
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
                if let Expr::Id(name) = func.as_ref() {
                    if name == "nuke" && !self.allow_nuke {
                        self.errors.push(
                            "'nuke' is a .fred-only call and cannot be used in .lua or .js sources".to_string(),
                        );
                    }
                }
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
                match obj.as_ref() {
                    Expr::Id(n) if n == "Math" => {
                        if !matches!(method.as_str(), "abs" | "sqrt" | "pow" | "floor" | "ceil" | "round" | "max" | "min" | "random") {
                            self.errors.push(format!("Unknown Math method: '{}'. Available: abs, sqrt, pow, floor, ceil, round, max, min, random", method));
                        }
                    }
                    Expr::Id(n) if n == "table" => {
                        if !matches!(method.as_str(), "insert" | "remove" | "concat" | "sort") {
                            self.errors.push(format!("Unknown table method: '{}'. Available: insert, remove, concat, sort", method));
                        }
                    }
                    Expr::Id(n) if n == "os" => {
                        if !matches!(method.as_str(), "time" | "exit" | "getenv" | "system" | "sleep") {
                            self.errors.push(format!("Unknown os method: '{}'. Available: time, exit, getenv, system, sleep", method));
                        }
                    }
                    Expr::Id(n) if n == "http" => {
                        if !matches!(method.as_str(), "get" | "post" | "get_file") {
                            self.errors.push(format!("Unknown http method: '{}'. Available: get, post, get_file", method));
                        }
                    }
                    Expr::Id(n) if n == "io" => {
                        if !matches!(method.as_str(), "open" | "close" | "read" | "write") {
                            self.errors.push(format!("Unknown io method: '{}'. Available: open, close, read, write", method));
                        }
                    }
                    Expr::Id(n) if n == "string" => {
                        if !matches!(method.as_str(), "find" | "split") {
                            self.errors.push(format!("Unknown string method: '{}'. Available: find, split", method));
                        }
                    }
                    _ => {
                        // Check if method exists for arrays and strings
                        let valid_array_methods = vec!["map", "filter", "reduce", "slice", "join", "includes", "len", "push", "pop"];
                        let valid_string_methods = vec!["length", "uppercase", "lowercase", "substring"];

                        if !valid_array_methods.contains(&method.as_str()) && !valid_string_methods.contains(&method.as_str()) {
                            // Allow it for now since we can't determine type at validation time
                            // Better error handling would require type inference
                        }
                    }
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
            Expr::Object(fields) => {
                for (_, v) in fields {
                    self.validate_expr(v);
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
            Expr::Ternary { cond, then_expr, else_expr } => {
                self.validate_expr(cond);
                self.validate_expr(then_expr);
                self.validate_expr(else_expr);
            }
            _ => {}
        }
    }
}
