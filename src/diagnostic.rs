//! Source locations and human-readable errors used throughout the pipeline.

use std::fmt;

#[derive(Debug, Clone)]
/// A source range and its original line, retained for error display.
// ============================================================
// SOURCE LOCATIONS
//
// Record the filename, position, and original line for a diagnostic.
// ============================================================
pub struct Span {
    pub filename: String,
    pub line: usize,
    pub column: usize,
    pub length: usize,
    pub source_line: String,
}

#[derive(Debug, Clone)]
/// An error code and message with optional corrective context.
// ============================================================
// DIAGNOSTICS
//
// Carry an error code, message, source location, and optional help.
// ============================================================
pub struct Diagnostic {
    pub code: &'static str,
    pub message: String,
    pub span: Box<Span>,
    pub help: Option<String>,
    pub note: Option<String>,
}

impl Diagnostic {
    pub fn new(code: &'static str, message: impl Into<String>, span: Span) -> Self {
        Self {
            code,
            message: message.into(),
            span: Box::new(span),
            help: None,
            note: None,
        }
    }
}

// ============================================================
// DIAGNOSTIC DISPLAY
//
// Render the source line and underline the relevant position.
// ============================================================
impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let span = &self.span;
        let width = span.line.to_string().len();
        writeln!(f, "error[{}]: {}", self.code, self.message)?;
        writeln!(f, " --> {}:{}:{}", span.filename, span.line, span.column)?;
        writeln!(f, " {:width$} |", "")?;
        writeln!(f, " {} | {}", span.line, span.source_line)?;
        write!(
            f,
            " {:width$} | {}{}",
            "",
            " ".repeat(span.column.saturating_sub(1)),
            "^".repeat(span.length.max(1))
        )?;
        if let Some(help) = &self.help {
            write!(f, "\n {:width$} = help: {}", "", help)?;
        }
        if let Some(note) = &self.note {
            write!(f, "\n {:width$} = note: {}", "", note)?;
        }
        Ok(())
    }
}
