use crate::diagnostic::Span;

#[derive(Debug, Clone)]
pub enum Expr {
    Located(Box<Expr>, Span),
    Integer(i64),
    String(String),
    Boolean(bool),
    None,
    Variable(String),
    Unary(TokenOp, Box<Expr>),
    Binary(Box<Expr>, TokenOp, Box<Expr>),
    Call(Box<Expr>, Vec<Expr>),
    List(Vec<Expr>),
    Map(Vec<(Expr, Expr)>),
    Index(Box<Expr>, Box<Expr>),
    Member(Box<Expr>, String),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TokenOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Not,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Located(Box<Stmt>, Span),
    Assign {
        name: String,
        declared_type: Option<String>,
        value: Expr,
    },
    CompoundAssign {
        name: String,
        op: TokenOp,
        value: Expr,
    },
    IndexAssign {
        target: Expr,
        index: Expr,
        value: Expr,
    },
    Import(String),
    Expression(Expr),
    If {
        condition: Expr,
        body: Vec<Stmt>,
        otherwise: Vec<Stmt>,
    },
    While {
        condition: Expr,
        body: Vec<Stmt>,
    },
    For {
        name: String,
        iterable: Expr,
        body: Vec<Stmt>,
    },
    Function {
        name: String,
        parameters: Vec<(String, Option<String>)>,
        return_type: Option<String>,
        body: Vec<Stmt>,
    },
    Return(Option<Expr>),
}
