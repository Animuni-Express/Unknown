use serde::{Deserialize, Serialize};
use std::io::{self, Write};
use ush_core::vfs;
use wasm_bindgen::prelude::*;

const MAX_SOURCE: usize = 65_536;
const MAX_OUTPUT: usize = 65_536;
#[derive(Default)]
struct Output(Vec<u8>);
impl Write for Output {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > MAX_OUTPUT.saturating_sub(self.0.len()) {
            return Err(io::Error::other("output limit reached (64 KiB)"));
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Diagnostic {
    severity: &'static str,
    code: String,
    message: String,
    line: usize,
    column: usize,
    end_line: usize,
    end_column: usize,
    help: Option<String>,
    filename: String,
}
#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct ResultData {
    stdout: String,
    stderr: String,
    diagnostics: Vec<Diagnostic>,
    exit_code: i32,
    files: Vec<(String, Option<String>)>,
    cwd: String,
}
#[derive(Deserialize)]
struct Request {
    runtime: String,
    source: String,
    filename: String,
    files: Vec<(String, Option<String>)>,
    cwd: String,
}

fn restore(files: Vec<(String, Option<String>)>, cwd: &str) -> Result<(), String> {
    if files.len() > 256 {
        return Err("too many virtual files".into());
    }
    let canonical = |path: &str| {
        path.starts_with('/')
            && path.len() <= 512
            && !path.contains(['\\', ':', '\0'])
            && (path == "/"
                || path[1..]
                    .split('/')
                    .all(|part| !part.is_empty() && part != "." && part != ".."))
    };
    let mut paths = std::collections::HashSet::new();
    let mut bytes = 0usize;
    for (path, data) in &files {
        if !canonical(path) || !paths.insert(path) {
            return Err("workspace contains noncanonical or duplicate paths".into());
        }
        bytes = bytes.saturating_add(data.as_ref().map_or(0, String::len));
    }
    if bytes > 1_048_576 {
        return Err("virtual filesystem exceeds 1 MiB".into());
    }
    if !canonical(cwd) || !files.iter().any(|(p, v)| p == cwd && v.is_none()) {
        return Err("working directory does not exist".into());
    }
    vfs::reset();
    let mut files = files;
    files.sort_by_key(|(p, _)| p.len());
    for (path, data) in files {
        if path == "/" && data.is_none() {
            continue;
        }
        match data {
            Some(data) => vfs::write(&path, data),
            None => vfs::create_dir(&path),
        }
        .map_err(|e| format!("{path}: {e}"))?;
    }
    vfs::set_current_dir(cwd).map_err(|e| e.to_string())
}
fn execute(request: Request) -> ResultData {
    let mut result = ResultData::default();
    let mut output = Output::default();
    let original_files = request.files.clone();
    let mut restored = false;
    let operation = (|| -> Result<i32, String> {
        if request.source.len() > MAX_SOURCE {
            return Err("source exceeds 64 KiB".into());
        }
        restore(request.files, &request.cwd)?;
        restored = true;
        match request.runtime.as_str() {
            "unk" => {
                let execution = unknown::lexer::lex(&request.source, &request.filename)
                    .and_then(unknown::parser::parse)
                    .and_then(|program| {
                        unknown::interpreter::Interpreter::new(&mut output).run(&program)
                    });
                match execution {
                    Ok(()) => Ok(0),
                    Err(error) => {
                        result.diagnostics.push(Diagnostic {
                            severity: "error",
                            code: error.code.into(),
                            message: error.message.clone(),
                            line: error.span.line,
                            column: error.span.column,
                            end_line: error.span.line,
                            end_column: error.span.column + error.span.length.max(1),
                            help: error.help.clone(),
                            filename: error.span.filename.clone(),
                        });
                        Err(error.to_string())
                    }
                }
            }
            "ush" => {
                for line in request
                    .source
                    .lines()
                    .filter(|line| !line.trim().is_empty())
                {
                    let args = shell_words::split(line).map_err(|e| e.to_string())?;
                    let code = ush_core::execute_to(&args, &mut output)?;
                    if code == -2 {
                        return Err("delete requires --force in a noninteractive playground".into());
                    }
                    if code == -1 {
                        break;
                    }
                    if code != 0 {
                        return Ok(code);
                    }
                }
                Ok(0)
            }
            _ => Err("unknown runtime".into()),
        }
    })();
    match operation {
        Ok(code) => result.exit_code = code,
        Err(error) => {
            result.exit_code = 1;
            result.stderr = error;
        }
    }
    result.stdout = String::from_utf8_lossy(&output.0).into_owned();
    if restored {
        result.files = vfs::snapshot();
        result.cwd = vfs::current_dir()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
    } else {
        result.files = original_files;
        result.cwd = request.cwd;
    }
    result
}

/// JSON keeps the boundary explicit and validates all caller-supplied state.
#[wasm_bindgen]
pub fn run(request: &str) -> Result<String, JsValue> {
    if request.len() > 7_000_000 {
        return Err(JsValue::from_str("request exceeds size limit"));
    }
    let request: Request =
        serde_json::from_str(request).map_err(|e| JsValue::from_str(&e.to_string()))?;
    serde_json::to_string(&execute(request)).map_err(|e| JsValue::from_str(&e.to_string()))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounded_output_and_source_validation() {
        let mut output = Output::default();
        assert!(output.write_all(&vec![b'x'; MAX_OUTPUT]).is_ok());
        assert!(output.write_all(b"x").is_err());
        let result = execute(Request {
            runtime: "unk".into(),
            source: "x".repeat(MAX_SOURCE + 1),
            filename: "/main.unk".into(),
            files: vec![],
            cwd: "/".into(),
        });
        assert_eq!(result.exit_code, 1);
        assert!(result.stderr.contains("64 KiB"));
    }
    #[test]
    fn failed_restore_preserves_every_input_file() {
        let files = vec![
            ("/".into(), None),
            ("/notes.txt".into(), Some("keep me".into())),
            ("//notes.txt".into(), Some("alias".into())),
        ];
        let result = execute(Request {
            runtime: "unk".into(),
            source: "print(1)".into(),
            filename: "/main.unk".into(),
            files: files.clone(),
            cwd: "/".into(),
        });
        assert_eq!(result.exit_code, 1);
        assert_eq!(result.files, files);
    }
}
