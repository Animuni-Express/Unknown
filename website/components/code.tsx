"use client";
import Link from "next/link";
import { useState } from "react";
import { Copy, Check, Play } from "lucide-react";
import { tokens } from "@/lib/language";
import { examples } from "@/lib/examples";
export function Code({
  source,
  language = "unk",
  runnable = false,
  output,
}: {
  source: string;
  language?: string;
  runnable?: boolean;
  output?: string;
}) {
  const [copied, setCopied] = useState(false);
  const [error, setError] = useState("");
  return (
    <div className="code-block">
      <div className="code-toolbar">
        <span>{language === "unk" ? "Unknown" : language}</span>
        <div>
          <button
            aria-label="Copy code"
            onClick={async () => {
              try {
                await navigator.clipboard.writeText(source);
                setCopied(true);
                setTimeout(() => setCopied(false), 1500);
              } catch {
                setError("Clipboard unavailable. Select and copy the code.");
              }
            }}
          >
            {copied ? <Check size={14} /> : <Copy size={14} />}{" "}
            {copied ? "Copied" : "Copy"}
          </button>
          {runnable && (
            <Link
              href={`/playground/?example=${encodeURIComponent(examples.find((e) => e.source === source)?.id || "")}`}
            >
              <Play size={13} /> Try it
            </Link>
          )}
        </div>
      </div>
      <pre>
        <code>
          {tokens(source).map((t, i) => (
            <span key={i} className={t.kind ? `syntax-${t.kind}` : undefined}>
              {t.text}
            </span>
          ))}
        </code>
      </pre>
      {output !== undefined && (
        <div className="expected">
          <span>Expected output</span>
          <pre>{output}</pre>
        </div>
      )}
      {error && <p role="status">{error}</p>}
    </div>
  );
}
export function Example({ id }: { id: string }) {
  const example = examples.find((e) => e.id === id);
  if (!example) throw new Error(`Unknown example ${id}`);
  return <Code source={example.source} runnable output={example.output} />;
}
