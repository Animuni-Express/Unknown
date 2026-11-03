mod ast;
mod diagnostic;
mod interpreter;
mod lexer;
mod parser;
mod value;

use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    let (source, filename) = match args.as_slice() {
        [_, command, file] if command == "run" => (
            std::fs::read_to_string(file).map_err(|e| format!("{file}: {e}"))?,
            file.as_str(),
        ),
        [_, command, code] if command == "-c" => (code.clone(), "<command>"),
        _ => return Err("usage: unk run <file.unk> | unk -c <code>".into()),
    };
    let tokens = lexer::lex(&source, filename).map_err(|e| e.to_string())?;
    let program = parser::parse(tokens).map_err(|e| e.to_string())?;
    interpreter::Interpreter::new(&mut std::io::stdout().lock())
        .run(&program)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn execute(source: &str) -> Result<String, String> {
        let tokens = lexer::lex(source, "test.unk").map_err(|e| e.to_string())?;
        let program = parser::parse(tokens).map_err(|e| e.to_string())?;
        let mut output = Vec::new();
        interpreter::Interpreter::new(&mut output)
            .run(&program)
            .map_err(|e| e.to_string())?;
        Ok(String::from_utf8(output).unwrap())
    }
    #[test]
    fn lexer_tracks_positions_and_indentation() {
        let tokens = lexer::lex("if (true):\n    print(\"hi\")\n", "test.unk").unwrap();
        let print = tokens
            .iter()
            .find(|t| t.kind == lexer::Kind::Identifier("print".into()))
            .unwrap();
        assert_eq!(
            (&*print.source, print.line, print.column),
            ("test.unk", 2, 5)
        );
        assert!(tokens.iter().any(|t| t.kind == lexer::Kind::Indent));
        assert!(tokens.iter().any(|t| t.kind == lexer::Kind::Dedent));
        assert!(lexer::lex("if (true):\n  x = 1\n x = 2\n", "test.unk").is_err());
    }
    #[test]
    fn parser_rejects_missing_block_and_reports_location() {
        let error =
            parser::parse(lexer::lex("if (true):\nprint(1)\n", "test.unk").unwrap()).unwrap_err();
        assert!(error.to_string().contains("test.unk:2:1"));
    }
    #[test]
    fn hello_and_arithmetic() {
        assert_eq!(
            execute("print(\"Hello, Unknown!\")\nint n = 2 + 3 * 4\nprint(n, (n - 2) / 3)\n")
                .unwrap(),
            "Hello, Unknown!\n14 4\n"
        );
    }
    #[test]
    fn compound_and_postfix_assignment() {
        assert_eq!(
            execute("int x = 5\nx += 2\nx++\nx *= 2\nx--\nx /= 3\nx -= 1\nprint(x)\n").unwrap(),
            "4\n"
        );
        assert!(execute("missing++\n")
            .unwrap_err()
            .contains("undefined name"));
        assert!(execute("x = 1\nx /= 0\n")
            .unwrap_err()
            .contains("division by zero"));
        assert!(execute("x = 1\ny = x++\n").is_err());
    }
    #[test]
    fn lists_ranges_and_scopes() {
        let source = "numbers = []\nfor (i in range(1, 11)):\n    numbers.append(i)\nint total = 0\nfor (n in numbers):\n    total += n\nnumbers[0] = 99\nprint(total, numbers[0], len(numbers), str(true))\n";
        assert_eq!(execute(source).unwrap(), "55 99 10 true\n");
        assert_eq!(
            execute("for (i in range(3, 0, -1)):\n    print(i)\n").unwrap(),
            "3\n2\n1\n"
        );
        for source in [
            "for (i in range(1)):\n    print(i)\nprint(i)\n",
            "a = [1]\nprint(a[1])\n",
            "a = []\na[\"x\"] = 1\n",
            "print(range(1, 2, 0))\n",
            "if (true):\n    int y = 1\nprint(y)\n",
        ] {
            assert!(execute(source).is_err(), "expected error for {source}");
        }
    }
    #[test]
    fn native_file_and_process_modules() {
        let path = std::env::temp_dir().join(format!(
            "unknown-test-{}-{}.tmp",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let path = path.to_string_lossy().replace('\\', "/");
        let source = format!("import file\nfile.write(\"{path}\", \"abc\")\nfile.append(\"{path}\", \"d\")\nprint(file.exists(\"{path}\"), file.read(\"{path}\"))\nfile.delete(\"{path}\")\n");
        assert_eq!(execute(&source).unwrap(), "true abcd\n");
        let source = format!("f = open(\"{path}\", \"w\")\nf.write(\"hello\")\nf.close()\nf = open(\"{path}\", \"r\")\nprint(f.read())\nf.close()\nimport file\nfile.delete(\"{path}\")\n");
        assert_eq!(execute(&source).unwrap(), "hello\n");
        let source = format!("import file\nnumbers = []\nfor (i in range(1, 11)):\n    numbers.append(i)\nint total = 0\nfor (n in numbers):\n    total += n\nfile.write(\"{path}\", \"Total: \" + str(total))\nprint(\"Total:\", total)\n");
        assert_eq!(execute(&source).unwrap(), "Total: 55\n");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "Total: 55");
        std::fs::remove_file(&path).unwrap();
        assert!(execute("f = open(\"ignored\", \"invalid\")\n")
            .unwrap_err()
            .contains("open mode"));
        let source = format!("f = open(\"{path}\", \"w\")\nf.close()\nf.write(\"x\")\n");
        assert!(execute(&source).unwrap_err().contains("file is closed"));
        std::fs::remove_file(&path).unwrap();
        assert!(execute("import missing\n").unwrap_err().contains("E4001"));
        assert!(execute("import process\nprocess.run(\"x\", [1])\n")
            .unwrap_err()
            .contains("arguments must be strings"));
        let executable = std::env::current_exe()
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        let source = format!(
            "import process\nr = process.run(\"{executable}\", [\"--list\"])\nprint(r.code)\n"
        );
        assert_eq!(execute(&source).unwrap(), "0\n");
    }
    #[test]
    fn maps_and_brace_blocks() {
        let source = "m = {\n    \"x\": 10,\n    \"enabled\": true\n}\nm[\"x\"] = 20\nif (true) {\n    print(m[\"x\"], len(m))\n}\n";
        assert_eq!(execute(source).unwrap(), "20 2\n");
        assert_eq!(execute("for (i in range(2)) {\n    if (i == 0) {\n        print(i)\n    } else {\n        print(9)\n    }\n}\n").unwrap(), "0\n9\n");
        assert!(execute("m = {}\nprint(m[\"missing\"])\n")
            .unwrap_err()
            .contains("not found"));
        assert!(execute("m = {1: 2}\n")
            .unwrap_err()
            .contains("map key must be a string"));
    }
    #[test]
    fn syntax_comments_and_function_returns() {
        let source = "// comment\nfn add(int a, int b) -> int {\n    /* multiline\n       comment */\n    return a + b // tail\n}\nif (false):\n    print(0)\nelse if (true):\n    print(add(2, 3), \"// not a comment\")\n";
        assert_eq!(execute(source).unwrap(), "5 // not a comment\n");
        assert!(execute("if true:\n    print(1)\n").is_err());
        assert!(execute("while true:\n    print(1)\n").is_err());
        assert!(execute("/* unfinished")
            .unwrap_err()
            .contains("unterminated block comment"));
        assert!(execute("fn f() -> int:\n    return true\nf()\n")
            .unwrap_err()
            .contains("must return int"));
        assert_eq!(
            execute("int x = 1\nfn f():\n    x = 2\nf()\nprint(x)\n").unwrap(),
            "1\n"
        );
    }
    #[test]
    fn variables_conditions_functions_and_loops() {
        let source = "bool ready = true\nstr name = \"Unknown\"\nfn greet(str who):\n    if (ready):\n        print(\"Hello\", who)\ngreet(name)\nfn sum(int n):\n    total = 0\n    while (n > 0):\n        total = total + n\n        n = n - 1\n    return total\nprint(sum(4), false == true, none)\n";
        assert_eq!(execute(source).unwrap(), "Hello Unknown\n10 false none\n");
    }
    #[test]
    fn errors_are_reported() {
        for source in [
            "print(missing)\n",
            "print(1 / 0)\n",
            "int n = true\n",
            "return 1\n",
            "if (1):\n    print(1)\n",
            "fn f(int x):\n    return x\nf(false)\n",
        ] {
            assert!(execute(source).is_err(), "expected error for {source}");
        }
    }
    #[test]
    fn diagnostics_include_source_and_suggestions() {
        let error = execute("total = 1\nprint(totla)\n").unwrap_err();
        assert!(error.contains("error[E2001]: undefined name 'totla'"));
        assert!(error.contains("test.unk:2:7"));
        assert!(error.contains("print(totla)"));
        assert!(error.contains("^^^^^"));
        assert!(error.contains("did you mean 'total'?"));

        let error = execute("print(1 / 0)\n").unwrap_err();
        assert!(error.contains("error[E2004]: division by zero"));
        assert!(error.contains("test.unk:1:9"));

        let error = execute("if (true):\n  print(1)\n print(2)\n").unwrap_err();
        assert!(error.contains("error[E0002]: inconsistent indentation"));
        assert!(error.contains("test.unk:3:1"));
    }

    #[test]
    fn runtime_errors_keep_function_source_locations() {
        let error = execute("fn fail():\n    print(missing)\nfail()\n").unwrap_err();
        assert!(error.contains("test.unk:2:11"));
        assert!(error.contains("print(missing)"));
    }
}
