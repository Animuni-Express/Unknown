"use client";
import dynamic from "next/dynamic";
import Link from "next/link";
import { useState, useEffect, useRef, useCallback } from "react";
import type { editor } from "monaco-editor";
import {
  Play,
  Square,
  RotateCcw,
  Copy,
  Share2,
  Plus,
  Trash2,
  Download,
  FileCode,
  Folder,
  Terminal,
  ChevronRight,
  CheckCircle2,
  AlertCircle,
  BookOpen,
} from "lucide-react";
import { Button } from "./ui/button";
import { examples } from "@/lib/examples";
import {
  asset,
  initialFiles,
  validFiles,
  byteLength,
  MAX_SOURCE,
  encodeSource,
  decodeSource,
  diagnosticLink,
  editorColumn,
  type Diagnostic,
  type Files,
  type ExecutionResult,
} from "@/lib/runtime";
const Editor = dynamic(() => import("./editor"), {
  ssr: false,
  loading: () => <div className="editor-loading">Loading Monaco editor…</div>,
});
const STORAGE = "unknown-workspace-v1";
export function Playground() {
  const [files, setFiles] = useState<Files>(initialFiles);
  const [active, setActive] = useState("/main.unk");
  const [cwd, setCwd] = useState("/");
  const [loaded, setLoaded] = useState(false);
  const [fontSize, setFontSize] = useState(14);
  const [result, setResult] = useState<ExecutionResult | null>(null);
  const [status, setStatus] = useState("Ready");
  const [message, setMessage] = useState("");
  const [running, setRunning] = useState(false);
  const [chosen, setChosen] = useState("hello");
  const [newPath, setNewPath] = useState("");
  const [adding, setAdding] = useState(false);
  const worker = useRef<Worker | null>(null);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const editorRef = useRef<editor.IStandaloneCodeEditor | null>(null);
  const pendingDiagnostic = useRef<Diagnostic | null>(null);
  const monacoRef = useRef<typeof import("monaco-editor") | null>(null);
  const runRef = useRef<() => void>(() => {});
  const source = files.find(([path]) => path === active)?.[1] ?? "";
  const runtime = active.endsWith(".ush") ? "ush" : "unk";
  useEffect(() => {
    let restoredFiles = initialFiles;
    let restoredActive = "/main.unk";
    let restoredCwd = "/";
    try {
      const raw = localStorage.getItem(STORAGE);
      if (raw) {
        const saved = JSON.parse(raw);
        if (
          validFiles(saved.files) &&
          saved.files.some(
            (f: [string, string | null]) =>
              f[0] === saved.active && typeof f[1] === "string",
          )
        ) {
          restoredFiles = saved.files;
          restoredActive = saved.active;
          restoredCwd = typeof saved.cwd === "string" ? saved.cwd : "/";
        } else
          setMessage("Saved workspace is invalid. Opened the default files.");
      }
    } catch {
      setMessage(
        "Local storage is unavailable or damaged. Use Download file to keep your work.",
      );
    }
    const params = new URLSearchParams(location.search);
    const example = examples.find((e) => e.id === params.get("example"));
    try {
      const shared = new URLSearchParams(location.hash.slice(1));
      if (shared.has("code")) {
        const text = decodeSource(shared.get("code")!);
        const path =
          shared.get("runtime") === "ush" ? "/shared.ush" : "/shared.unk";
        const next: Files = [
          ...restoredFiles.filter(([p]) => p !== path),
          [path, text],
        ];
        if (!validFiles(next))
          throw new Error(
            "Shared source would exceed workspace limits. Download or delete files first.",
          );
        restoredFiles = next;
        restoredActive = path;
        setMessage("Shared source loaded. Review it, then choose Run.");
      } else if (example) {
        const next: Files = [
          ...restoredFiles.filter(([p]) => p !== "/main.unk"),
          ["/main.unk", example.source],
        ];
        if (!validFiles(next))
          throw new Error(
            "Example would exceed workspace limits. Download or delete files first.",
          );
        restoredFiles = next;
        restoredActive = "/main.unk";
        setChosen(example.id);
      }
    } catch (error) {
      setMessage(
        error instanceof Error ? error.message : "Invalid shared source.",
      );
    }
    setFiles(restoredFiles);
    setActive(restoredActive);
    setCwd(restoredCwd);
    setLoaded(true);
    return () => {
      worker.current?.terminate();
      if (timer.current) clearTimeout(timer.current);
    };
  }, []);
  useEffect(() => {
    if (!loaded) return;
    try {
      localStorage.setItem(STORAGE, JSON.stringify({ files, active, cwd }));
    } catch {
      setMessage(
        "Workspace could not be saved. Use Download file to keep your work.",
      );
    }
  }, [files, active, cwd, loaded]);
  const updateSource = useCallback(
    (text: string) => {
      if (byteLength(text) > MAX_SOURCE) {
        setMessage("Edit rejected: source is limited to 64 KiB.");
        queueMicrotask(() => editorRef.current?.getModel()?.setValue(source));
        return;
      }
      const updated: Files = files.map(([p, v]) => [
        p,
        p === active ? text : v,
      ]);
      if (!validFiles(updated)) {
        setMessage("Edit rejected: the workspace is limited to 1 MiB.");
        queueMicrotask(() => editorRef.current?.getModel()?.setValue(source));
        return;
      }
      setFiles(updated);
    },
    [active, files, source],
  );
  function endWorker() {
    worker.current?.terminate();
    worker.current = null;
    if (timer.current) clearTimeout(timer.current);
    timer.current = null;
    setRunning(false);
  }
  function stop() {
    endWorker();
    setStatus("Stopped");
    setMessage("Execution stopped. Pending file changes were discarded.");
  }
  function run() {
    if (running) return;
    if (byteLength(source) > MAX_SOURCE) {
      setMessage("Source exceeds 64 KiB.");
      return;
    }
    setMessage("");
    setResult(null);
    setRunning(true);
    setStatus("Running");
    const current = new Worker(asset("/runtime/worker.js"), { type: "module" });
    worker.current = current;
    current.onmessage = (event: MessageEvent<ExecutionResult>) => {
      const data = event.data;
      endWorker();
      setResult(data);
      setStatus(data.exitCode === 0 ? "Finished" : "Failed");
      if (validFiles(data.files)) {
        setFiles(data.files);
        setCwd(data.cwd);
        if (!data.files.some(([p, v]) => p === active && v !== null)) {
          const next = data.files.find(([, v]) => v !== null);
          if (next) setActive(next[0]);
        }
      }
    };
    current.onerror = (event) => {
      endWorker();
      setStatus("Failed");
      setMessage(
        `Runtime worker failed: ${event.message || "Could not initialize WebAssembly. Check browser support and asset access."}`,
      );
    };
    timer.current = setTimeout(() => {
      endWorker();
      setStatus("Timed out");
      setMessage(
        "Execution exceeded 5 seconds. The worker was terminated; pending file changes were discarded.",
      );
    }, 5000);
    current.postMessage({ runtime, source, filename: active, files, cwd });
  }
  runRef.current = run;
  useEffect(() => {
    if (!monacoRef.current) return;
    const model = editorRef.current?.getModel();
    if (!model) return;
    monacoRef.current.editor.setModelMarkers(
      model,
      "unknown",
      (result?.diagnostics || [])
        .filter((d) => d.line && (!d.filename || d.filename === active))
        .map((d) => ({
          severity: monacoRef.current!.MarkerSeverity.Error,
          message: `${d.code ? `[${d.code}] ` : ""}${d.message}${d.help ? `\n${d.help}` : ""}`,
          startLineNumber: d.line!,
          startColumn: editorColumn(source, d.line!, d.column || 1),
          endLineNumber: d.endLine || d.line!,
          endColumn: editorColumn(
            source,
            d.endLine || d.line!,
            d.endColumn || (d.column || 1) + 1,
          ),
        })),
    );
    if (pendingDiagnostic.current) {
      const d = pendingDiagnostic.current;
      pendingDiagnostic.current = null;
      editorRef.current?.focus();
      editorRef.current?.setPosition({
        lineNumber: d.line || 1,
        column: editorColumn(source, d.line || 1, d.column || 1),
      });
      editorRef.current?.revealLineInCenter(d.line || 1);
    }
  }, [result, active]);
  async function copy(text: string, label: string) {
    try {
      await navigator.clipboard.writeText(text);
      setMessage(`${label} copied.`);
    } catch {
      setMessage(
        "Clipboard unavailable. Select the text and copy it manually.",
      );
    }
  }
  function loadExample(id: string) {
    const example = examples.find((e) => e.id === id)!;
    if (
      source &&
      source !== example.source &&
      !confirm("Replace /main.unk with the selected example?")
    )
      return;
    const next: Files = [
      ...files.filter(([p]) => p !== "/main.unk"),
      ["/main.unk", example.source],
    ];
    if (!validFiles(next)) {
      setMessage(
        "Example would exceed workspace limits. Download or delete files first.",
      );
      return;
    }
    setChosen(id);
    setActive("/main.unk");
    setFiles(next);
    setResult(null);
  }
  function addFile() {
    const path = newPath.startsWith("/")
      ? newPath
      : `${cwd.replace(/\/$/, "")}/${newPath}`;
    if (
      !newPath ||
      !validFiles([[path, ""]]) ||
      files.some(([p]) => p === path)
    ) {
      setMessage("Use a unique path without .., backslashes, or colons.");
      return;
    }
    if (files.length >= 256) {
      setMessage("Workspace is limited to 256 entries.");
      return;
    }
    const parent = path.slice(0, path.lastIndexOf("/")) || "/";
    if (!files.some(([p, v]) => p === parent && v === null)) {
      setMessage("Create the parent folder with USH first.");
      return;
    }
    setFiles((current) => [...current, [path, ""]]);
    setActive(path);
    setAdding(false);
    setNewPath("");
  }
  function download() {
    const url = URL.createObjectURL(
      new Blob([source], { type: "text/plain;charset=utf-8" }),
    );
    const anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = active.split("/").pop() || "program.unk";
    anchor.click();
    URL.revokeObjectURL(url);
  }
  const cleanOutput = (result?.stdout || "").replace(
    /\x1b\[[0-9;]*[A-Za-z]/g,
    "",
  );
  return (
    <main className="playground">
      <div className="playground-heading">
        <div>
          <span className="eyebrow">Your browser. Your workspace.</span>
          <h1>
            Playground<span className="version-badge">v0.1.0</span>
          </h1>
        </div>
        <Link href="/docs/ush/">
          <BookOpen size={14} /> Workspace guide <ChevronRight size={14} />
        </Link>
      </div>
      <div className="workspace-toolbar">
        <label className="example-select">
          <span>Examples</span>
          <select
            aria-label="Load example"
            value={chosen}
            disabled={running}
            onChange={(e) => loadExample(e.target.value)}
          >
            {[...new Set(examples.map((e) => e.category))].map((group) => (
              <optgroup label={group} key={group}>
                {examples
                  .filter((e) => e.category === group)
                  .map((e) => (
                    <option key={e.id} value={e.id}>
                      {e.title} · {e.difficulty}
                    </option>
                  ))}
              </optgroup>
            ))}
          </select>
        </label>
        <div className="toolbar-actions">
          <Button
            disabled={running || !loaded || !active.match(/\.(unk|ush)$/)}
            onClick={run}
          >
            <Play size={14} fill="currentColor" /> Run <kbd>⌘ ↵</kbd>
          </Button>
          <Button variant="secondary" disabled={!running} onClick={stop}>
            <Square size={12} /> Stop
          </Button>
          <button
            className="tool-button"
            disabled={running}
            aria-label="Reset active file"
            onClick={() => {
              if (confirm("Reset the active file?")) {
                updateSource(
                  active.endsWith(".ush")
                    ? initialFiles[2][1]!
                    : examples.find((e) => e.id === chosen)!.source,
                );
                setResult(null);
              }
            }}
          >
            <RotateCcw size={15} />
            <span>Reset</span>
          </button>
          <button
            className="tool-button"
            aria-label="Copy source"
            onClick={() => copy(source, "Source")}
          >
            <Copy size={15} />
            <span>Copy</span>
          </button>
          <button
            className="tool-button"
            aria-label="Share source"
            onClick={() => {
              try {
                const code = encodeSource(source);
                const url = `${location.origin}${asset("/playground/")}#code=${code}&runtime=${runtime}`;
                if (url.length > 16000)
                  throw new Error(
                    "Share links are limited to 16,000 characters. Download this file instead.",
                  );
                void copy(url, "Share link");
              } catch (error) {
                setMessage((error as Error).message);
              }
            }}
          >
            <Share2 size={15} />
            <span>Share</span>
          </button>
        </div>
      </div>
      <div className="example-description">
        {examples.find((e) => e.id === chosen)?.description}{" "}
        <span>
          Ctrl/Cmd+Enter to run. Shared source never runs automatically.
        </span>
      </div>
      {message && (
        <div role="status" className="workspace-message">
          {message}
          <button aria-label="Dismiss message" onClick={() => setMessage("")}>
            ×
          </button>
        </div>
      )}
      <div className="workspace">
        <aside className="file-explorer">
          <div className="panel-title">
            <span>Files</span>
            <button
              className="icon-button"
              aria-label="New file"
              disabled={running}
              onClick={() => setAdding(!adding)}
            >
              <Plus size={15} />
            </button>
          </div>
          <div className="directory">
            <Folder size={13} />
            <span>workspace</span>
          </div>
          {adding && (
            <form
              className="new-file"
              onSubmit={(e) => {
                e.preventDefault();
                addFile();
              }}
            >
              <input
                aria-label="New file path"
                placeholder="module.unk"
                value={newPath}
                onChange={(e) => setNewPath(e.target.value)}
                autoFocus
              />
              <button type="submit">Add</button>
            </form>
          )}
          <nav aria-label="Workspace files">
            {files
              .filter(([p]) => p !== "/")
              .sort(([a], [b]) => a.localeCompare(b))
              .map(([path, value]) =>
                value === null ? (
                  <div className="file-folder" key={path}>
                    <Folder size={13} />
                    {path.slice(1)}
                  </div>
                ) : (
                  <button
                    key={path}
                    className={`file-item ${active === path ? "active" : ""}`}
                    disabled={running}
                    onClick={() => {
                      setActive(path);
                      setResult(null);
                    }}
                    title={path}
                  >
                    <FileCode size={14} />
                    <span>{path.slice(1)}</span>
                  </button>
                ),
              )}
          </nav>
          <div className="file-actions">
            <button onClick={download}>
              <Download size={13} /> Download file
            </button>
            <button
              disabled={
                running || files.filter(([, v]) => v !== null).length < 2
              }
              onClick={() => {
                if (confirm(`Delete ${active}?`)) {
                  const next = files.filter(([p]) => p !== active);
                  setFiles(next);
                  setActive(next.find(([, v]) => v !== null)![0]);
                }
              }}
            >
              <Trash2 size={13} /> Delete file
            </button>
          </div>
          <div className="filesystem-note">
            Browser-local files
            <br />1 MiB · 256 entries
          </div>
        </aside>
        <section className="editor-panel" aria-label="Source editor">
          <div className="editor-tab">
            <span>
              <FileCode size={14} />
              {active.split("/").pop()}
            </span>
            <label>
              Font size{" "}
              <select
                aria-label="Editor font size"
                value={fontSize}
                onChange={(e) => setFontSize(Number(e.target.value))}
              >
                {[12, 14, 16, 18, 20].map((v) => (
                  <option key={v} value={v}>
                    {v}px
                  </option>
                ))}
              </select>
            </label>
          </div>
          <div className="monaco-container">
            <Editor
              onLimit={() =>
                setMessage("Edit rejected: source is limited to 64 KiB.")
              }
              readOnly={running}
              value={source}
              filename={active}
              onChange={updateSource}
              fontSize={fontSize}
              diagnostics={result?.diagnostics || []}
              onRun={() => runRef.current()}
              onMount={(ed, m) => {
                editorRef.current = ed;
                monacoRef.current = m;
              }}
            />
          </div>
        </section>
        <section className="output-panel" aria-label="Execution output">
          <div className="panel-title">
            <span>
              <Terminal size={14} /> Console
            </span>
            <button onClick={() => setResult(null)}>Clear console</button>
          </div>
          <div className="console-content" aria-live="polite">
            {result ? (
              <>
                <pre data-testid="stdout">{cleanOutput}</pre>
                {result.stderr && (
                  <pre className="stderr" data-testid="stderr">
                    {result.stderr}
                  </pre>
                )}
                {!result.stdout && !result.stderr && (
                  <p className="muted">Program completed without output.</p>
                )}
              </>
            ) : (
              <div className="console-empty">
                <Terminal size={25} />
                <p>Your program starts here.</p>
                <span>Choose Run to see output.</span>
              </div>
            )}
          </div>
          <div className="diagnostics">
            <div className="panel-title">
              <span>
                Diagnostics{" "}
                {result?.diagnostics.length
                  ? `(${result.diagnostics.length})`
                  : ""}
              </span>
              {result && (
                <button onClick={() => copy(result.stderr, "Error")}>
                  Copy error
                </button>
              )}
            </div>
            {result?.diagnostics.length ? (
              result.diagnostics.map((d, i) => (
                <div className="diagnostic" key={i}>
                  <p>
                    <AlertCircle size={14} />
                    <strong>{d.code}</strong> {d.message}
                  </p>
                  {d.help && <p>{d.help}</p>}
                  <div>
                    <button
                      onClick={() => {
                        if (
                          d.filename &&
                          d.filename !== active &&
                          files.some(([p, v]) => p === d.filename && v !== null)
                        ) {
                          pendingDiagnostic.current = d;
                          setActive(d.filename);
                          return;
                        }
                        editorRef.current?.focus();
                        editorRef.current?.setPosition({
                          lineNumber: d.line || 1,
                          column: editorColumn(
                            source,
                            d.line || 1,
                            d.column || 1,
                          ),
                        });
                        editorRef.current?.revealLineInCenter(d.line || 1);
                      }}
                    >
                      Go to error {d.line ? `${d.line}:${d.column}` : ""}
                    </button>
                    <Link href={diagnosticLink(d)}>Read documentation</Link>
                  </div>
                </div>
              ))
            ) : (
              <p className="diagnostics-empty">
                {result?.exitCode === 0 ? (
                  <>
                    <CheckCircle2 size={14} /> No diagnostics.
                  </>
                ) : (
                  "Source diagnostics appear here."
                )}
              </p>
            )}
          </div>
        </section>
      </div>
      <div className="statusbar">
        <span>
          <span className={`status-dot ${running ? "running" : ""}`} />
          {status}
        </span>
        <span>
          {runtime === "ush" ? "USH" : "Unknown"} · Rust / WebAssembly
        </span>
        <span>
          {cwd}{" "}
          <span className="status-duration">
            {result
              ? `${result.durationMs.toFixed(1)} ms`
              : "5 s execution limit"}
          </span>
        </span>
      </div>
      <p className="workspace-footnote">
        Files stay in this browser. Download work you want to keep. Native
        processes and machine access are unavailable.
      </p>
    </main>
  );
}
