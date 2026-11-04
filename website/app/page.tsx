import Link from "next/link";
import {
  ArrowRight,
  BookOpen,
  Code2,
  Cpu,
  Layers,
  Terminal,
  GitBranch,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { Code } from "@/components/code";
import { examples } from "@/lib/examples";
export const metadata = {
  alternates: { canonical: `${process.env.NEXT_PUBLIC_BASE_PATH || ""}/` },
};
export default function Home() {
  return (
    <main>
      <section className="hero">
        <div className="hero-grid" aria-hidden="true" />
        <div className="hero-content">
          <Link
            href="/docs/getting-started/introduction/"
            className="release-pill"
          >
            <span className="status-dot" /> Unknown 0.1.0{" "}
            <span className="pill-divider" /> A language in the making{" "}
            <ArrowRight size={12} />
          </Link>
          <h1>
            Unknown<span className="hero-period">.</span>
          </h1>
          <h2>
            Readable like Python.
            <br />
            Control like C.
          </h2>
          <p>
            Explore Unknown, a general-purpose programming language designed to
            combine readable syntax with explicit control.
          </p>
          <div className="hero-actions">
            <Button asChild>
              <Link href="/playground/">
                <Terminal size={16} /> Start coding <ArrowRight size={15} />
              </Link>
            </Button>
            <Button asChild variant="secondary">
              <Link href="/docs/">
                <BookOpen size={16} /> Read documentation
              </Link>
            </Button>
          </div>
          <div className="hero-caption">
            <span className="status-dot" /> Runs locally in your browser.
            Powered by Rust + WebAssembly.
          </div>
        </div>
        <div className="hero-editor">
          <div className="window-bar">
            <span className="window-dots">
              <i />
              <i />
              <i />
            </span>
            <span>
              <Code2 size={13} /> hello.unk
            </span>
            <span className="window-language">Unknown</span>
          </div>
          <div className="hero-code">
            <Code source={examples.find((e) => e.id === "functions")!.source} />
          </div>
          <div className="hero-result">
            <div>
              <span className="status-dot" /> Output <span>Example result</span>
            </div>
            <pre>30</pre>
          </div>
          <Link className="hero-try" href="/playground/?example=functions">
            Make it your own <ArrowRight size={14} />
          </Link>
        </div>
      </section>
      <div className="capability-strip">
        <span>
          <Cpu size={16} /> Rust interpreter
        </span>
        <span>
          <Layers size={16} /> WebAssembly runtime
        </span>
        <span>
          <Terminal size={16} /> Unknown + USH
        </span>
        <span>
          <GitBranch size={16} /> Open source
        </span>
      </div>
      <section className="home-section">
        <div className="section-intro">
          <div>
            <p className="eyebrow">Less ceremony. More clarity.</p>
            <h2>
              A small language.
              <br />A place to explore it.
            </h2>
          </div>
          <p>
            Learn the syntax, follow a working example, and change it yourself.
            Documentation and execution, side by side.
          </p>
        </div>
        <div className="feature-grid">
          <Link href="/docs/language/variables/">
            <Code2 size={23} />
            <h3>Read it. Understand it.</h3>
            <p>
              Familiar expressions, clear control flow, and a choice of braces
              or indentation. Start with the features that work today.
            </p>
            <span>
              Explore the language <ArrowRight size={14} />
            </span>
          </Link>
          <Link href="/playground/">
            <Terminal size={23} />
            <h3>Try the real thing.</h3>
            <p>
              Edit Unknown and USH, run the Rust cores in a worker, and work
              with files in an isolated browser workspace.
            </p>
            <span>
              Open the playground <ArrowRight size={14} />
            </span>
          </Link>
          <Link href="/docs/errors/">
            <BookOpen size={23} />
            <h3>Find your next step.</h3>
            <p>
              Original diagnostics, source highlights, and documentation that
              explains the language without hiding its limits.
            </p>
            <span>
              Understand diagnostics <ArrowRight size={14} />
            </span>
          </Link>
        </div>
      </section>
      <section className="start-section">
        <div>
          <h2>Your first program is one line away.</h2>
          <p>
            No setup in the browser. Build from source when you need native
            capabilities.
          </p>
          <Link href="/docs/getting-started/installation/">
            Native installation instructions <ArrowRight size={14} />
          </Link>
        </div>
        <Code source={'print("Hello, Unknown!")'} runnable />
      </section>
      <footer className="footer">
        <Link className="brand" href="/">
          u_ <span>Unknown Trying</span>
        </Link>
        <span>Built to be understood.</span>
        <a href="https://github.com/Animuni-Express/Unknown">
          GitHub <ArrowRight size={13} />
        </a>
      </footer>
    </main>
  );
}
