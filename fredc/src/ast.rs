#[derive(Debug, Clone)]
pub enum Stmt {
    FnDef {
        name: String,
        params: Vec<String>,
        body: Vec<Stmt>,
    },
    Let {
        name: String,
        value: Option<Expr>,
    },
    Assign {
        target: String,
        value: Expr,
    },
    AssignIndex {
        obj: Expr,
        index: Expr,
        value: Expr,
    },
    AssignField {
        obj: Expr,
        field: String,
        value: Expr,
    },
    If {
        cond: Expr,
        then_body: Vec<Stmt>,
        else_body: Option<Vec<Stmt>>,
    },
    While {
        cond: Expr,
        body: Vec<Stmt>,
    },
    Loop {
        var: String,
        from: Expr,
        to: Expr,
        step: Option<Expr>,
        body: Vec<Stmt>,
    },
    Return(Option<Expr>),
    Break,
    Expr(Expr),
    ForIn {
        var: String,
        iter: Expr,
        body: Vec<Stmt>,
    },
    Switch {
        expr: Expr,
        cases: Vec<(Option<Expr>, Vec<Stmt>)>,
    },
}

#[derive(Debug, Clone)]
pub enum Expr {
    Number(f64),
    Float(f64),
    String(String),
    Bool(bool),
    Nil,
    Id(String),
    BinOp {
        left: Box<Expr>,
        op: String,
        right: Box<Expr>,
    },
    UnOp {
        op: String,
        expr: Box<Expr>,
    },
    Call {
        func: Box<Expr>,
        args: Vec<Expr>,
    },
    MethodCall {
        obj: Box<Expr>,
        method: String,
        args: Vec<Expr>,
    },
    Index {
        obj: Box<Expr>,
        index: Box<Expr>,
    },
    Field {
        obj: Box<Expr>,
        field: String,
    },
    Array(Vec<Expr>),
    Object(Vec<(String, Expr)>),
    Closure {
        params: Vec<String>,
        body: Vec<Stmt>,
    },
    TemplateString(Vec<TemplateStringNode>),
    Ternary {
        cond: Box<Expr>,
        then_expr: Box<Expr>,
        else_expr: Box<Expr>,
    },
}

#[derive(Debug, Clone)]
pub enum TemplateStringNode {
    Text(String),
    Expr(Box<Expr>),
}
