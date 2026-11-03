//! Execute syntax trees, manage scopes and modules, and dispatch built-ins.

use crate::ast::{Expr, Stmt, TokenOp};
use crate::diagnostic::{Diagnostic, Span};
use crate::value::{ModuleScope, Value};
use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::path::PathBuf;
use std::{cell::RefCell, rc::Rc};

/// Control flow propagated through nested blocks until its owner handles it.
// ============================================================
// CONTROL FLOW
//
// Carries return, break, and continue through nested blocks.
// Loops consume break and continue; function calls consume return.
// ============================================================
enum Flow {
    Return(Value),
    Break,
    Continue,
}

/// Bindings retain their declared type so later assignments can be checked.
type Scope = HashMap<String, (Value, Option<String>)>;

/// Execution state shared across statements in a single program.
// ============================================================
// INTERPRETER STATE
//
// Stores scopes, output, source locations, and execution context.
// Module caches are shared with child interpreters during imports.
// ============================================================
pub struct Interpreter<'a> {
    scopes: Vec<Scope>,
    output: &'a mut dyn Write,
    function_depth: usize,
    loop_depth: usize,
    arguments: Vec<String>,
    started: std::time::Instant,
    current_span: Option<Span>,
    function_scopes: Vec<usize>,
    module_context: Vec<Option<Rc<ModuleScope>>>,
    module_root: Option<usize>,
    modules: Rc<RefCell<HashMap<PathBuf, Rc<ModuleScope>>>>,
    loading: Rc<RefCell<HashSet<PathBuf>>>,
}

impl<'a> Interpreter<'a> {
    // ============================================================
    // INITIALIZATION
    //
    // Create the global scope and register built-in functions.
    // The caller supplies the output stream used by print.
    // ============================================================
    pub fn new(output: &'a mut dyn Write) -> Self {
        let mut globals = Scope::new();
        globals.insert("print".into(), (Value::Print, None));
        for name in ["range", "len", "str", "open"] {
            globals.insert(name.into(), (Value::Builtin(name), None));
        }
        Self {
            scopes: vec![globals],
            output,
            function_depth: 0,
            loop_depth: 0,
            arguments: Vec::new(),
            started: std::time::Instant::now(),
            current_span: None,
            function_scopes: Vec::new(),
            module_context: Vec::new(),
            module_root: None,
            modules: Rc::new(RefCell::new(HashMap::new())),
            loading: Rc::new(RefCell::new(HashSet::new())),
        }
    }
    pub fn set_arguments(&mut self, arguments: Vec<String>) {
        self.arguments = arguments;
    }
    pub fn run(&mut self, statements: &[Stmt]) -> Result<(), Diagnostic> {
        self.block(statements).map(|_| ())
    }
    // Stop immediately when a statement transfers control out of this block.
    // ============================================================
    // BLOCK EXECUTION
    //
    // Run statements in order until one transfers control or fails.
    // Nested scoped blocks discard their temporary bindings afterward.
    // ============================================================
    fn block(&mut self, statements: &[Stmt]) -> Result<Option<Flow>, Diagnostic> {
        for statement in statements {
            if let Some(value) = self.statement(statement)? {
                return Ok(Some(value));
            }
        }
        Ok(None)
    }
    // Function boundaries hide caller locals; module functions use their own globals.
    // ============================================================
    // NAME LOOKUP
    //
    // Search visible local scopes, then applicable module and global bindings.
    // A function does not see the local variables of its caller.
    // ============================================================
    fn lookup_scope(&self, name: &str) -> Option<usize> {
        let start = self.function_scopes.last().copied().unwrap_or(0);
        (start..self.scopes.len())
            .rev()
            .find(|&i| self.scopes[i].contains_key(name))
            .or_else(|| {
                if self.module_context.last().is_some_and(Option::is_some) {
                    return None;
                }
                self.module_root
                    .filter(|&i| self.scopes[i].contains_key(name))
            })
            .or_else(|| self.scopes[0].contains_key(name).then_some(0))
    }
    // Pop the temporary scope even when execution returns an error or control flow.
    fn scoped_block(&mut self, statements: &[Stmt]) -> Result<Option<Flow>, Diagnostic> {
        self.scopes.push(Scope::new());
        let result = self.block(statements);
        self.scopes.pop();
        result
    }
    // Preserve the defining module when an exported function is called later.
    fn bind_module(value: Value, module: &Rc<ModuleScope>) -> Value {
        if matches!(value, Value::Function { .. }) {
            Value::BoundFunction(Box::new(value), module.clone())
        } else {
            value
        }
    }
    // Resolve imports relative to their source, cache modules, and detect cycles.
    // ============================================================
    // MODULE IMPORTS
    //
    // Resolve a .unk file relative to the importing source.
    // Canonical paths identify cached modules and detect circular imports.
    // ============================================================
    fn import_module(&mut self, name: &str) -> Result<Value, Diagnostic> {
        let filename = &self
            .current_span
            .as_ref()
            .expect("import has a span")
            .filename;
        let base = if filename.starts_with('<') {
            std::path::Path::new(".")
        } else {
            std::path::Path::new(filename)
                .parent()
                .unwrap_or(std::path::Path::new("."))
        };
        let path = base.join(format!("{name}.unk"));
        let path = path
            .canonicalize()
            .map_err(|e| self.error("E4001", format!("cannot import '{name}': {e}")))?;
        if let Some(module) = self.modules.borrow().get(&path).cloned() {
            return Ok(Value::UserModule(module));
        }
        if !self.loading.borrow_mut().insert(path.clone()) {
            return Err(self.error("E4003", format!("circular import of '{name}'")));
        }
        let result = (|| {
            let source = std::fs::read_to_string(&path)
                .map_err(|e| self.error("E4001", format!("cannot import '{name}': {e}")))?;
            let filename = path.to_string_lossy();
            let tokens = crate::lexer::lex(&source, &filename)?;
            let statements = crate::parser::parse(tokens)?;
            let mut child = Interpreter::new(self.output);
            child.arguments = self.arguments.clone();
            child.started = self.started;
            child.modules = self.modules.clone();
            child.loading = self.loading.clone();
            child.scopes.push(Scope::new());
            child.module_root = Some(child.scopes.len() - 1);
            child.run(&statements)?;
            Ok(Rc::new(child.scopes.pop().expect("module scope exists")))
        })();
        self.loading.borrow_mut().remove(&path);
        let module = result?;
        self.modules.borrow_mut().insert(path, module.clone());
        Ok(Value::UserModule(module))
    }
    // Execute one statement, propagating return and loop control to enclosing blocks.
    // ============================================================
    // STATEMENT EXECUTION
    //
    // Execute assignments, branches, loops, imports, and function definitions.
    // Return, break, and continue propagate through the enclosing blocks.
    // ============================================================
    fn statement(&mut self, statement: &Stmt) -> Result<Option<Flow>, Diagnostic> {
        match statement {
            Stmt::Located(stmt, span) => {
                let previous = self.current_span.replace(span.clone());
                let result = self.statement(stmt);
                self.current_span = previous;
                return result;
            }
            Stmt::Assign {
                name,
                declared_type,
                value,
            } => {
                let value = self.evaluate(value)?;
                // Typed declarations create local bindings; ordinary assignments reuse visible ones.
                let index = if declared_type.is_some() {
                    self.scopes.len() - 1
                } else {
                    self.lookup_scope(name)
                        .filter(|&i| {
                            (i != 0
                                || (self.function_scopes.is_empty() && self.module_root.is_none()))
                                && (self.function_scopes.is_empty() || Some(i) != self.module_root)
                        })
                        .unwrap_or(self.scopes.len() - 1)
                };
                let scope = &self.scopes[index];
                let expected = declared_type
                    .as_ref()
                    .or_else(|| scope.get(name).and_then(|(_, t)| t.as_ref()));
                if let Some(expected) = expected {
                    if !value.matches_type(expected) {
                        return Err(self.error("E2002", format!("'{name}' requires {expected}")));
                    }
                }
                let declared_type = declared_type
                    .clone()
                    .or_else(|| scope.get(name).and_then(|(_, t)| t.clone()));
                self.scopes[index].insert(name.clone(), (value, declared_type));
            }
            Stmt::Expression(expr) => {
                self.evaluate(expr)?;
            }
            Stmt::CompoundAssign { name, op, value } => {
                let left = self.evaluate(&Expr::Variable(name.clone()))?;
                let right = self.evaluate(value)?;
                let value = Self::binary(left, *op, right)
                    .map_err(|message| self.error("E2004", message))?;
                let scope = self.lookup_scope(name);
                if !self.function_scopes.is_empty()
                    && (scope == Some(0) || scope == self.module_root || scope.is_none())
                {
                    self.scopes
                        .last_mut()
                        .expect("scope exists")
                        .insert(name.clone(), (value, None));
                    return Ok(None);
                }
                let scope =
                    scope.ok_or_else(|| self.error("E2001", format!("undefined name '{name}'")))?;
                if let Some(expected) = &self.scopes[scope][name].1 {
                    if !value.matches_type(expected) {
                        return Err(self.error("E2002", format!("'{name}' requires {expected}")));
                    }
                }
                self.scopes[scope]
                    .get_mut(name)
                    .expect("scope contains name")
                    .0 = value;
            }
            Stmt::IndexAssign {
                target,
                index,
                value,
            } => {
                let target = self.evaluate(target)?;
                let index = self.evaluate(index)?;
                let value = self.evaluate(value)?;
                match (target, index) {
                    (Value::List(list), index) => {
                        let index = self.list_index(&index, list.borrow().len())?;
                        list.borrow_mut()[index] = value;
                    }
                    (Value::Map(map), Value::String(key)) => {
                        map.borrow_mut().insert(key, value);
                    }
                    (Value::Map(_), _) => {
                        return Err(self.error("E2002", "map key must be a string"))
                    }
                    _ => {
                        return Err(self.error("E2002", "value does not support indexed assignment"))
                    }
                }
            }
            Stmt::Import { name, alias } => {
                let module = if matches!(
                    name.as_str(),
                    "file" | "process" | "envar" | "path" | "unknown-shell" | "system" | "time"
                ) {
                    Value::Module(name.clone())
                } else {
                    self.import_module(name)?
                };
                self.scopes
                    .last_mut()
                    .expect("scope exists")
                    .insert(alias.as_ref().unwrap_or(name).clone(), (module, None));
            }
            Stmt::If {
                condition,
                body,
                otherwise,
            } => {
                let condition = self.condition(condition)?;
                return self.scoped_block(if condition { body } else { otherwise });
            }
            Stmt::While { condition, body } => {
                while self.condition(condition)? {
                    self.loop_depth += 1;
                    let result = self.scoped_block(body);
                    self.loop_depth -= 1;
                    match result? {
                        Some(Flow::Break) => break,
                        Some(Flow::Continue) | None => {}
                        flow => return Ok(flow),
                    }
                }
            }
            Stmt::For {
                name,
                iterable,
                body,
            } => {
                let iterable = self.evaluate(iterable)?;
                self.scopes.push(Scope::new());
                let result = (|| {
                    let mut execute = |value: Value| -> Result<Option<Flow>, Diagnostic> {
                        self.scopes
                            .last_mut()
                            .expect("loop scope")
                            .insert(name.clone(), (value, None));
                        self.loop_depth += 1;
                        let result = self.scoped_block(body);
                        self.loop_depth -= 1;
                        result
                    };
                    match iterable {
                        Value::List(list) => {
                            // Iterate a snapshot so the loop body can mutate the original list.
                            let values = list.borrow().clone();
                            for value in values {
                                match execute(value)? {
                                    Some(Flow::Break) => break,
                                    Some(Flow::Continue) | None => {}
                                    flow => return Ok(flow),
                                }
                            }
                        }
                        Value::Range(mut current, end, step) => {
                            while if step > 0 {
                                current < end
                            } else {
                                current > end
                            } {
                                match execute(Value::Integer(current))? {
                                    Some(Flow::Break) => break,
                                    Some(Flow::Continue) | None => {}
                                    flow => return Ok(flow),
                                }
                                match current.checked_add(step) {
                                    Some(next) => current = next,
                                    None => break,
                                }
                            }
                        }
                        _ => return Err(self.error("E2002", "value is not iterable")),
                    }
                    Ok(None)
                })();
                self.scopes.pop();
                return result;
            }
            Stmt::Function {
                name,
                parameters,
                return_type,
                body,
            } => {
                self.scopes.last_mut().unwrap().insert(
                    name.clone(),
                    (
                        Value::Function {
                            parameters: parameters.clone(),
                            return_type: return_type.clone(),
                            body: body.clone(),
                        },
                        None,
                    ),
                );
            }
            Stmt::Return(value) => {
                if self.function_depth == 0 {
                    return Err(self.error("E2003", "'return' outside a function"));
                }
                return Ok(Some(Flow::Return(match value {
                    Some(expr) => self.evaluate(expr)?,
                    None => Value::None,
                })));
            }
            Stmt::Break | Stmt::Continue => {
                if self.loop_depth == 0 {
                    return Err(self.error("E2003", "loop control outside a loop"));
                }
                return Ok(Some(if matches!(statement, Stmt::Break) {
                    Flow::Break
                } else {
                    Flow::Continue
                }));
            }
        }
        Ok(None)
    }
    // Conditions require booleans rather than implicit truthiness.
    // ============================================================
    // CONDITIONS
    //
    // Evaluate a condition and require a boolean value.
    // ============================================================
    fn condition(&mut self, expr: &Expr) -> Result<bool, Diagnostic> {
        match self.evaluate(expr)? {
            Value::Boolean(b) => Ok(b),
            _ => Err(self.error("E2002", "condition must be a bool")),
        }
    }
    // Evaluate expressions recursively while retaining the most specific source span.
    // ============================================================
    // EXPRESSION EVALUATION
    //
    // Turn an Unknown expression into a runtime value.
    //
    // Examples:
    //     42       -> an integer value
    //     a + b    -> evaluate both operands and apply the operator
    //     foo()    -> evaluate the callee and its arguments, then call it
    //     list[0]  -> look up an element in a list
    // ============================================================
    fn evaluate(&mut self, expr: &Expr) -> Result<Value, Diagnostic> {
        match expr {
            Expr::Located(inner, span) => {
                let previous = self.current_span.replace(span.clone());
                let result = self.evaluate(inner);
                self.current_span = previous;
                result
            }
            Expr::Integer(n) => Ok(Value::Integer(*n)),
            Expr::String(s) => Ok(Value::String(s.clone())),
            Expr::Boolean(b) => Ok(Value::Boolean(*b)),
            Expr::None => Ok(Value::None),
            Expr::Variable(name) => {
                let scope = self.lookup_scope(name);
                if let Some(index) = scope.filter(|&i| i != 0) {
                    return Ok(self.scopes[index][name].0.clone());
                }
                let value = if let Some(Some(module)) = self.module_context.last() {
                    if let Some((value, _)) = module.get(name) {
                        return Ok(Self::bind_module(value.clone(), module));
                    }
                    match name.as_str() {
                        "print" => Some(Value::Print),
                        "range" => Some(Value::Builtin("range")),
                        "len" => Some(Value::Builtin("len")),
                        "str" => Some(Value::Builtin("str")),
                        "open" => Some(Value::Builtin("open")),
                        _ => None,
                    }
                } else {
                    scope.map(|i| self.scopes[i][name].0.clone())
                };
                value.ok_or_else(|| {
                    let mut error = self.error("E2001", format!("undefined name '{name}'"));
                    if let Some(suggestion) = self.suggest(name) {
                        error.help = Some(format!("did you mean '{suggestion}'?"));
                    }
                    error
                })
            }
            Expr::Unary(op, expr) => match (op, self.evaluate(expr)?) {
                (TokenOp::Subtract, Value::Integer(n)) => n
                    .checked_neg()
                    .map(Value::Integer)
                    .ok_or_else(|| self.error("E2004", "integer overflow")),
                (TokenOp::Not, Value::Boolean(b)) => Ok(Value::Boolean(!b)),
                _ => Err(self.error("E2002", "invalid unary operand")),
            },
            Expr::Binary(left, op, right) => {
                let left = self.evaluate(left)?;
                let right = self.evaluate(right)?;
                Self::binary(left, *op, right).map_err(|message| self.error("E2004", message))
            }
            Expr::List(items) => Ok(Value::List(Rc::new(RefCell::new(
                items
                    .iter()
                    .map(|item| self.evaluate(item))
                    .collect::<Result<Vec<_>, _>>()?,
            )))),
            Expr::Map(entries) => {
                let mut result = HashMap::new();
                for (key, value) in entries {
                    let Value::String(key) = self.evaluate(key)? else {
                        return Err(self.error("E2002", "map key must be a string"));
                    };
                    result.insert(key, self.evaluate(value)?);
                }
                Ok(Value::Map(Rc::new(RefCell::new(result))))
            }
            Expr::Index(target, index) => {
                let target = self.evaluate(target)?;
                let index = self.evaluate(index)?;
                match target {
                    Value::List(list) => {
                        let i = self.list_index(&index, list.borrow().len())?;
                        let value = list.borrow()[i].clone();
                        Ok(value)
                    }
                    Value::Map(map) => {
                        let Value::String(key) = index else {
                            return Err(self.error("E2002", "map key must be a string"));
                        };
                        let value = map.borrow().get(&key).cloned().ok_or_else(|| {
                            self.error("E2004", format!("map key '{key}' not found"))
                        })?;
                        Ok(value)
                    }
                    _ => Err(self.error("E2002", "value does not support indexing")),
                }
            }
            Expr::Member(target, name) => match (self.evaluate(target)?, name.as_str()) {
                (Value::List(list), "append") => Ok(Value::ListAppend(list)),
                (Value::UserModule(module), member) => module
                    .get(member)
                    .map(|(value, _)| Self::bind_module(value.clone(), &module))
                    .ok_or_else(|| self.error("E4002", format!("module has no member '{member}'"))),
                (Value::File(file), member @ ("read" | "write" | "close")) => {
                    Ok(Value::FileMethod(file, member.to_owned()))
                }
                (Value::Module(module), member)
                    if matches!(
                        (module.as_str(), member),
                        ("file", "read" | "write" | "append" | "exists" | "delete")
                            | ("process", "run" | "cwd" | "chdir" | "args" | "pid")
                            | ("envar", "get" | "set" | "unset" | "exists" | "list")
                            | (
                                "path",
                                "join"
                                    | "parent"
                                    | "filename"
                                    | "extension"
                                    | "absolute"
                                    | "exists"
                                    | "is_file"
                                    | "is_dir"
                            )
                            | (
                                "unknown-shell",
                                "read_line" | "write" | "write_line" | "clear" | "is_tty"
                            )
                            | ("system", "os" | "arch" | "hostname" | "cpu_count")
                            | ("time", "now" | "sleep" | "monotonic")
                    ) =>
                {
                    Ok(Value::Native(module, member.to_owned()))
                }
                (Value::ProcessResult { stdout, .. }, "stdout") => Ok(Value::String(stdout)),
                (Value::ProcessResult { stderr, .. }, "stderr") => Ok(Value::String(stderr)),
                (Value::ProcessResult { code, .. }, "code") => Ok(Value::Integer(code)),
                _ => Err(self.error("E2002", format!("unknown member '{name}'"))),
            },
            Expr::Call(callee, args) => {
                let callee = self.evaluate(callee)?;
                let args = args
                    .iter()
                    .map(|arg| self.evaluate(arg))
                    .collect::<Result<Vec<_>, _>>()?;
                let (callee, module) = match callee {
                    Value::BoundFunction(function, module) => (*function, Some(module)),
                    value => (value, None),
                };
                match callee {
                    Value::Builtin(name) => self.builtin(name, args),
                    Value::Native(module, name) => self.native(&module, &name, args),
                    Value::FileMethod(file, name) => {
                        use std::io::Read;
                        let mut file = file.borrow_mut();
                        let handle = file
                            .as_mut()
                            .ok_or_else(|| self.error("E5001", "file is closed"))?;
                        match (name.as_str(), args.as_slice()) {
                            ("read", []) => {
                                let mut text = String::new();
                                handle
                                    .read_to_string(&mut text)
                                    .map_err(|e| self.error("E5001", format!("file.read: {e}")))?;
                                Ok(Value::String(text))
                            }
                            ("write", [Value::String(text)]) => {
                                handle
                                    .write_all(text.as_bytes())
                                    .map_err(|e| self.error("E5001", format!("file.write: {e}")))?;
                                Ok(Value::None)
                            }
                            ("close", []) => {
                                *file = None;
                                Ok(Value::None)
                            }
                            _ => {
                                Err(self
                                    .error("E2002", format!("invalid arguments to file.{name}()")))
                            }
                        }
                    }
                    Value::ListAppend(list) => {
                        if args.len() != 1 {
                            return Err(self.error("E2002", "append expects one argument"));
                        }
                        list.borrow_mut().push(args[0].clone());
                        Ok(Value::None)
                    }
                    Value::Print => {
                        let text = args
                            .iter()
                            .map(ToString::to_string)
                            .collect::<Vec<_>>()
                            .join(" ");
                        writeln!(self.output, "{text}").map_err(|e| {
                            self.error("E2005", format!("could not write output: {e}"))
                        })?;
                        Ok(Value::None)
                    }
                    Value::Function {
                        parameters,
                        return_type,
                        body,
                    } => {
                        if parameters.len() != args.len() {
                            return Err(self.error(
                                "E2002",
                                format!(
                                    "expected {} arguments, got {}",
                                    parameters.len(),
                                    args.len()
                                ),
                            ));
                        }
                        let mut scope = Scope::new();
                        for ((name, declared_type), value) in parameters.into_iter().zip(args) {
                            if let Some(expected) = &declared_type {
                                if !value.matches_type(expected) {
                                    return Err(self
                                        .error("E2002", format!("'{name}' requires {expected}")));
                                }
                            }
                            scope.insert(name, (value, declared_type));
                        }
                        self.scopes.push(scope);
                        self.function_scopes.push(self.scopes.len() - 1);
                        self.module_context.push(module);
                        self.function_depth += 1;
                        // A function cannot break or continue a loop in its caller.
                        let outer_loop_depth = std::mem::replace(&mut self.loop_depth, 0);
                        let result = self.block(&body);
                        self.loop_depth = outer_loop_depth;
                        self.function_depth -= 1;
                        self.module_context.pop();
                        self.function_scopes.pop();
                        self.scopes.pop();
                        let value = match result? {
                            Some(Flow::Return(value)) => value,
                            None => Value::None,
                            _ => unreachable!("loop control validated"),
                        };
                        if let Some(expected) = return_type {
                            if !value.matches_type(&expected) {
                                return Err(
                                    self.error("E2002", format!("function must return {expected}"))
                                );
                            }
                        }
                        Ok(value)
                    }
                    _ => Err(self.error("E2002", "value is not callable")),
                }
            }
        }
    }
    // Validate before converting to usize to reject negative and out-of-range indices.
    // ============================================================
    // LIST INDEX VALIDATION
    //
    // Accept only nonnegative integer indices within the list bounds.
    // ============================================================
    fn list_index(&self, index: &Value, len: usize) -> Result<usize, Diagnostic> {
        match index {
            Value::Integer(n) if *n >= 0 && (*n as u128) < len as u128 => Ok(*n as usize),
            Value::Integer(_) => Err(self.error("E2004", "list index out of bounds")),
            _ => Err(self.error("E2002", "list index must be an integer")),
        }
    }
    // Dispatch global built-ins by name and validated argument shape.
    // ============================================================
    // GLOBAL BUILT-INS
    //
    // Dispatch str, open, len, and range using their argument shapes.
    // Invalid arguments produce a runtime diagnostic.
    // ============================================================
    fn builtin(&self, name: &str, args: Vec<Value>) -> Result<Value, Diagnostic> {
        match (name, args.as_slice()) {
            ("str", [value]) => Ok(Value::String(value.to_string())),
            ("open", [Value::String(path), Value::String(mode)]) => {
                let mut options = std::fs::OpenOptions::new();
                match mode.as_str() {
                    "r" => {
                        options.read(true);
                    }
                    "w" => {
                        options.write(true).create(true).truncate(true);
                    }
                    "a" => {
                        options.append(true).create(true);
                    }
                    _ => return Err(self.error("E2002", "open mode must be 'r', 'w', or 'a'")),
                }
                let file = options
                    .open(path)
                    .map_err(|e| self.error("E5001", format!("open: {e}")))?;
                Ok(Value::File(Rc::new(RefCell::new(Some(file)))))
            }
            ("len", [Value::String(value)]) => Ok(Value::Integer(value.chars().count() as i64)),
            ("len", [Value::List(value)]) => Ok(Value::Integer(value.borrow().len() as i64)),
            ("len", [Value::Map(value)]) => Ok(Value::Integer(value.borrow().len() as i64)),
            ("range", [Value::Integer(end)]) => Ok(Value::Range(0, *end, 1)),
            ("range", [Value::Integer(start), Value::Integer(end)]) => {
                Ok(Value::Range(*start, *end, 1))
            }
            ("range", [Value::Integer(start), Value::Integer(end), Value::Integer(step)])
                if *step != 0 =>
            {
                Ok(Value::Range(*start, *end, *step))
            }
            _ => Err(self.error("E2002", format!("invalid arguments to {name}()"))),
        }
    }
    // Native modules translate host results and I/O errors into language values.
    // ============================================================
    // NATIVE MODULES
    //
    // Dispatch host operations for files, processes, paths, and other modules.
    // Host failures are converted into interpreter diagnostics.
    // ============================================================
    fn native(&mut self, module: &str, name: &str, args: Vec<Value>) -> Result<Value, Diagnostic> {
        use std::io::IsTerminal;
        if module == "unknown-shell" {
            let result = match (name, args.as_slice()) {
                ("write", [Value::String(text)]) => self
                    .output
                    .write_all(text.as_bytes())
                    .and_then(|()| self.output.flush())
                    .map(|()| Value::None),
                ("write_line", [Value::String(text)]) => writeln!(self.output, "{text}")
                    .and_then(|()| self.output.flush())
                    .map(|()| Value::None),
                ("read_line", []) => {
                    let mut line = String::new();
                    std::io::stdin().read_line(&mut line).map(|_| {
                        if line.ends_with('\n') {
                            line.pop();
                            if line.ends_with('\r') {
                                line.pop();
                            }
                        }
                        Value::String(line)
                    })
                }
                ("clear", []) => ush_core::clear_screen_to(self.output).map(|()| Value::None),
                ("is_tty", []) => return Ok(Value::Boolean(std::io::stdout().is_terminal())),
                _ => {
                    return Err(
                        self.error("E2002", format!("invalid arguments to {module}.{name}()"))
                    )
                }
            };
            return result.map_err(|e| self.error("E5001", format!("{module}.{name}: {e}")));
        }
        use std::io::Write as _;
        let io_error =
            |error: std::io::Error| self.error("E5001", format!("{module}.{name}: {error}"));
        match (module, name, args.as_slice()) {
            ("process", "cwd", []) => std::env::current_dir()
                .map(|p| Value::String(p.to_string_lossy().into_owned()))
                .map_err(io_error),
            ("process", "chdir", [Value::String(path)]) => std::env::set_current_dir(path)
                .map(|()| Value::None)
                .map_err(io_error),
            ("process", "args", []) => Ok(Value::List(Rc::new(RefCell::new(
                self.arguments.iter().cloned().map(Value::String).collect(),
            )))),
            ("process", "pid", []) => Ok(Value::Integer(i64::from(std::process::id()))),
            ("envar", "get" | "exists" | "set" | "unset", [Value::String(key), ..]) => {
                let key = key.trim();
                if key.is_empty() || key.contains(['=', '\0']) {
                    return Err(self.error("E2002", "invalid environment variable name"));
                }
                match (name, args.as_slice()) {
                    ("get", [_]) => match std::env::var(key) {
                        Ok(v) => Ok(Value::String(v)),
                        Err(std::env::VarError::NotPresent) => Ok(Value::None),
                        Err(e) => Err(self.error("E5001", format!("envar.get: {e}"))),
                    },
                    ("exists", [_]) => Ok(Value::Boolean(std::env::var_os(key).is_some())),
                    ("set", [_, Value::String(value)]) if !value.contains('\0') => {
                        std::env::set_var(key, value);
                        Ok(Value::None)
                    }
                    ("unset", [_]) => {
                        std::env::remove_var(key);
                        Ok(Value::None)
                    }
                    _ => {
                        Err(self.error("E2002", format!("invalid arguments to {module}.{name}()")))
                    }
                }
            }
            ("envar", "list", []) => {
                let mut vars = HashMap::new();
                for (key, value) in std::env::vars_os() {
                    let (Ok(key), Ok(value)) = (key.into_string(), value.into_string()) else {
                        return Err(self.error("E5001", "environment contains non-UTF-8 data"));
                    };
                    vars.insert(key, Value::String(value));
                }
                Ok(Value::Map(Rc::new(RefCell::new(vars))))
            }
            ("path", "join", parts)
                if !parts.is_empty() && parts.iter().all(|p| matches!(p, Value::String(_))) =>
            {
                let mut path = PathBuf::new();
                for part in parts {
                    if let Value::String(part) = part {
                        path.push(part);
                    }
                }
                Ok(Value::String(path.to_string_lossy().into_owned()))
            }
            ("path", "parent" | "filename" | "extension", [Value::String(path)]) => {
                let path = std::path::Path::new(path);
                let value = match name {
                    "parent" => path.parent().map(|v| v.as_os_str()),
                    "filename" => path.file_name(),
                    _ => path.extension(),
                };
                Ok(value
                    .map(|v| Value::String(v.to_string_lossy().into_owned()))
                    .unwrap_or(Value::None))
            }
            ("path", "absolute", [Value::String(path)]) => std::path::absolute(path)
                .map(|p| Value::String(p.to_string_lossy().into_owned()))
                .map_err(io_error),
            ("path", "exists", [Value::String(path)]) => std::path::Path::new(path)
                .try_exists()
                .map(Value::Boolean)
                .map_err(io_error),
            ("path", "is_file" | "is_dir", [Value::String(path)]) => {
                match std::fs::metadata(path) {
                    Ok(meta) => Ok(Value::Boolean(if name == "is_file" {
                        meta.is_file()
                    } else {
                        meta.is_dir()
                    })),
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Value::Boolean(false)),
                    Err(e) => Err(io_error(e)),
                }
            }
            ("system", "os", []) => Ok(Value::String(std::env::consts::OS.into())),
            ("system", "arch", []) => Ok(Value::String(std::env::consts::ARCH.into())),
            ("system", "hostname", []) => hostname().map(Value::String).map_err(io_error),
            ("system", "cpu_count", []) => std::thread::available_parallelism()
                .map(|n| Value::Integer(n.get() as i64))
                .map_err(io_error),
            ("time", "now", []) => std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|n| Value::Integer(n.as_millis().min(i64::MAX as u128) as i64))
                .map_err(|e| self.error("E5001", format!("time.now: {e}"))),
            ("time", "monotonic", []) => Ok(Value::Integer(
                self.started.elapsed().as_millis().min(i64::MAX as u128) as i64,
            )),
            ("time", "sleep", [Value::Integer(ms)]) if *ms >= 0 => {
                std::thread::sleep(std::time::Duration::from_millis(*ms as u64));
                Ok(Value::None)
            }
            ("file", "read", [Value::String(path)]) => std::fs::read_to_string(path)
                .map(Value::String)
                .map_err(io_error),
            ("file", "write", [Value::String(path), Value::String(data)]) => {
                std::fs::write(path, data)
                    .map(|()| Value::None)
                    .map_err(io_error)
            }
            ("file", "append", [Value::String(path), Value::String(data)]) => {
                let mut file = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(path)
                    .map_err(io_error)?;
                file.write_all(data.as_bytes())
                    .map(|()| Value::None)
                    .map_err(io_error)
            }
            ("file", "exists", [Value::String(path)]) => std::path::Path::new(path)
                .try_exists()
                .map(Value::Boolean)
                .map_err(io_error),
            ("file", "delete", [Value::String(path)]) => std::fs::remove_file(path)
                .map(|()| Value::None)
                .map_err(io_error),
            ("process", "run", [Value::String(program), Value::List(arguments)]) => {
                let arguments = arguments.borrow();
                let mut command = std::process::Command::new(program);
                for argument in arguments.iter() {
                    let Value::String(argument) = argument else {
                        return Err(self.error("E2002", "process arguments must be strings"));
                    };
                    command.arg(argument);
                }
                let output = command.output().map_err(io_error)?;
                let stdout = String::from_utf8(output.stdout)
                    .map_err(|_| self.error("E5002", "process stdout is not UTF-8"))?;
                let stderr = String::from_utf8(output.stderr)
                    .map_err(|_| self.error("E5002", "process stderr is not UTF-8"))?;
                let code = output.status.code().ok_or_else(|| {
                    self.error("E5002", "process terminated without an exit code")
                })?;
                Ok(Value::ProcessResult {
                    stdout,
                    stderr,
                    code: i64::from(code),
                })
            }
            _ => Err(self.error("E2002", format!("invalid arguments to {module}.{name}()"))),
        }
    }
    // ============================================================
    // RUNTIME DIAGNOSTICS
    //
    // Attach runtime errors to the current expression or statement.
    // Nearby names can be suggested for unresolved identifiers.
    // ============================================================
    fn error(&self, code: &'static str, message: impl Into<String>) -> Diagnostic {
        Diagnostic::new(
            code,
            message,
            self.current_span.clone().expect("AST nodes carry spans"),
        )
    }
    // Offer a nearby visible name for likely spelling mistakes.
    fn suggest(&self, name: &str) -> Option<&str> {
        self.scopes
            .last()
            .into_iter()
            .chain(self.scopes.first())
            .flat_map(|scope| scope.keys())
            .filter_map(|candidate| {
                let distance = edit_distance(name, candidate);
                (distance <= 2 && distance < name.chars().count())
                    .then_some((distance, candidate.as_str()))
            })
            .min()
            .map(|(_, candidate)| candidate)
    }
    // Checked integer arithmetic turns overflow into a runtime error.
    // ============================================================
    // BINARY OPERATORS
    //
    // Apply arithmetic and comparisons to compatible values.
    // Integer arithmetic checks for overflow and division by zero.
    // ============================================================
    fn binary(left: Value, op: TokenOp, right: Value) -> Result<Value, String> {
        use TokenOp::*;
        match (left, right) {
            (Value::Integer(a), Value::Integer(b)) => match op {
                Add => a.checked_add(b),
                Subtract => a.checked_sub(b),
                Multiply => a.checked_mul(b),
                Divide if b == 0 => return Err("division by zero".into()),
                Divide => a.checked_div(b),
                Equal => return Ok(Value::Boolean(a == b)),
                NotEqual => return Ok(Value::Boolean(a != b)),
                Less => return Ok(Value::Boolean(a < b)),
                LessEqual => return Ok(Value::Boolean(a <= b)),
                Greater => return Ok(Value::Boolean(a > b)),
                GreaterEqual => return Ok(Value::Boolean(a >= b)),
                _ => return Err("invalid integer operation".into()),
            }
            .map(Value::Integer)
            .ok_or_else(|| "integer overflow".into()),
            (Value::String(a), Value::String(b)) => match op {
                Add => Ok(Value::String(a + &b)),
                Equal => Ok(Value::Boolean(a == b)),
                NotEqual => Ok(Value::Boolean(a != b)),
                Less => Ok(Value::Boolean(a < b)),
                LessEqual => Ok(Value::Boolean(a <= b)),
                Greater => Ok(Value::Boolean(a > b)),
                GreaterEqual => Ok(Value::Boolean(a >= b)),
                _ => Err("invalid string operation".into()),
            },
            (Value::Boolean(a), Value::Boolean(b)) => match op {
                Equal => Ok(Value::Boolean(a == b)),
                NotEqual => Ok(Value::Boolean(a != b)),
                _ => Err("invalid bool operation".into()),
            },
            (Value::None, Value::None) => match op {
                Equal => Ok(Value::Boolean(true)),
                NotEqual => Ok(Value::Boolean(false)),
                _ => Err("invalid none operation".into()),
            },
            _ => match op {
                Equal => Ok(Value::Boolean(false)),
                NotEqual => Ok(Value::Boolean(true)),
                _ => Err("operands have incompatible types".into()),
            },
        }
    }
}

/// Compute Levenshtein distance using one row of working storage.
// ============================================================
// NAME SUGGESTIONS
//
// Compute edit distance with a single row of working storage.
// ============================================================
fn edit_distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, x) in a.chars().enumerate() {
        let mut previous = row[0];
        row[0] = i + 1;
        for (j, y) in b.iter().enumerate() {
            let old = row[j + 1];
            row[j + 1] = (row[j + 1] + 1)
                .min(row[j] + 1)
                .min(previous + usize::from(x != *y));
            previous = old;
        }
    }
    row[b.len()]
}

// Platform-specific hostname implementations return host errors to the caller.
// ============================================================
// HOSTNAME SUPPORT
//
// Use the platform hostname API and report host or encoding errors.
// ============================================================
#[cfg(windows)]
fn hostname() -> std::io::Result<String> {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetComputerNameW(buffer: *mut u16, size: *mut u32) -> i32;
    }
    let mut buffer = [0u16; 256];
    let mut size = buffer.len() as u32;
    if unsafe { GetComputerNameW(buffer.as_mut_ptr(), &mut size) } == 0 {
        return Err(std::io::Error::last_os_error());
    }
    String::from_utf16(&buffer[..size as usize])
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}
#[cfg(unix)]
fn hostname() -> std::io::Result<String> {
    unsafe extern "C" {
        fn gethostname(name: *mut std::ffi::c_char, len: usize) -> i32;
    }
    let mut buffer = [0u8; 256];
    if unsafe { gethostname(buffer.as_mut_ptr().cast(), buffer.len()) } != 0 {
        return Err(std::io::Error::last_os_error());
    }
    let end = buffer
        .iter()
        .position(|b| *b == 0)
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "hostname too long"))?;
    String::from_utf8(buffer[..end].to_vec())
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}
#[cfg(not(any(unix, windows)))]
fn hostname() -> std::io::Result<String> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "hostname unavailable",
    ))
}
