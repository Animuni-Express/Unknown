"use client";
import { useState, useEffect, useRef } from "react";
import Link from "next/link";
import { Search as SearchIcon, X, FileText } from "lucide-react";
import index from "@/lib/search.generated.json";
export function Search() {
  const [query, setQuery] = useState("");
  const [open, setOpen] = useState(false);
  const input = useRef<HTMLInputElement>(null);
  useEffect(() => {
    function key(e: KeyboardEvent) {
      if ((e.ctrlKey || e.metaKey) && e.key === "k") {
        e.preventDefault();
        setOpen(true);
        input.current?.focus();
      }
      if (e.key === "Escape") setOpen(false);
    }
    document.addEventListener("keydown", key);
    return () => document.removeEventListener("keydown", key);
  }, []);
  const terms = query.toLowerCase().trim().split(/\s+/);
  const results = query.trim()
    ? index
        .filter((p) =>
          terms.every((t) =>
            `${p.title} ${p.heading} ${p.text}`.toLowerCase().includes(t),
          ),
        )
        .slice(0, 12)
    : [];
  return (
    <div className="search">
      <div className="search-input">
        <SearchIcon size={15} />
        <input
          ref={input}
          aria-label="Search documentation"
          placeholder="Search documentation…"
          value={query}
          onFocus={() => setOpen(true)}
          onChange={(e) => {
            setQuery(e.target.value);
            setOpen(true);
          }}
        />
        <kbd>⌘ K</kbd>
        {open && (
          <button
            className="icon-button"
            aria-label="Close search"
            onClick={() => setOpen(false)}
          >
            <X size={14} />
          </button>
        )}
      </div>
      {open && query.trim() && (
        <div
          className="search-results"
          role="region"
          aria-label="Search results"
        >
          {results.length ? (
            results.map((p) => (
              <Link key={p.href} href={p.href} onClick={() => setOpen(false)}>
                <FileText size={15} />
                <span>
                  <strong>{p.title}</strong>
                  <small>{p.heading}</small>
                </span>
              </Link>
            ))
          ) : (
            <p>No matching sections. Try a keyword or error code.</p>
          )}
        </div>
      )}
    </div>
  );
}
