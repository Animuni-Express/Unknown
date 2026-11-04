import { createServer } from "node:http";
import { readFile, stat } from "node:fs/promises";
import { resolve, extname, sep } from "node:path";
const root = resolve("out");
const prefix = process.env.TEST_BASE_PATH || "";
const port = Number(process.env.PORT || 4173);
const mime = {
  ".html": "text/html;charset=utf-8",
  ".js": "text/javascript;charset=utf-8",
  ".css": "text/css;charset=utf-8",
  ".json": "application/json",
  ".wasm": "application/wasm",
  ".svg": "image/svg+xml",
  ".woff2": "font/woff2",
  ".txt": "text/plain;charset=utf-8",
  ".ico": "image/x-icon",
  ".xml": "application/xml",
};
createServer(async (req, res) => {
  try {
    let path = decodeURIComponent(
      new URL(req.url, "http://localhost").pathname,
    );
    if (prefix) {
      if (path !== prefix && !path.startsWith(prefix + "/")) {
        res.writeHead(404).end("Not found");
        return;
      }
      path = path.slice(prefix.length) || "/";
    }
    let file = resolve(root, "." + path);
    if (file !== root && !file.startsWith(root + sep)) {
      res.writeHead(403).end("Forbidden");
      return;
    }
    if ((await stat(file)).isDirectory()) file = resolve(file, "index.html");
    const data = await readFile(file);
    res.writeHead(200, {
      "Content-Type": mime[extname(file)] || "application/octet-stream",
    });
    res.end(req.method === "HEAD" ? undefined : data);
  } catch {
    res.writeHead(404, { "Content-Type": "text/plain" }).end("Not found");
  }
}).listen(port, "127.0.0.1", () =>
  console.log(`Static export: http://127.0.0.1:${port}${prefix}/`),
);
