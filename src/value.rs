//! Runtime values, including shared mutable collections and callable handles.

use crate::ast::Stmt;
use std::collections::HashMap;
use std::fmt;
use std::{cell::RefCell, rc::Rc};

/// Exported module bindings paired with their optional declared types.
pub type ModuleScope = HashMap<String, (Value, Option<String>)>;

#[derive(Debug, Clone)]
/// Values evaluated by the interpreter; cloned collections share their contents.
// ============================================================
// RUNTIME VALUES
//
// Represent primitive values, collections, functions, modules, and host handles.
// Lists and maps share their contents when a value is cloned.
// ============================================================
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
    UserModule(Rc<ModuleScope>),
    BoundFunction(Box<Value>, Rc<ModuleScope>),
    Native(String, String),
    File(Rc<RefCell<Option<ush_core::filesystem::File>>>),
    FileMethod(Rc<RefCell<Option<ush_core::filesystem::File>>>, String),
    ProcessResult {
        stdout: String,
        stderr: String,
        code: i64,
    },
}

// ============================================================
// TYPE CHECKING
//
// Check values against supported declared types: int, str, and bool.
// ============================================================
impl Value {
    pub fn matches_type(&self, name: &str) -> bool {
        matches!(
            (self, name),
            (Self::Integer(_), "int") | (Self::String(_), "str") | (Self::Boolean(_), "bool")
        )
    }
}

// ============================================================
// VALUE DISPLAY
//
// Format values for print and string conversion.
// Recursive collections use an abbreviated representation.
// ============================================================
impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Integer(n) => write!(f, "{n}"),
            Self::String(s) => write!(f, "{s}"),
            Self::Boolean(b) => write!(f, "{b}"),
            Self::None => write!(f, "none"),
            Self::Function { .. } | Self::Print => write!(f, "<function>"),
            Self::Builtin(_) | Self::ListAppend(_) => write!(f, "<function>"),
            // A failed borrow also stops recursive collections from printing forever.
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
            Self::UserModule(_) => write!(f, "<module>"),
            Self::BoundFunction(_, _) => write!(f, "<function>"),
            Self::Native(_, _) => write!(f, "<function>"),
            Self::File(_) => write!(f, "<file>"),
            Self::FileMethod(_, _) => write!(f, "<function>"),
            Self::ProcessResult { .. } => write!(f, "<process result>"),
        }
    }
}
