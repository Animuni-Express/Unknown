//! Syntax trees shared by the parser and interpreter.

use crate::diagnostic::Span;

#[derive(Debug, Clone)]
/// Expressions produce runtime values; `Located` carries their diagnostic position.
// ============================================================
// EXPRESSION SYNTAX
//
// Represent expressions before evaluation.
// Located nodes retain source positions for diagnostics.
// ============================================================
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
/// Operators retained after lexing, independent of their token spelling.
// ============================================================
// OPERATORS
//
// Represent the arithmetic, comparison, and unary operators.
// ============================================================
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
/// Statements describe execution and may carry a source location.
// ============================================================
// STATEMENT SYNTAX
//
// Represent assignments, control flow, functions, and imports.
// ============================================================
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
    Import {
        name: String,
        alias: Option<String>,
    },
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
    Break,
    Continue,
}
