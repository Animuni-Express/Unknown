import { execFileSync } from "node:child_process";
import { cpSync, rmSync } from "node:fs";
import { resolve, basename } from "node:path";
const target = process.argv[2];
if (!["sites", "pages"].includes(target))
  throw new Error("Use build-target.mjs sites|pages");
const basePath =
  target === "pages" ? (process.env.PAGES_BASE_PATH ?? "/Unknown") : "";
const siteUrl =
  target === "pages"
    ? process.env.PAGES_SITE_URL || "https://animuni-express.github.io"
    : process.env.SITES_SITE_URL ||
      "https://unknown-trying.animuni-609.chatgpt.site";
execFileSync(process.execPath, ["scripts/content.mjs"], { stdio: "inherit" });
execFileSync(
  process.execPath,
  ["node_modules/next/dist/bin/next", "build", "--turbopack"],
  {
    stdio: "inherit",
    env: {
      ...process.env,
      NEXT_PUBLIC_BASE_PATH: basePath,
      NEXT_PUBLIC_SITE_URL: siteUrl,
    },
  },
);
console.log(
  `Built ${target} static export: out/ (basePath=${basePath || "/"})`,
);
if (target === "sites") {
  const projectRoot = resolve(import.meta.dirname, "../..");
  const siteOut = resolve(projectRoot, "out");
  if (basename(siteOut) !== "out" || resolve(siteOut, "..") !== projectRoot)
    throw new Error("Invalid Sites artifact directory");
  rmSync(siteOut, { recursive: true, force: true });
  cpSync(resolve(import.meta.dirname, "../out"), siteOut, { recursive: true });
}
