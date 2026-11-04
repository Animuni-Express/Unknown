import Link from "next/link";
export const metadata = {
  title: "About",
  alternates: {
    canonical: `${process.env.NEXT_PUBLIC_BASE_PATH || ""}/about/`,
  },
};
export default function About() {
  return (
    <main className="article standalone">
      <div className="breadcrumb">The project</div>
      <h1>
        Making the unknown
        <br />
        understandable.
      </h1>
      <p className="article-lead">
        A language to explore. A workspace to experiment in.
      </p>
      <div className="article-content">
        <h2>Unknown Trying</h2>
        <p>
          This is the documentation and browser playground for Unknown. It runs
          the actual Rust interpreter through WebAssembly, alongside the Rust
          core of USH. Both runtimes use one virtual filesystem.
        </p>
        <h2>Early, and explicit about it</h2>
        <p>
          Unknown is version 0.1.0. Its design aims to combine readable syntax
          with explicit control. The current implementation is a tree-walking
          interpreter; claims about speed, safety, or low-level capabilities
          need evidence. The documentation separates working features from
          planned ones.
        </p>
        <h2>One source, two hosts</h2>
        <p>
          The same static Next.js export supports ChatGPT Sites and GitHub
          Pages. Programs execute in your browser, without a backend, accounts,
          or a paid execution service. Workspace files stay in local storage
          unless you choose to download or share source.
        </p>
        <p>
          <Link href="/docs/getting-started/introduction/">
            Read the documentation
          </Link>{" "}
          or{" "}
          <a href="https://github.com/Animuni-Express/Unknown">
            explore the source on GitHub
          </a>
          .
        </p>
      </div>
    </main>
  );
}
