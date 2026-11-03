use crate::ast::{Expr, Stmt, TokenOp};
use crate::diagnostic::{Diagnostic, Span};
use crate::value::Value;
use std::collections::HashMap;
use std::io::Write;
use std::{cell::RefCell, rc::Rc};

type Scope = HashMap<String, (Value, Option<String>)>;

pub struct Interpreter<'a> {
    scopes: Vec<Scope>,
    output: &'a mut dyn Write,
    function_depth: usize,
    current_span: Option<Span>,
    function_scopes: Vec<usize>,
}

impl<'a> Interpreter<'a> {
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
            current_span: None,
            function_scopes: Vec::new(),
        }
    }
    pub fn run(&mut self, statements: &[Stmt]) -> Result<(), Diagnostic> {
        self.block(statements).map(|_| ())
    }
    fn block(&mut self, statements: &[Stmt]) -> Result<Option<Value>, Diagnostic> {
        for statement in statements {
            if let Some(value) = self.statement(statement)? {
                return Ok(Some(value));
            }
        }
        Ok(None)
    }
    fn lookup_scope(&self, name: &str) -> Option<usize> {
        let start = self.function_scopes.last().copied().unwrap_or(0);
        (start..self.scopes.len())
            .rev()
            .find(|&i| self.scopes[i].contains_key(name))
            .or_else(|| self.scopes[0].contains_key(name).then_some(0))
    }
    fn scoped_block(&mut self, statements: &[Stmt]) -> Result<Option<Value>, Diagnostic> {
        self.scopes.push(Scope::new());
        let result = self.block(statements);
        self.scopes.pop();
        result
    }
    fn statement(&mut self, statement: &Stmt) -> Result<Option<Value>, Diagnostic> {
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
                let index = if declared_type.is_some() {
                    self.scopes.len() - 1
                } else {
                    self.lookup_scope(name)
                        .filter(|&i| i != 0 || self.function_scopes.is_empty())
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
                let scope = self
                    .lookup_scope(name)
                    .ok_or_else(|| self.error("E2001", format!("undefined name '{name}'")))?;
                if scope == 0 && !self.function_scopes.is_empty() {
                    self.scopes
                        .last_mut()
                        .expect("scope exists")
                        .insert(name.clone(), (value, None));
                    return Ok(None);
                }
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
            Stmt::Import(name) => {
                if !matches!(name.as_str(), "file" | "process") {
                    return Err(self.error("E4001", format!("unknown module '{name}'")));
                }
                self.scopes
                    .last_mut()
                    .expect("scope exists")
                    .insert(name.clone(), (Value::Module(name.clone()), None));
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
                    if let Some(value) = self.scoped_block(body)? {
                        return Ok(Some(value));
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
                    let mut execute = |value: Value| -> Result<Option<Value>, Diagnostic> {
                        self.scopes
                            .last_mut()
                            .expect("loop scope")
                            .insert(name.clone(), (value, None));
                        self.scoped_block(body)
                    };
                    match iterable {
                        Value::List(list) => {
                            let values = list.borrow().clone();
                            for value in values {
                                if let Some(value) = execute(value)? {
                                    return Ok(Some(value));
                                }
                            }
                        }
                        Value::Range(mut current, end, step) => {
                            while if step > 0 {
                                current < end
                            } else {
                                current > end
                            } {
                                if let Some(value) = execute(Value::Integer(current))? {
                                    return Ok(Some(value));
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
                return Ok(Some(match value {
                    Some(expr) => self.evaluate(expr)?,
                    None => Value::None,
                }));
            }
        }
        Ok(None)
    }
    fn condition(&mut self, expr: &Expr) -> Result<bool, Diagnostic> {
        match self.evaluate(expr)? {
            Value::Boolean(b) => Ok(b),
            _ => Err(self.error("E2002", "condition must be a bool")),
        }
    }
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
            Expr::Variable(name) => self
                .lookup_scope(name)
                .and_then(|i| self.scopes[i].get(name))
                .map(|(v, _)| v.clone())
                .ok_or_else(|| {
                    let mut error = self.error("E2001", format!("undefined name '{name}'"));
                    if let Some(suggestion) = self.suggest(name) {
                        error.help = Some(format!("did you mean '{suggestion}'?"));
                    }
                    error
                }),
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
                (Value::File(file), member @ ("read" | "write" | "close")) => {
                    Ok(Value::FileMethod(file, member.to_owned()))
                }
                (Value::Module(module), member)
                    if matches!(
                        (module.as_str(), member),
                        ("file", "read" | "write" | "append" | "exists" | "delete")
                            | ("process", "run")
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
                        self.function_depth += 1;
                        let result = self.block(&body);
                        self.function_depth -= 1;
                        self.function_scopes.pop();
                        self.scopes.pop();
                        let value = result?.unwrap_or(Value::None);
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
    fn list_index(&self, index: &Value, len: usize) -> Result<usize, Diagnostic> {
        match index {
            Value::Integer(n) if *n >= 0 && (*n as u128) < len as u128 => Ok(*n as usize),
            Value::Integer(_) => Err(self.error("E2004", "list index out of bounds")),
            _ => Err(self.error("E2002", "list index must be an integer")),
        }
    }
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
    fn native(&self, module: &str, name: &str, args: Vec<Value>) -> Result<Value, Diagnostic> {
        use std::io::Write as _;
        let io_error =
            |error: std::io::Error| self.error("E5001", format!("{module}.{name}: {error}"));
        match (module, name, args.as_slice()) {
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
    fn error(&self, code: &'static str, message: impl Into<String>) -> Diagnostic {
        Diagnostic::new(
            code,
            message,
            self.current_span.clone().expect("AST nodes carry spans"),
        )
    }
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
