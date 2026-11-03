# Unknown

Unknown is a small, independent tree-walking interpreter written in Rust. Its frontend consists of a lexer, parser, and interpreter-independent AST.

## Run

Install Rust and run the included example:

```text
cargo run -- run examples/hello.unk
```

To use the `unk` command directly, build with `cargo build` and run `target/debug/unk run examples/hello.unk` (on Windows, use `target\debug\unk.exe`).

Run a snippet directly with `unk -c 'print("Hello")'` (the quoting shown works in PowerShell and Unix shells). Diagnostics identify inline code as `<command>`.

## Current language

The interpreter supports integers, strings, booleans, lists, string-keyed maps, variables, arithmetic, comparisons, `if`/`else if`/`else`, `while`, `for`, functions, and `return`. Blocks can use spaces for indentation or braces; conditions require parentheses. Variables can be inferred (`name = "Unknown"`) or declared with `int`, `str`, or `bool`. Declared types are checked on assignment, function arguments and annotated return values. Comments use `//` or `/* ... */`.

```text
numbers = []
for (i in range(1, 4)):
    numbers.append(i)
print(numbers[0], len(numbers))
```

`range` accepts an exclusive end and an optional start and step. Lists support `append`, indexed reads and writes, and iteration. Maps support string-keyed reads and writes. Containers are shared on assignment; list iteration uses a snapshot. `len` accepts strings, lists, and maps; `str` converts values to text. Compound assignments and standalone postfix `++` and `--` operate on variables.

`import file` provides `read`, `write`, `append`, `exists`, and `delete`; `open(path, mode)` provides file handles with `read`, `write`, and `close` in modes `r`, `w`, and `a`. `import process` provides `process.run(executable, arguments)`, returning `stdout`, `stderr`, and `code`. Process output must be UTF-8. These operations access the local machine with the interpreter's permissions.

Conditions require booleans; integer division truncates toward zero. Functions can access their own locals and globals, but do not capture enclosing function locals. Loop variables and block declarations are local to their blocks. Imports currently cover built-in modules only.

Run the tests with `cargo test`.

The `examples` directory also contains runnable programs for collections,
Fibonacci numbers, brace blocks, file operations, and process execution. Run
one with, for example, `cargo run -- run examples/collections.unk`. The process
example requires Git on `PATH`. The file example reads `README.md` when run
from the project root.

## Diagnostics

Lexer, parser, and runtime errors report an error code, filename, line and column,
source excerpt, and a marker. Undefined names may include a spelling suggestion.
Columns and marker widths count Unicode characters. Runtime checks still occur
during execution; Unknown does not yet have a separate semantic checker.
