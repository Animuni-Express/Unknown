# Unknown Trying

One static Next.js App Router website, one MDX content system, one Monaco
playground, and the actual Rust Unknown and USH cores. No mandatory backend or
Netlify dependency. Both hosts use the same source and browser execution path.

## Prerequisites and development

- Node.js 24 and npm (the lockfile pins JavaScript dependencies).
- Stable Rust; `rustup target add wasm32-unknown-unknown`.
- wasm-bindgen-cli **0.2.130**, matching `browser/Cargo.lock`:
  `cargo install wasm-bindgen-cli --version 0.2.130 --locked --root .tools`
  from the repository root. This is project-local; no global PATH change is needed.

From `website/`:

```sh
npm ci
npm run build:wasm
npm run dev
```

Development and production use Turbopack. Next.js uses its default SWC compiler;
there is no Babel override. Tailwind 4 supplies styling and the shared Button
follows shadcn/ui conventions. MDX is integrated directly with Next.js; a small
generated index provides navigation, section search, and TOC without an additional
documentation framework. Monaco and workers are self-hosted, with no CDN dependency.

`build:wasm` compiles `browser/`, generates web bindings, and bundles the execution
and editor workers into `public/`. Regenerate it after any Rust core change.

## Architecture

```text
src/ Unknown library ──> unk CLI
         │
         └──> browser/ wasm-bindgen adapter ──> Web Worker ──> Monaco
crates/ush-core shared command dispatcher ────> same adapter
         │
         ├── native std::fs backend
         └── browser VFS shared by both interpreters
content/docs/*.mdx + lib/examples.ts ──> static docs, search, Try it
```

The original `USH/` checkout is preserved. The tracked integration core in
`crates/ush-core` is the sole shell implementation used by Unknown Trying.
It retains the native USH dispatcher and switches filesystem backends for WASM.

## Independent production builds

```sh
npm run build:sites
npm run test:e2e

npm run build:pages
```

Each build writes a fully static `out/` directory. The next build replaces that
directory, so run its browser tests before building the other target.

For the Pages browser tests set `TEST_BASE_PATH=/Unknown` before
`npm run test:e2e`. In PowerShell use `$env:TEST_BASE_PATH='/Unknown'`.
In a POSIX shell use `TEST_BASE_PATH=/Unknown npm run test:e2e`.

| Setting | Purpose |
| --- | --- |
| `NEXT_PUBLIC_BASE_PATH` | Generic build-time subpath; empty at an origin root |
| `NEXT_PUBLIC_SITE_URL` | Generic canonical origin; no path component |
| `PAGES_BASE_PATH` | Pages build subpath, defaults to `/Unknown`; empty for a custom domain or account-root repository |
| `PAGES_SITE_URL` | Pages canonical origin |
| `SITES_SITE_URL` | Sites canonical origin; defaults to the registered project's expected URL |
| `TEST_BASE_PATH` | Static test server prefix matching the already-built export |

`basePath`, `assetPrefix`, all public worker/WASM URLs, documentation links,
canonical URLs, and sitemap entries use the selected host settings. Trailing
slashes emit actual `route/index.html` files for direct static navigation.
No Next.js server or SSR is required in production. See
[Next.js static exports](https://nextjs.org/docs/app/guides/static-exports).

## GitHub Pages

In repository Settings → Pages, select **GitHub Actions** as the publishing source.
`.github/workflows/pages.yml` tests both targets in independent jobs, then deploys
the Pages `website/out/` artifact after successful tests on a push to `main` or
`master`, or a manual dispatch. Pull requests validate without deployment.
It uses the repository name for the subpath and repository owner for the origin.
The configured GitHub Pages environment must permit the selected branch.
See [GitHub's custom Pages workflow documentation](https://docs.github.com/en/pages/getting-started-with-github-pages/using-custom-workflows-with-github-pages).

## ChatGPT Sites

The root `.openai/hosting.json` binds the project and declares the root `out/` as
the static directory. `build:sites` copies `website/out/` there for Sites packaging;
GitHub Pages uses `website/out/` directly. Next.js static exports use Sites' static asset hosting;
they do not require a server-side Next.js adapter. WASM and module Web Workers
are ordinary same-origin browser assets. Verify real execution on the published
origin too, because host response headers and access controls can affect them.

Publishing is explicit: build the Sites target, preview and test locally, push the
exact source state to the registered Sites source repository through the Sites
workflow, package that build with the hosting manifest, save a version, deploy
the saved version, and inspect deployment status. Preserve the current audience.
Do not reuse Pages output for Sites: it contains repository-prefixed URLs.
Every Sites deployment URL is production; local preview is used for QA.

**GitHub pushes do not publish Sites updates.** The Actions Sites artifact is a
review artifact, not a publication trigger. No secrets belong in the manifest.

## Verification

```sh
npm run typecheck
npm test
npm run test:examples
npm run test:wasm
npx playwright install chromium
npm run test:e2e
```

Rust checks from the repository root:

```sh
cargo test --locked
cargo test --manifest-path crates/ush-core/Cargo.toml --locked
cargo test --manifest-path browser/Cargo.toml --locked
```

Examples are checked against both the native Rust interpreter and generated WASM.
Browser tests exercise documentation/search/Try it, Monaco edits, both runtimes,
shared files, diagnostics, cancellation, timeout recovery, Unicode sharing,
persistence, and mobile overflow at both deployment paths.

## Limits and recovery

Browser files are UTF-8, virtual, and stored per origin in localStorage. The limit
is 1 MiB total file contents and 256 entries. Source and captured output are each
limited to 64 KiB. Share links contain only the active source and are limited to
16,000 characters. They do not execute automatically or include supporting files.
Use Download file for durable copies; browser storage is not a backup.

Workers are terminated on Stop, timeout (five seconds), or component cleanup. Each
run creates a fresh worker. Cancellation discards the pending run's file changes;
completed runs persist them. Editor writes are disabled during execution. Malformed
workspace restores preserve the input snapshot rather than replacing it partially.

Native processes, environment variables, terminal input, hardware I/O, and blocking
sleep are unavailable in the browser and produce explicit errors. USH scripts run
one quoted-argument command per line; pipelines and redirection are not emulated.
The native CLI retains native capabilities. There is no real formatter, so no
Format control is exposed. Planned language features are labeled in the docs.
