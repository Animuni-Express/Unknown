"use client";
import Editor, { loader, type OnMount } from "@monaco-editor/react";
import * as monaco from "monaco-editor/esm/vs/editor/editor.api.js";
import { useEffect, useState } from "react";
import { keywords, types } from "@/lib/language";
import { asset, byteLength, MAX_SOURCE, type Diagnostic } from "@/lib/runtime";
loader.config({ monaco });
if (typeof self !== "undefined") {
  (self as typeof self & { MonacoEnvironment: unknown }).MonacoEnvironment = {
    getWorker: () => new Worker(asset("/monaco/editor.worker.js")),
  };
}
let registered = false;
export default function CodeEditor({
  value,
  onChange,
  filename,
  fontSize,
  diagnostics,
  onMount,
  onRun,
  readOnly,
  onLimit,
}: {
  value: string;
  onChange: (source: string) => void;
  filename: string;
  fontSize: number;
  diagnostics: Diagnostic[];
  onMount: OnMount;
  onRun: () => void;
  readOnly: boolean;
  onLimit: () => void;
}) {
  const [theme, setTheme] = useState("vs-dark");
  useEffect(() => {
    const update = () =>
      setTheme(
        document.documentElement.dataset.theme === "light" ? "vs" : "vs-dark",
      );
    update();
    const observer = new MutationObserver(update);
    observer.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ["data-theme"],
    });
    return () => observer.disconnect();
  }, []);
  return (
    <Editor
      height="100%"
      path={filename}
      value={value}
      language={
        filename.endsWith(".unk")
          ? "unknown"
          : filename.endsWith(".ush")
            ? "ush"
            : "plaintext"
      }
      theme={theme}
      loading={<div className="editor-loading">Loading editor…</div>}
      beforeMount={(m) => {
        if (registered) return;
        registered = true;
        m.languages.register({ id: "unknown", extensions: [".unk"] });
        m.languages.setMonarchTokensProvider("unknown", {
          keywords,
          types,
          tokenizer: {
            root: [
              [/\/\/.*$/, "comment"],
              [/\/\*/, "comment", "@comment"],
              [/"(?:\\.|[^"\\])*"|'(?:\\.|[^'\\])*'/, "string"],
              [/\b\d+\b/, "number"],
              [
                /[a-zA-Z_]\w*/,
                {
                  cases: {
                    "@keywords": "keyword",
                    "@types": "type",
                    "@default": "identifier",
                  },
                },
              ],
              [/[{}()[\]]/, "@brackets"],
            ],
            comment: [
              [/[^/*]+/, "comment"],
              [/\*\//, "comment", "@pop"],
              [/[/*]/, "comment"],
            ],
          },
        });
        m.languages.setLanguageConfiguration("unknown", {
          comments: { lineComment: "//", blockComment: ["/*", "*/"] },
          brackets: [
            ["{", "}"],
            ["[", "]"],
            ["(", ")"],
          ],
          autoClosingPairs: [
            { open: "{", close: "}" },
            { open: "[", close: "]" },
            { open: "(", close: ")" },
            { open: '"', close: '"' },
            { open: "'", close: "'" },
          ],
          indentationRules: {
            increaseIndentPattern: /(:|\{)\s*$/,
            decreaseIndentPattern: /^\s*\}/,
          },
        });
        m.languages.registerCompletionItemProvider("unknown", {
          provideCompletionItems: (
            model: monaco.editor.ITextModel,
            position: monaco.Position,
          ) => {
            const word = model.getWordUntilPosition(position);
            return {
              suggestions: [
                ...keywords,
                ...types,
                "print",
                "range",
                "len",
                "open",
              ].map((label) => ({
                label,
                kind: m.languages.CompletionItemKind.Keyword,
                insertText: label,
                range: {
                  startLineNumber: position.lineNumber,
                  endLineNumber: position.lineNumber,
                  startColumn: word.startColumn,
                  endColumn: word.endColumn,
                },
              })),
            };
          },
        });
        m.languages.register({ id: "ush", extensions: [".ush"] });
        m.languages.setMonarchTokensProvider("ush", {
          tokenizer: {
            root: [
              [/".*?"|'.*?'/, "string"],
              [
                /\b(list|goto|back|where|make|read|copy|move|delete|clear|help|exit)\b/,
                "keyword",
              ],
            ],
          },
        });
      }}
      onMount={(editor, m) => {
        onMount(editor, m);
        const node = editor.getDomNode();
        const checkInput = (event: Event) => {
          const text =
            event instanceof ClipboardEvent
              ? event.clipboardData?.getData("text/plain")
              : (event as InputEvent).data;
          if (text && byteLength(text) > MAX_SOURCE) {
            event.preventDefault();
            onLimit();
          }
        };
        node?.addEventListener("beforeinput", checkInput, true);
        node?.addEventListener("paste", checkInput, true);
        editor.onDidDispose(() => {
          node?.removeEventListener("beforeinput", checkInput, true);
          node?.removeEventListener("paste", checkInput, true);
        });
        editor.addAction({
          id: "unknown-run",
          label: "Run program",
          keybindings: [m.KeyMod.CtrlCmd | m.KeyCode.Enter],
          run: onRun,
        });
      }}
      onChange={(source) => onChange(source || "")}
      options={{
        editContext: false,
        readOnly,
        fontSize,
        fontFamily: '"Cascadia Code", "SFMono-Regular", Consolas, monospace',
        minimap: { enabled: false },
        scrollBeyondLastLine: false,
        automaticLayout: true,
        padding: { top: 20, bottom: 20 },
        tabSize: 4,
        insertSpaces: true,
        wordWrap: "on",
        renderLineHighlight: "gutter",
        smoothScrolling: false,
        accessibilitySupport: "on",
        bracketPairColorization: { enabled: true },
      }}
    />
  );
}
