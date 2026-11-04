import { readdirSync, readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
const root = resolve(import.meta.dirname, "..");
const files = readdirSync(resolve(root, "content/docs"), { recursive: true })
  .filter((f) => f.endsWith(".mdx"))
  .sort();
const pages = files
  .map((file, index) => {
    const text = readFileSync(resolve(root, "content/docs", file), "utf8");
    const match = text.match(/^export const meta = (.+);$/m);
    if (!match) throw new Error(`Missing metadata: ${file}`);
    const meta = JSON.parse(match[1]);
    const headings = [...text.matchAll(/^## (.+)$/gm)].map((m) => ({
      title: m[1],
      id: m[1]
        .toLowerCase()
        .replace(/[^a-z0-9\s-]/g, "")
        .trim()
        .replace(/\s+/g, "-"),
    }));
    return {
      file,
      index,
      ...meta,
      slug: file.replace(/\.mdx$/, "").replaceAll("\\", "/"),
      headings,
      text: text
        .replace(/^export const meta.+\n/m, "")
        .replace(/<[^>]+>/g, " ")
        .replace(/[`#*]/g, ""),
    };
  })
  .sort((a, b) => a.order - b.order);
writeFileSync(
  resolve(root, "lib/content.generated.ts"),
  files
    .map(
      (f, i) =>
        `import Page${i} from '../content/docs/${f.replaceAll("\\", "/")}';`,
    )
    .join("\n") +
    "\nexport const docs = [\n" +
    pages
      .map(
        ({ text, file, index, ...p }) =>
          `{...${JSON.stringify(p)},Component:Page${index}}`,
      )
      .join(",\n") +
    "\n];\n",
);
const search = pages.flatMap((p) => {
  const parts = readFileSync(
    resolve(root, "content/docs", p.file),
    "utf8",
  ).split(/^## /m);
  return [
    {
      title: p.title,
      heading: "Overview",
      href: `/docs/${p.slug}/`,
      text: p.description + " " + p.title,
    },
    ...parts
      .slice(1)
      .map((part, i) => ({
        title: p.title,
        heading: p.headings[i].title,
        href: `/docs/${p.slug}/#${p.headings[i].id}`,
        text: part.replace(/<[^>]+>/g, " ").replace(/[`#*]/g, ""),
      })),
  ];
});
writeFileSync(
  resolve(root, "lib/search.generated.json"),
  JSON.stringify(search),
);
console.log(`Indexed ${pages.length} MDX pages and ${search.length} sections.`);
