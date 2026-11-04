import { notFound } from "next/navigation";
import { docs } from "@/lib/content.generated";
import { DocsPage } from "@/components/docs-page";
export const dynamicParams = false;
export function generateStaticParams() {
  return docs.map((d) => ({ slug: d.slug.split("/") }));
}
export async function generateMetadata({
  params,
}: {
  params: Promise<{ slug: string[] }>;
}) {
  const { slug } = await params;
  const doc = docs.find((d) => d.slug === slug.join("/"));
  return {
    title: doc?.title,
    description: doc?.description,
    alternates: {
      canonical: `${process.env.NEXT_PUBLIC_BASE_PATH || ""}/docs/${slug.join("/")}/`,
    },
  };
}
export default async function Page({
  params,
}: {
  params: Promise<{ slug: string[] }>;
}) {
  const { slug } = await params;
  const path = slug.join("/");
  if (!docs.some((d) => d.slug === path)) notFound();
  return <DocsPage slug={path} />;
}
