import Link from "next/link";
export default function NotFound() {
  return (
    <main className="article standalone">
      <p className="eyebrow">404</p>
      <h1>This page is still unknown.</h1>
      <p>That address does not match a documentation page.</p>
      <Link className="button button-primary" href="/docs/">
        Browse documentation
      </Link>
    </main>
  );
}
