use crate::ast::{Expr, Stmt, TokenOp};
use crate::diagnostic::{Diagnostic, Span};
use crate::lexer::{Kind, Token};

pub fn parse(tokens: Vec<Token>) -> Result<Vec<Stmt>, Diagnostic> {
    let mut parser = Parser { tokens, current: 0 };
    let mut statements = Vec::new();
    while !parser.at(&Kind::Eof) {
        statements.push(parser.statement()?);
    }
    Ok(statements)
}

struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

impl Parser {
    fn at(&self, kind: &Kind) -> bool {
        std::mem::discriminant(&self.tokens[self.current].kind) == std::mem::discriminant(kind)
    }
    fn word(&self, word: &str) -> bool {
        matches!(&self.tokens[self.current].kind, Kind::Identifier(name) if name == word)
    }
    fn take(&mut self, kind: &Kind) -> bool {
        if self.at(kind) {
            self.current += 1;
            true
        } else {
            false
        }
    }
    fn take_word(&mut self, word: &str) -> bool {
        if self.word(word) {
            self.current += 1;
            true
        } else {
            false
        }
    }
    fn span(&self) -> Span {
        let t = &self.tokens[self.current];
        Span {
            filename: t.source.clone(),
            line: t.line,
            column: t.column,
            length: t.length,
            source_line: t.source_line.clone(),
        }
    }
    fn error(&self, message: &str) -> Diagnostic {
        Diagnostic::new("E1001", message, self.span())
    }
    fn expect(&mut self, kind: &Kind, message: &str) -> Result<(), Diagnostic> {
        if self.take(kind) {
            Ok(())
        } else {
            Err(self.error(message))
        }
    }
    fn name(&mut self) -> Result<String, Diagnostic> {
        match self.tokens[self.current].kind.clone() {
            Kind::Identifier(name)
                if !matches!(
                    name.as_str(),
                    "fn" | "if" | "else" | "while" | "return" | "true" | "false" | "none" | "not"
                ) =>
            {
                self.current += 1;
                Ok(name)
            }
            _ => Err(self.error("expected a name")),
        }
    }
    fn type_name(&mut self) -> Option<String> {
        if self.word("int") || self.word("str") || self.word("bool") {
            if let Kind::Identifier(name) = self.tokens[self.current].kind.clone() {
                self.current += 1;
                return Some(name);
            }
        }
        None
    }
    fn statement(&mut self) -> Result<Stmt, Diagnostic> {
        let span = self.span();
        self.statement_inner()
            .map(|stmt| Stmt::Located(Box::new(stmt), span))
    }
    fn statement_inner(&mut self) -> Result<Stmt, Diagnostic> {
        if self.take_word("import") {
            let name = self.name()?;
            self.expect(&Kind::Newline, "expected end of line")?;
            return Ok(Stmt::Import(name));
        }
        if self.take_word("if") {
            let condition = self.parenthesized_condition()?;
            let body = self.block()?;
            let otherwise = if self.take_word("else") {
                if self.word("if") {
                    vec![self.statement()?]
                } else {
                    self.block()?
                }
            } else {
                Vec::new()
            };
            return Ok(Stmt::If {
                condition,
                body,
                otherwise,
            });
        }
        if self.take_word("while") {
            let condition = self.parenthesized_condition()?;
            let body = self.block()?;
            return Ok(Stmt::While { condition, body });
        }
        if self.take_word("for") {
            self.expect(&Kind::LeftParen, "expected '(' after 'for'")?;
            let name = self.name()?;
            if !self.take_word("in") {
                return Err(self.error("expected 'in'"));
            }
            let iterable = self.expression()?;
            self.expect(&Kind::RightParen, "expected ')' after iterable")?;
            let body = self.block()?;
            return Ok(Stmt::For {
                name,
                iterable,
                body,
            });
        }
        if self.take_word("fn") {
            let name = self.name()?;
            self.expect(&Kind::LeftParen, "expected '('")?;
            let mut parameters = Vec::new();
            if !self.at(&Kind::RightParen) {
                loop {
                    let declared_type = self.type_name();
                    let parameter = self.name()?;
                    if parameters.iter().any(|(n, _)| *n == parameter) {
                        return Err(self.error("duplicate parameter"));
                    }
                    parameters.push((parameter, declared_type));
                    if !self.take(&Kind::Comma) {
                        break;
                    }
                }
            }
            self.expect(&Kind::RightParen, "expected ')'")?;
            let return_type = if self.take(&Kind::Arrow) {
                Some(
                    self.type_name()
                        .ok_or_else(|| self.error("expected return type"))?,
                )
            } else {
                None
            };
            let body = self.block()?;
            return Ok(Stmt::Function {
                name,
                parameters,
                return_type,
                body,
            });
        }
        if self.take_word("return") {
            let value = if self.at(&Kind::Newline) {
                None
            } else {
                Some(self.expression()?)
            };
            self.expect(&Kind::Newline, "expected end of line")?;
            return Ok(Stmt::Return(value));
        }
        if matches!(&self.tokens[self.current].kind, Kind::Identifier(_)) {
            let op = match self.tokens.get(self.current + 1).map(|t| &t.kind) {
                Some(Kind::PlusAssign | Kind::Increment) => Some(TokenOp::Add),
                Some(Kind::MinusAssign | Kind::Decrement) => Some(TokenOp::Subtract),
                Some(Kind::StarAssign) => Some(TokenOp::Multiply),
                Some(Kind::SlashAssign) => Some(TokenOp::Divide),
                _ => None,
            };
            if let Some(op) = op {
                let name = self.name()?;
                let postfix = self.at(&Kind::Increment) || self.at(&Kind::Decrement);
                self.current += 1;
                let value = if postfix {
                    Expr::Integer(1)
                } else {
                    self.expression()?
                };
                self.expect(&Kind::Newline, "expected end of line")?;
                return Ok(Stmt::CompoundAssign { name, op, value });
            }
        }
        let typed = (self.word("int") || self.word("str") || self.word("bool"))
            && matches!(
                self.tokens.get(self.current + 1).map(|t| &t.kind),
                Some(Kind::Identifier(_))
            );
        let assignment = matches!(
            self.tokens.get(self.current + 1).map(|t| &t.kind),
            Some(Kind::Assign)
        );
        if typed || assignment {
            let declared_type = if typed { self.type_name() } else { None };
            let name = self.name()?;
            self.expect(&Kind::Assign, "expected '='")?;
            let value = self.expression()?;
            self.expect(&Kind::Newline, "expected end of line")?;
            return Ok(Stmt::Assign {
                name,
                declared_type,
                value,
            });
        }
        let value = self.expression()?;
        if self.take(&Kind::Assign) {
            let Expr::Located(inner, _) = value else {
                return Err(self.error("invalid assignment target"));
            };
            let Expr::Index(target, index) = *inner else {
                return Err(self.error("invalid assignment target"));
            };
            let value = self.expression()?;
            self.expect(&Kind::Newline, "expected end of line")?;
            return Ok(Stmt::IndexAssign {
                target: *target,
                index: *index,
                value,
            });
        }
        self.expect(&Kind::Newline, "expected end of line")?;
        Ok(Stmt::Expression(value))
    }
    fn parenthesized_condition(&mut self) -> Result<Expr, Diagnostic> {
        self.expect(&Kind::LeftParen, "expected '(' before condition")?;
        let condition = self.expression()?;
        self.expect(&Kind::RightParen, "expected ')' after condition")?;
        Ok(condition)
    }
    fn block(&mut self) -> Result<Vec<Stmt>, Diagnostic> {
        if self.take(&Kind::LeftBrace) {
            self.take(&Kind::Newline);
            self.take(&Kind::Indent);
            let mut body = Vec::new();
            while !self.at(&Kind::RightBrace) && !self.at(&Kind::Eof) {
                if self.take(&Kind::Newline) || self.take(&Kind::Dedent) {
                    continue;
                }
                body.push(self.statement()?);
            }
            self.expect(&Kind::RightBrace, "expected '}'")?;
            self.take(&Kind::Newline);
            return Ok(body);
        }
        self.expect(&Kind::Colon, "expected ':'")?;
        self.expect(&Kind::Newline, "expected end of line")?;
        self.expect(&Kind::Indent, "expected an indented block")?;
        let mut body = Vec::new();
        while !self.at(&Kind::Dedent) && !self.at(&Kind::Eof) {
            body.push(self.statement()?);
        }
        self.expect(&Kind::Dedent, "expected end of block")?;
        Ok(body)
    }
    fn expression(&mut self) -> Result<Expr, Diagnostic> {
        self.binary(0)
    }
    fn binary(&mut self, minimum: u8) -> Result<Expr, Diagnostic> {
        let mut left = self.unary()?;
        loop {
            let (op, precedence) = match self.tokens[self.current].kind {
                Kind::Equal => (TokenOp::Equal, 1),
                Kind::NotEqual => (TokenOp::NotEqual, 1),
                Kind::Less => (TokenOp::Less, 1),
                Kind::LessEqual => (TokenOp::LessEqual, 1),
                Kind::Greater => (TokenOp::Greater, 1),
                Kind::GreaterEqual => (TokenOp::GreaterEqual, 1),
                Kind::Plus => (TokenOp::Add, 2),
                Kind::Minus => (TokenOp::Subtract, 2),
                Kind::Star => (TokenOp::Multiply, 3),
                Kind::Slash => (TokenOp::Divide, 3),
                _ => break,
            };
            if precedence < minimum {
                break;
            }
            let span = self.span();
            self.current += 1;
            let right = self.binary(precedence + 1)?;
            left = Expr::Located(
                Box::new(Expr::Binary(Box::new(left), op, Box::new(right))),
                span,
            );
        }
        Ok(left)
    }
    fn unary(&mut self) -> Result<Expr, Diagnostic> {
        let span = self.span();
        self.unary_inner()
            .map(|expr| Expr::Located(Box::new(expr), span))
    }
    fn unary_inner(&mut self) -> Result<Expr, Diagnostic> {
        if self.take(&Kind::Minus) {
            return Ok(Expr::Unary(TokenOp::Subtract, Box::new(self.unary()?)));
        }
        if self.take_word("not") {
            return Ok(Expr::Unary(TokenOp::Not, Box::new(self.unary()?)));
        }
        let mut expr = self.primary()?;
        loop {
            if self.take(&Kind::LeftBracket) {
                let index = self.expression()?;
                self.expect(&Kind::RightBracket, "expected ']'")?;
                expr = Expr::Index(Box::new(expr), Box::new(index));
                continue;
            }
            if self.take(&Kind::Dot) {
                expr = Expr::Member(Box::new(expr), self.name()?);
                continue;
            }
            if !self.take(&Kind::LeftParen) {
                break;
            }
            let mut args = Vec::new();
            if !self.at(&Kind::RightParen) {
                loop {
                    args.push(self.expression()?);
                    if !self.take(&Kind::Comma) {
                        break;
                    }
                }
            }
            self.expect(&Kind::RightParen, "expected ')'")?;
            expr = Expr::Call(Box::new(expr), args);
        }
        Ok(expr)
    }
    fn primary(&mut self) -> Result<Expr, Diagnostic> {
        let result = match self.tokens[self.current].kind.clone() {
            Kind::Integer(n) => Expr::Integer(n),
            Kind::String(s) => Expr::String(s),
            Kind::Identifier(s) if s == "true" => Expr::Boolean(true),
            Kind::Identifier(s) if s == "false" => Expr::Boolean(false),
            Kind::Identifier(s) if s == "none" => Expr::None,
            Kind::Identifier(_) => return Ok(Expr::Variable(self.name()?)),
            Kind::LeftParen => {
                self.current += 1;
                let expr = self.expression()?;
                self.expect(&Kind::RightParen, "expected ')'")?;
                return Ok(expr);
            }
            Kind::LeftBracket => {
                self.current += 1;
                let mut values = Vec::new();
                while !self.at(&Kind::RightBracket) {
                    values.push(self.expression()?);
                    if !self.take(&Kind::Comma) {
                        break;
                    }
                }
                self.expect(&Kind::RightBracket, "expected ']'")?;
                return Ok(Expr::List(values));
            }
            Kind::LeftBrace => {
                self.current += 1;
                let mut entries = Vec::new();
                while self.take(&Kind::Newline)
                    || self.take(&Kind::Indent)
                    || self.take(&Kind::Dedent)
                {}
                while !self.at(&Kind::RightBrace) {
                    let key = self.expression()?;
                    self.expect(&Kind::Colon, "expected ':' after map key")?;
                    let value = self.expression()?;
                    entries.push((key, value));
                    let comma = self.take(&Kind::Comma);
                    while self.take(&Kind::Newline)
                        || self.take(&Kind::Indent)
                        || self.take(&Kind::Dedent)
                    {}
                    if !comma {
                        break;
                    }
                }
                self.expect(&Kind::RightBrace, "expected '}'")?;
                return Ok(Expr::Map(entries));
            }
            _ => return Err(self.error("expected an expression")),
        };
        self.current += 1;
        Ok(result)
    }
}
