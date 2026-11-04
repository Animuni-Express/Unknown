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

`import "file"` provides `read`, `write`, `append`, `exists`, and `delete`; `open(path, mode)` provides file handles with `read`, `write`, and `close` in modes `r`, `w`, and `a`. `import "process"` provides `process.run(executable, arguments)`, returning `stdout`, `stderr`, and `code`. Process output must be UTF-8. These operations access the local machine with the interpreter's permissions.

Conditions require booleans; integer division truncates toward zero. Functions can access their own locals and globals, but do not capture enclosing function locals. Loop variables and block declarations are local to their blocks.

`import "greeter"` also loads `greeter.unk` beside the importing source file; use
members as `greeter.greet("Unknown")`. For `-c` code, imports resolve from the
current directory. An alias also uses a string, for example `import "process" as "proc"`. Built-in modules take precedence. A user module executes
once per interpreter and is cached; circular imports produce an error. Module
functions can read their module's globals. See `examples/modules/main.unk`.

Run the tests with `cargo test`.

## Unknown Trying website

The shared interpreter library also powers the browser documentation and playground
in [`website/`](website/README.md). Unknown and the integrated USH Rust core execute
through WebAssembly in a worker and share a virtual filesystem. The same static
Next.js codebase targets ChatGPT Sites and GitHub Pages; see the website README for
development, independent host builds, verification, deployment, and browser limits.

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

## System modules

All imports use quoted names; `import "unknown-shell" as "ush"` binds the shell module as `ush`.

- `envar.get(name)`, `exists(name)`, `set(name, value)`, `unset(name)`, and `list()` operate **only on the current process environment**. Missing `get` values return `none`, and `list` returns a string-keyed map. Names are trimmed and rejected if empty or containing `=` or NUL, matching EnVAR's name validation. Unlike EnVAR's persistent Windows user/system registry operations, these changes affect only this interpreter and its children; they are not broadcast or saved. Non-UTF-8 environment values produce a diagnostic when read or listed.
- `path.join(...)`, `parent`, `filename`, `extension`, `absolute`, `exists`, `is_file`, and `is_dir` use platform path semantics. `join` needs at least one string; absent path components return `none`. `absolute` does not require the target to exist.
- `process.cwd()`, `chdir(path)`, `args()`, and `pid()` expose the working directory, program arguments after `unk run <file>`, and process ID. `-c` receives no program arguments.
- `unknown-shell` provides `read_line()`, `write(text)`, `write_line(text)`, `clear()`, and `is_tty()` (stdout). Input strips a trailing newline; EOF returns an empty string. Output is flushed. Clear uses USH's ANSI sequence and Windows console setup.
- `system.os()`, `arch()`, `hostname()`, and `cpu_count()` report platform information; CPU count reflects available parallelism.
- `time.now()` returns milliseconds since the Unix epoch; `time.monotonic()` returns elapsed milliseconds since interpreter creation; `time.sleep(milliseconds)` accepts a nonnegative integer. The monotonic values are intended for differences, not wall-clock dates.

`break` and `continue` affect the nearest enclosing loop and are errors outside a loop, including across function boundaries. See `examples/system.unk`.
