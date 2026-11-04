import { DocsPage } from "@/components/docs-page";
export const metadata = {
  title: "Documentation",
  alternates: { canonical: `${process.env.NEXT_PUBLIC_BASE_PATH || ""}/docs/` },
};
export default function Page() {
  return <DocsPage slug="getting-started/introduction" />;
}
