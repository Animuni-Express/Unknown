import { execFileSync } from "node:child_process";
import { mkdtempSync, rmSync, readFileSync, readdirSync } from "node:fs";
import { tmpdir } from "node:os";
import { resolve, join } from "node:path";
import { examples } from "../lib/examples.ts";
const root = resolve(import.meta.dirname, "../..");
execFileSync("cargo", ["build", "--locked"], { cwd: root, stdio: "inherit" });
const dir = mkdtempSync(join(tmpdir(), "unknown-examples-"));
try {
  for (const example of examples) {
    const actual = execFileSync(
      resolve(
        root,
        `target/debug/unk${process.platform === "win32" ? ".exe" : ""}`,
      ),
      ["-c", example.source],
      { cwd: dir, encoding: "utf8" },
    ).replaceAll("\r\n", "\n");
    if (actual !== example.output)
      throw new Error(
        `Example ${example.id}: expected ${JSON.stringify(example.output)}, received ${JSON.stringify(actual)}`,
      );
  }
  for (const file of readdirSync(resolve(root, "website/content/docs"), {
    recursive: true,
  }).filter((f) => f.endsWith(".mdx"))) {
    const content = readFileSync(
      resolve(root, "website/content/docs", file),
      "utf8",
    );
    for (const match of content.matchAll(/<Example id="([^"]+)"/g))
      if (!examples.some((e) => e.id === match[1]))
        throw new Error(`Missing example ${match[1]}`);
  }
  console.log(
    `All ${examples.length} examples and MDX example references passed against the native Rust interpreter.`,
  );
} finally {
  rmSync(dir, { recursive: true, force: true });
}
