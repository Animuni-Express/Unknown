export type Files = [string, string | null][];
export interface Diagnostic {
  severity: "error" | "warning" | "info";
  code?: string;
  message: string;
  line?: number;
  column?: number;
  endLine?: number;
  endColumn?: number;
  help?: string;
  filename?: string;
}
export interface ExecutionResult {
  stdout: string;
  stderr: string;
  diagnostics: Diagnostic[];
  exitCode: number;
  durationMs: number;
  files: Files;
  cwd: string;
}
export interface RunRequest {
  runtime: "unk" | "ush";
  source: string;
  filename: string;
  files: Files;
  cwd: string;
}
export const MAX_SOURCE = 65_536;
export const byteLength = (source: string) =>
  new TextEncoder().encode(source).length;
export function editorColumn(source: string, line: number, column: number) {
  return (
    Array.from(source.split("\n")[line - 1] || "")
      .slice(0, column - 1)
      .join("").length + 1
  );
}
export const asset = (path: string) =>
  `${process.env.NEXT_PUBLIC_BASE_PATH || ""}${path}`;
export const initialFiles: Files = [
  ["/", null],
  ["/main.unk", 'print("Hello, Unknown!")\n'],
  ["/session.ush", "where\nlist\nread README.txt\n"],
  [
    "/README.txt",
    "Welcome to your browser workspace.\nUnknown and USH share these files.\n",
  ],
];
export function canonicalPath(path: string) {
  return (
    path.startsWith("/") &&
    path.length <= 512 &&
    !/[\\:\0]/.test(path) &&
    (path === "/" ||
      path
        .slice(1)
        .split("/")
        .every((part) => part !== "" && part !== "." && part !== ".."))
  );
}
export function validFiles(value: unknown): value is Files {
  return (
    Array.isArray(value) &&
    value.length <= 256 &&
    value.every(
      (v) =>
        Array.isArray(v) &&
        v.length === 2 &&
        typeof v[0] === "string" &&
        canonicalPath(v[0]) &&
        (v[1] === null || typeof v[1] === "string"),
    ) &&
    new Set(value.map((v) => v[0])).size === value.length &&
    value.reduce(
      (total, v) => total + (typeof v[1] === "string" ? byteLength(v[1]) : 0),
      0,
    ) <= 1_048_576
  );
}
export function encodeSource(source: string) {
  if (byteLength(source) > MAX_SOURCE)
    throw new Error("Source exceeds 64 KiB.");
  const bytes = new TextEncoder().encode(source);
  return btoa(Array.from(bytes, (b) => String.fromCharCode(b)).join(""))
    .replaceAll("+", "-")
    .replaceAll("/", "_")
    .replaceAll("=", "");
}
export function decodeSource(encoded: string) {
  if (encoded.length > MAX_SOURCE * 2)
    throw new Error("Shared source is too large.");
  const bytes = Uint8Array.from(
    atob(encoded.replaceAll("-", "+").replaceAll("_", "/")),
    (c) => c.charCodeAt(0),
  );
  if (bytes.length > MAX_SOURCE)
    throw new Error("Shared source exceeds 64 KiB.");
  return new TextDecoder("utf-8", { fatal: true }).decode(bytes);
}
export function diagnosticLink(d: Diagnostic) {
  if (d.message.includes("expected '('")) return "/docs/language/conditions/";
  if (d.code === "E2001") return "/docs/language/scope/";
  if (d.code?.startsWith("E4")) return "/docs/standard-library/#modules";
  return "/docs/errors/";
}
