"use client";
import Link from "next/link";
import { usePathname } from "next/navigation";
import { useEffect, useState } from "react";
import { Moon, Sun, Github, Terminal, ArrowUpRight } from "lucide-react";
export function Header() {
  const path = usePathname();
  const [light, setLight] = useState(false);
  useEffect(() => {
    setLight(document.documentElement.dataset.theme === "light");
  }, []);
  function toggle() {
    const next = !light;
    setLight(next);
    document.documentElement.dataset.theme = next ? "light" : "dark";
    try {
      localStorage.setItem("unknown-theme", next ? "light" : "dark");
    } catch {}
  }
  return (
    <header className="header">
      <div className="header-inner">
        <Link className="brand" href="/" aria-label="Unknown home">
          <span className="logo">
            u<span>_</span>
          </span>
          <span>
            Unknown<span className="brand-suffix"> / trying</span>
          </span>
        </Link>
        <nav aria-label="Main navigation">
          <Link
            aria-current={path.startsWith("/docs") ? "page" : undefined}
            href="/docs/"
          >
            Docs
          </Link>
          <Link
            aria-current={path.startsWith("/playground") ? "page" : undefined}
            href="/playground/"
          >
            Playground
          </Link>
          <Link className="desktop-link" href="/about/">
            About
          </Link>
        </nav>
        <div className="header-actions">
          <a
            className="icon-button"
            aria-label="GitHub repository"
            href="https://github.com/Animuni-Express/Unknown"
          >
            <Github size={18} />
          </a>
          <button
            className="icon-button"
            onClick={toggle}
            aria-label={light ? "Use dark theme" : "Use light theme"}
          >
            {light ? <Moon size={17} /> : <Sun size={17} />}
          </button>
          <Link className="header-code" href="/playground/">
            <Terminal size={14} /> Start coding <ArrowUpRight size={13} />
          </Link>
        </div>
      </div>
    </header>
  );
}
