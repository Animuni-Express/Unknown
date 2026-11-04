import type { MetadataRoute } from "next";
import { docs } from "@/lib/content.generated";
export const dynamic = "force-static";
export default function sitemap(): MetadataRoute.Sitemap {
  const origin =
    (
      process.env.NEXT_PUBLIC_SITE_URL || "https://animuni-express.github.io"
    ).replace(/\/$/, "") + (process.env.NEXT_PUBLIC_BASE_PATH || "");
  return [
    "",
    "docs/",
    "playground/",
    "about/",
    ...docs.map((d) => `docs/${d.slug}/`),
  ].map((p) => ({ url: `${origin}/${p}` }));
}
