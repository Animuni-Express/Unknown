use crate::diagnostic::{Diagnostic, Span};

#[derive(Debug, Clone, PartialEq)]
pub enum Kind {
    Identifier(String),
    Integer(i64),
    String(String),
    LeftParen,
    RightParen,
    LeftBracket,
    RightBracket,
    LeftBrace,
    RightBrace,
    Dot,
    Comma,
    Colon,
    Plus,
    Minus,
    Star,
    Slash,
    PlusAssign,
    MinusAssign,
    StarAssign,
    SlashAssign,
    Increment,
    Decrement,
    Arrow,
    Assign,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Newline,
    Indent,
    Dedent,
    Eof,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: Kind,
    pub line: usize,
    pub column: usize,
    pub source: String,
    pub source_line: String,
    pub length: usize,
}

pub fn lex(source: &str, filename: &str) -> Result<Vec<Token>, Diagnostic> {
    let span = |line: usize, column: usize, length: usize| Span {
        filename: filename.to_owned(),
        line,
        column,
        length,
        source_line: source
            .lines()
            .nth(line.saturating_sub(1))
            .unwrap_or("")
            .to_owned(),
    };
    let mut tokens = Vec::new();
    let mut indents = vec![0];
    let chars: Vec<char> = source.chars().collect();
    let mut cleaned = String::new();
    let (mut i, mut line, mut column) = (0, 1, 1);
    let mut quote = None;
    let mut block = None;
    let mut line_comment = false;
    while i < chars.len() {
        let ch = chars[i];
        let next = chars.get(i + 1).copied();
        if ch == '\n' {
            cleaned.push(ch);
            line += 1;
            column = 1;
            line_comment = false;
            quote = None;
        } else if line_comment {
            cleaned.push(' ');
            column += 1;
        } else if block.is_some() {
            cleaned.push(' ');
            column += 1;
            if ch == '*' && next == Some('/') {
                cleaned.push(' ');
                i += 1;
                column += 1;
                block = None;
            }
        } else if let Some(delimiter) = quote {
            cleaned.push(ch);
            column += 1;
            if ch == '\\' && next.is_some() && next != Some('\n') {
                cleaned.push(next.expect("checked above"));
                i += 1;
                column += 1;
            } else if ch == delimiter {
                quote = None;
            }
        } else if ch == '/' && matches!(next, Some('/') | Some('*')) {
            if next == Some('/') {
                line_comment = true;
            } else {
                block = Some((line, column));
            }
            cleaned.push_str("  ");
            i += 1;
            column += 2;
        } else {
            if ch == '"' || ch == '\'' {
                quote = Some(ch);
            }
            cleaned.push(ch);
            column += 1;
        }
        i += 1;
    }
    if let Some((line, column)) = block {
        return Err(Diagnostic::new(
            "E0007",
            "unterminated block comment",
            span(line, column, 2),
        ));
    }
    for (line_index, line) in cleaned.lines().enumerate() {
        let line_number = line_index + 1;
        let chars: Vec<char> = line.chars().collect();
        let mut i = 0;
        while i < chars.len() && chars[i] == ' ' {
            i += 1;
        }
        if i == chars.len() {
            continue;
        }
        if chars[i] == '\t' {
            return Err(Diagnostic::new(
                "E0001",
                "tabs are not supported for indentation",
                span(line_number, i + 1, 1),
            ));
        }
        let mut push = |kind, column, length| {
            tokens.push(Token {
                kind,
                line: line_number,
                column,
                source: filename.to_owned(),
                source_line: source.lines().nth(line_index).unwrap_or("").to_owned(),
                length,
            })
        };
        if i > *indents.last().unwrap() {
            indents.push(i);
            push(Kind::Indent, 1, 1);
        } else if i < *indents.last().unwrap() {
            while i < *indents.last().unwrap() {
                indents.pop();
                push(Kind::Dedent, 1, 1);
            }
            if i != *indents.last().unwrap() {
                return Err(Diagnostic::new(
                    "E0002",
                    "inconsistent indentation",
                    span(line_number, 1, i.max(1)),
                ));
            }
        }
        while i < chars.len() {
            if chars[i].is_whitespace() {
                i += 1;
                continue;
            }
            let start = i;
            let ch = chars[i];
            i += 1;
            let kind = match ch {
                '(' => Kind::LeftParen,
                ')' => Kind::RightParen,
                '[' => Kind::LeftBracket,
                ']' => Kind::RightBracket,
                '{' => Kind::LeftBrace,
                '}' => Kind::RightBrace,
                '.' => Kind::Dot,
                ',' => Kind::Comma,
                ':' => Kind::Colon,
                '+' | '-' | '*' | '/' => {
                    let next = chars.get(i).copied();
                    let kind = match (ch, next) {
                        ('+', Some('+')) => Kind::Increment,
                        ('-', Some('-')) => Kind::Decrement,
                        ('-', Some('>')) => Kind::Arrow,
                        ('+', Some('=')) => Kind::PlusAssign,
                        ('-', Some('=')) => Kind::MinusAssign,
                        ('*', Some('=')) => Kind::StarAssign,
                        ('/', Some('=')) => Kind::SlashAssign,
                        ('+', _) => Kind::Plus,
                        ('-', _) => Kind::Minus,
                        ('*', _) => Kind::Star,
                        _ => Kind::Slash,
                    };
                    if matches!(
                        kind,
                        Kind::Increment
                            | Kind::Decrement
                            | Kind::Arrow
                            | Kind::PlusAssign
                            | Kind::MinusAssign
                            | Kind::StarAssign
                            | Kind::SlashAssign
                    ) {
                        i += 1;
                    }
                    kind
                }
                '=' | '!' | '<' | '>' => {
                    let equals = i < chars.len() && chars[i] == '=';
                    if equals {
                        i += 1;
                    }
                    match (ch, equals) {
                        ('=', false) => Kind::Assign,
                        ('=', true) => Kind::Equal,
                        ('!', true) => Kind::NotEqual,
                        ('<', false) => Kind::Less,
                        ('<', true) => Kind::LessEqual,
                        ('>', false) => Kind::Greater,
                        ('>', true) => Kind::GreaterEqual,
                        _ => {
                            return Err(Diagnostic::new(
                                "E0003",
                                "unexpected '!'",
                                span(line_number, start + 1, 1),
                            ))
                        }
                    }
                }
                '"' | '\'' => {
                    let mut value = String::new();
                    let mut closed = false;
                    while i < chars.len() {
                        let c = chars[i];
                        i += 1;
                        if c == ch {
                            closed = true;
                            break;
                        }
                        if c == '\\' {
                            if i == chars.len() {
                                break;
                            }
                            let escaped = chars[i];
                            i += 1;
                            value.push(match escaped {
                                'n' => '\n',
                                't' => '\t',
                                'r' => '\r',
                                '\\' => '\\',
                                '\'' => '\'',
                                '"' => '"',
                                _ => {
                                    return Err(Diagnostic::new(
                                        "E0004",
                                        "unknown escape sequence",
                                        span(line_number, i.saturating_sub(1), 2),
                                    ))
                                }
                            });
                        } else {
                            value.push(c);
                        }
                    }
                    if !closed {
                        return Err(Diagnostic::new(
                            "E0005",
                            "unterminated string",
                            span(line_number, start + 1, i - start),
                        ));
                    }
                    Kind::String(value)
                }
                c if c.is_ascii_digit() => {
                    while i < chars.len() && chars[i].is_ascii_digit() {
                        i += 1;
                    }
                    let text: String = chars[start..i].iter().collect();
                    Kind::Integer(text.parse().map_err(|_| {
                        Diagnostic::new(
                            "E0006",
                            "integer out of range",
                            span(line_number, start + 1, i - start),
                        )
                    })?)
                }
                c if c.is_alphabetic() || c == '_' => {
                    while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                        i += 1;
                    }
                    Kind::Identifier(chars[start..i].iter().collect())
                }
                _ => {
                    return Err(Diagnostic::new(
                        "E0003",
                        format!("unexpected character '{ch}'"),
                        span(line_number, start + 1, 1),
                    ))
                }
            };
            push(kind, start + 1, i - start);
        }
        push(Kind::Newline, chars.len() + 1, 1);
    }
    let line = source.lines().count() + 1;
    while indents.len() > 1 {
        indents.pop();
        tokens.push(Token {
            kind: Kind::Dedent,
            line,
            column: 1,
            source: filename.to_owned(),
            source_line: String::new(),
            length: 1,
        });
    }
    tokens.push(Token {
        kind: Kind::Eof,
        line,
        column: 1,
        source: filename.to_owned(),
        source_line: String::new(),
        length: 1,
    });
    Ok(tokens)
}
