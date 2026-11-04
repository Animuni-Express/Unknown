import { readFileSync } from "node:fs";
import assert from "node:assert/strict";
import { examples } from "../lib/examples.ts";
import { initSync, run } from "../public/runtime/unknown_browser.js";
initSync({
  module: readFileSync(
    new URL("../public/runtime/unknown_browser_bg.wasm", import.meta.url),
  ),
});
let files = [["/", null]],
  cwd = "/";
function execute(
  source,
  runtime = "unk",
  filename = runtime === "unk" ? "/main.unk" : "/session.ush",
) {
  const result = JSON.parse(
    run(JSON.stringify({ source, runtime, filename, files, cwd })),
  );
  files = result.files;
  cwd = result.cwd;
  return result;
}
for (const example of examples) {
  const result = execute(example.source);
  assert.equal(result.exitCode, 0, `${example.id}: ${result.stderr}`);
  assert.equal(result.stdout, example.output, example.id);
}
assert.equal(execute("read message.txt", "ush").stdout, "Hello from Unknown");
assert.equal(
  execute(
    "make folder modules\nmake file modules/math.unk\nlist --sort=name",
    "ush",
  ).exitCode,
  0,
);
const imported = execute(
  'import "file"\nfile.write("modules/math.unk", "fn add(int a, int b) -> int {\\n    return a + b\\n}\\n")\nimport "modules/math" as "math"\nprint(math.add(2, 3))',
);
assert.equal(imported.stdout, "5\n", imported.stderr);
const handle = execute(
  'f = open("message.txt", "a")\nf.write("!")\nf.close()\nimport "file"\nprint(file.read("message.txt"))',
);
assert.equal(handle.stdout, "Hello from Unknown!\n", handle.stderr);
assert.equal(execute("read message.txt", "ush").stdout, "Hello from Unknown!");
const error = execute("print(missing)");
assert.equal(error.exitCode, 1);
assert.equal(error.diagnostics[0].code, "E2001");
assert.equal(error.diagnostics[0].line, 1);
assert.equal(
  execute('import "process"\nprocess.run("whoami", [])').exitCode,
  1,
);
assert.equal(execute("whoami", "ush").exitCode, 1);
assert.equal(execute("goto ../../escape", "ush").exitCode, 1);
assert.equal(
  execute(
    'for (i in range(100000)):\n    print("xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx")',
  ).exitCode,
  1,
);
assert.equal(execute('print("recovered")').stdout, "recovered\n");
const eof = execute(
  'import "file"\nfile.write("cursor.txt", "abc")\nf = open("cursor.txt", "r")\nprint(f.read())\nfile.write("cursor.txt", "")\nprint(f.read())',
);
assert.equal(eof.exitCode, 0, eof.stderr);
assert.equal(eof.stdout, "abc\n\n");
const overLimit = [
  ["/", null],
  ["/big.txt", "x".repeat(1_048_576)],
  ["/notes.txt", "keep"],
];
const unicodeFiles = [
  ["/", null],
  ["/source.txt", "x".repeat(8191) + "é" + "y".repeat(30)],
];
const unicodeCopy = JSON.parse(
  run(
    JSON.stringify({
      runtime: "ush",
      source: "copy source.txt target.txt",
      filename: "/session.ush",
      files: unicodeFiles,
      cwd: "/",
    }),
  ),
);
assert.equal(unicodeCopy.exitCode, 0, unicodeCopy.stderr);
assert.equal(
  unicodeCopy.files.find(([p]) => p === "/target.txt")[1],
  unicodeFiles[1][1],
);
const preserved = JSON.parse(
  run(
    JSON.stringify({
      runtime: "unk",
      source: "print(1)",
      filename: "/main.unk",
      files: overLimit,
      cwd: "/",
    }),
  ),
);
assert.equal(preserved.exitCode, 1);
assert.deepEqual(preserved.files, overLimit);
assert.equal(
  execute(
    'import "file"\nfile.write("modules/broken.unk", "print(missing)\\n")\nimport "modules/broken" as "broken"',
  ).diagnostics[0].filename,
  "/modules/broken.unk",
);
console.log(
  `WASM: ${examples.length} examples, shared USH/Unknown files, handles, imports, diagnostics, output limits, sandbox restrictions, and recovery passed.`,
);
