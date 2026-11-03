use crate::ast::Stmt;
use std::collections::HashMap;
use std::fmt;
use std::{cell::RefCell, rc::Rc};

#[derive(Debug, Clone)]
pub enum Value {
    Integer(i64),
    String(String),
    Boolean(bool),
    None,
    Function {
        parameters: Vec<(String, Option<String>)>,
        return_type: Option<String>,
        body: Vec<Stmt>,
    },
    Print,
    Builtin(&'static str),
    List(Rc<RefCell<Vec<Value>>>),
    Map(Rc<RefCell<HashMap<String, Value>>>),
    ListAppend(Rc<RefCell<Vec<Value>>>),
    Range(i64, i64, i64),
    Module(String),
    Native(String, String),
    File(Rc<RefCell<Option<std::fs::File>>>),
    FileMethod(Rc<RefCell<Option<std::fs::File>>>, String),
    ProcessResult {
        stdout: String,
        stderr: String,
        code: i64,
    },
}

impl Value {
    pub fn matches_type(&self, name: &str) -> bool {
        matches!(
            (self, name),
            (Self::Integer(_), "int") | (Self::String(_), "str") | (Self::Boolean(_), "bool")
        )
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Integer(n) => write!(f, "{n}"),
            Self::String(s) => write!(f, "{s}"),
            Self::Boolean(b) => write!(f, "{b}"),
            Self::None => write!(f, "none"),
            Self::Function { .. } | Self::Print => write!(f, "<function>"),
            Self::Builtin(_) | Self::ListAppend(_) => write!(f, "<function>"),
            Self::List(values) => match values.try_borrow_mut() {
                Ok(values) => write!(
                    f,
                    "[{}]",
                    values
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                Err(_) => write!(f, "[...]"),
            },
            Self::Map(values) => match values.try_borrow_mut() {
                Ok(values) => write!(
                    f,
                    "{{{}}}",
                    values
                        .iter()
                        .map(|(key, value)| format!("{key}: {value}"))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                Err(_) => write!(f, "{{...}}"),
            },
            Self::Range(start, end, step) => write!(f, "range({start}, {end}, {step})"),
            Self::Module(name) => write!(f, "<module {name}>"),
            Self::Native(_, _) => write!(f, "<function>"),
            Self::File(_) => write!(f, "<file>"),
            Self::FileMethod(_, _) => write!(f, "<function>"),
            Self::ProcessResult { .. } => write!(f, "<process result>"),
        }
    }
}
