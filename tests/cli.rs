//! Integration tests for the executable and its command-line behavior.

use std::process::Command;

// Invoke the built executable with the requested arguments.
// ============================================================
// COMMAND-LINE INTEGRATION TESTS
//
// Run the built executable and inspect its output and exit status.
// ============================================================
fn unk(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_unk"))
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn executes_inline_and_file_code() {
    let inline = unk(&["-c", "print(2 + 3)"]);
    assert!(inline.status.success());
    assert_eq!(inline.stdout, b"5\n");

    let file = unk(&["run", "examples/hello.unk"]);
    assert!(file.status.success());
    assert_eq!(file.stdout, b"Hello, Unknown!\n");
}

#[test]
fn inline_errors_and_invalid_arguments_fail() {
    let error = unk(&["-c", "print(missing)"]);
    assert!(!error.status.success());
    assert!(String::from_utf8_lossy(&error.stderr).contains("<command>:1:7"));

    for args in [&["-c"][..], &["-c", "print(1)", "extra"][..]] {
        let error = unk(args);
        assert!(!error.status.success());
        assert!(String::from_utf8_lossy(&error.stderr).contains("usage: unk"));
    }
}

#[test]
fn examples_run_successfully() {
    for name in [
        "arithmetic",
        "brace_blocks",
        "collections",
        "conditions",
        "fibonacci",
        "files",
        "functions",
        "hello",
        "loops",
        "variables",
    ] {
        let output = unk(&["run", &format!("examples/{name}.unk")]);
        assert!(
            output.status.success(),
            "{name}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let module_example = unk(&["run", "examples/modules/main.unk"]);
    assert!(
        module_example.status.success(),
        "{}",
        String::from_utf8_lossy(&module_example.stderr)
    );
    assert_eq!(module_example.stdout, b"Hello, Unknown!\n");
    if Command::new("git").arg("--version").output().is_ok() {
        let output = unk(&["run", "examples/process.unk"]);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn user_modules_are_cached_and_keep_their_own_globals() {
    let output = unk(&["run", "tests/fixtures/modules/program.unk"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        output.stdout,
        b"loading math_tools\ninitialized: 4\n14 3 4 900 99\n"
    );
}

#[test]
fn module_errors_report_the_source_and_cycles() {
    let cycle = unk(&["run", "tests/fixtures/modules/cycle_a.unk"]);
    assert!(!cycle.status.success());
    assert!(String::from_utf8_lossy(&cycle.stderr).contains("circular import"));
    let broken = unk(&["-c", "import \"tests_does_not_exist\""]);
    assert!(!broken.status.success());
    assert!(String::from_utf8_lossy(&broken.stderr).contains("E4001"));
    let broken = unk(&["run", "tests/fixtures/modules/broken_program.unk"]);
    assert!(!broken.status.success());
    let error = String::from_utf8_lossy(&broken.stderr);
    assert!(error.contains("broken.unk:1:7"), "{error}");
    assert!(error.contains("undefined name 'missing'"), "{error}");
    let isolation = unk(&["run", "tests/fixtures/modules/isolation.unk"]);
    assert!(!isolation.status.success());
    assert!(String::from_utf8_lossy(&isolation.stderr).contains("undefined name 'outside'"));
}

#[test]
fn imports_require_quoted_names_and_aliases() {
    let output = unk(&[
        "-c",
        "import \"process\" as \"proc\"\nprint(proc.run(\"missing-unknown-command\", []))",
    ]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("process.run"));
    for source in ["import process", "import \"process\" as proc"] {
        let output = unk(&["-c", source]);
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("expected quoted"));
    }
    let output = unk(&[
        "-c",
        "import \"file\" as \"f\"\nprint(f.exists(\"Cargo.toml\"))",
    ]);
    assert!(output.status.success());
    assert_eq!(output.stdout, b"true\n");
}

#[test]
fn system_modules_and_loop_control_execute() {
    let source = r#"import "envar"
import "path"
import "process"
import "system"
import "time"
import "unknown-shell" as "ush"
envar.set("UNKNOWN_TEST_VARIABLE", "hello")
print(envar.exists("UNKNOWN_TEST_VARIABLE"), envar.get("UNKNOWN_TEST_VARIABLE"), envar.list()["UNKNOWN_TEST_VARIABLE"])
envar.unset("UNKNOWN_TEST_VARIABLE")
print(envar.get("UNKNOWN_TEST_VARIABLE"), envar.exists("UNKNOWN_TEST_VARIABLE"))
print(path.filename(path.join("a", "b", "file.txt")), path.extension("file.txt"))
print(path.parent("a/b"), path.is_file("Cargo.toml"), path.is_dir("src"), path.exists("Cargo.toml"))
print(path.exists(path.absolute("Cargo.toml")))
print(process.pid() > 0, len(process.cwd()) > 0, len(system.os()) > 0, len(system.arch()) > 0, len(system.hostname()) > 0, system.cpu_count() > 0)
print(time.now() > 0)
start = time.monotonic()
time.sleep(1)
print(time.monotonic() >= start)
ush.write("A")
ush.write_line("B")
for (i in range(3)) {
    for (j in range(4)) {
        if (j == 1) {
            continue
        }
        if (j == 3) {
            break
        }
        print(i, j)
    }
}
"#;
    let output = unk(&["-c", source]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "true hello hello\nnone false\nfile.txt txt\na true true true\ntrue\ntrue true true true true true\ntrue\ntrue\nAB\n0 0\n0 2\n1 0\n1 2\n2 0\n2 2\n");
}

#[test]
fn loop_control_and_program_arguments() {
    for source in [
        "break",
        "continue",
        "fn f() {\n break\n}\nwhile (true) {\n f()\n}",
    ] {
        let output = unk(&["-c", source]);
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("outside a loop"));
    }
    let output = unk(&["run", "tests/fixtures/args.unk", "hello", "world"]);
    assert!(output.status.success());
    assert_eq!(output.stdout, b"[hello, world]\n");
    let output = unk(&[
        "-c",
        "fn f() {\n for (i in range(2)) {\n return i\n }\n}\nprint(f())",
    ]);
    assert!(output.status.success());
    assert_eq!(output.stdout, b"0\n");
}

#[test]
fn shell_input_and_process_directory() {
    use std::io::Write;
    use std::process::Stdio;
    let mut child = Command::new(env!("CARGO_BIN_EXE_unk"))
        .args(["-c", "import \"unknown-shell\" as \"ush\"\nush.write(\"Name: \")\nprint(ush.read_line())\nprint(ush.is_tty())"])
        .stdin(Stdio::piped()).stdout(Stdio::piped()).spawn().unwrap();
    child.stdin.take().unwrap().write_all(b"Ada\r\n").unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    assert_eq!(output.stdout, b"Name: Ada\nfalse\n");
    let output = unk(&["-c", "import \"process\"\nimport \"path\"\nprocess.chdir(\"examples\")\nprint(path.filename(process.cwd()))"]);
    assert!(output.status.success());
    assert_eq!(output.stdout, b"examples\n");
}

#[test]
fn while_control_and_terminal_clear() {
    let source = "import \"unknown-shell\" as \"ush\"\ni = 0\nwhile (i < 5) {\n i++\n if (i == 2) {\n  continue\n }\n if (i == 4) {\n  break\n }\n print(i)\n}\nush.clear()";
    let output = unk(&["-c", source]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"1\n3\n\x1b[2J\x1b[3J\x1b[H");
    for source in [
        "import \"envar\"\nenvar.set(\"BAD=NAME\", \"x\")",
        "import \"time\"\ntime.sleep(-1)",
    ] {
        assert!(!unk(&["-c", source]).status.success());
    }
}
