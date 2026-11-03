use std::process::Command;

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
