import { execFileSync } from "node:child_process";
import { mkdirSync, existsSync } from "node:fs";
import { resolve } from "node:path";
const root = resolve(import.meta.dirname, "../..");
execFileSync(
  "cargo",
  [
    "build",
    "--manifest-path",
    resolve(root, "browser/Cargo.toml"),
    "--target",
    "wasm32-unknown-unknown",
    "--release",
    "--locked",
  ],
  { cwd: root, stdio: "inherit" },
);
mkdirSync(resolve(root, "website/public/runtime"), { recursive: true });
const localBindgen = resolve(
  root,
  `.tools/bin/wasm-bindgen${process.platform === "win32" ? ".exe" : ""}`,
);
execFileSync(
  existsSync(localBindgen) ? localBindgen : "wasm-bindgen",
  [
    "--target",
    "web",
    "--out-dir",
    resolve(root, "website/public/runtime"),
    "--out-name",
    "unknown_browser",
    resolve(
      root,
      "browser/target/wasm32-unknown-unknown/release/unknown_browser.wasm",
    ),
  ],
  { stdio: "inherit" },
);
const { build } = await import("esbuild");
await build({
  entryPoints: [resolve(root, "website/workers/runtime.ts")],
  bundle: true,
  format: "esm",
  platform: "browser",
  outfile: resolve(root, "website/public/runtime/worker.js"),
  external: ["./unknown_browser.js"],
});
mkdirSync(resolve(root, "website/public/monaco"), { recursive: true });
// Monaco's editor worker is bundled locally; the language has no remote service.
await build({
  entryPoints: [
    resolve(
      root,
      "website/node_modules/monaco-editor/esm/vs/editor/editor.worker.js",
    ),
  ],
  bundle: true,
  format: "iife",
  outfile: resolve(root, "website/public/monaco/editor.worker.js"),
});
