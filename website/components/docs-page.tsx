import Link from "next/link";
import {
  ChevronLeft,
  ChevronRight,
  BookOpen,
  ExternalLink,
} from "lucide-react";
import { docs } from "@/lib/content.generated";
import { Search } from "./search";
export function DocsPage({ slug }: { slug: string }) {
  const index = docs.findIndex((d) => d.slug === slug);
  const doc = docs[index];
  const Page = doc.Component;
  const groups = [...new Set(docs.map((d) => d.group))];
  return (
    <main className="docs-shell">
      <aside className="docs-sidebar">
        <Search />
        <details className="docs-menu" open>
          <summary>Documentation navigation</summary>
          <nav aria-label="Documentation">
            {groups.map((group) => (
              <div className="nav-group" key={group}>
                <h2>{group}</h2>
                {docs
                  .filter((d) => d.group === group)
                  .map((d) => (
                    <Link
                      key={d.slug}
                      href={`/docs/${d.slug}/`}
                      aria-current={slug === d.slug ? "page" : undefined}
                    >
                      {d.title}
                    </Link>
                  ))}
              </div>
            ))}
          </nav>
        </details>
        <Link className="sidebar-play" href="/playground/">
          <BookOpen size={15} /> Learn by doing <ChevronRight size={14} />
        </Link>
      </aside>
      <article className="article">
        <div className="breadcrumb">
          Documentation <ChevronRight size={12} /> {doc.group}
        </div>
        <h1>{doc.title}</h1>
        <p className="article-lead">{doc.description}</p>
        <div className="article-content">
          <Page />
        </div>
        <div className="article-edit">
          <a
            href={`https://github.com/Animuni-Express/Unknown/blob/main/website/content/docs/${doc.slug}.mdx`}
          >
            Edit this page on GitHub <ExternalLink size={12} />
          </a>
          <span>Unknown 0.1.0</span>
        </div>
        <nav className="page-pagination" aria-label="Previous and next page">
          {docs[index - 1] ? (
            <Link href={`/docs/${docs[index - 1].slug}/`}>
              <ChevronLeft size={16} />
              <span>
                <small>Previous</small>
                {docs[index - 1].title}
              </span>
            </Link>
          ) : (
            <span />
          )}
          {docs[index + 1] && (
            <Link href={`/docs/${docs[index + 1].slug}/`}>
              <span>
                <small>Next</small>
                {docs[index + 1].title}
              </span>
              <ChevronRight size={16} />
            </Link>
          )}
        </nav>
      </article>
      <aside className="toc">
        <p>On this page</p>
        <nav aria-label="On this page">
          {doc.headings.map((h) => (
            <a key={h.id} href={`#${h.id}`}>
              {h.title}
            </a>
          ))}
        </nav>
        <div className="toc-note">
          <span className="status-dot" /> Runs in your browser
          <p>
            Real Rust. Real results.
            <br />
            No account required.
          </p>
          <Link href="/playground/">
            Open playground <ChevronRight size={12} />
          </Link>
        </div>
      </aside>
    </main>
  );
}
