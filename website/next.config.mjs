import createMDX from "@next/mdx";
const basePath = process.env.NEXT_PUBLIC_BASE_PATH || "";
if (
  basePath &&
  (!basePath.startsWith("/") ||
    basePath.endsWith("/") ||
    !/^\/[A-Za-z0-9_/-]+$/.test(basePath))
)
  throw new Error("Invalid NEXT_PUBLIC_BASE_PATH");
export default createMDX({ options: { remarkPlugins: ["remark-gfm"] } })({
  output: "export",
  trailingSlash: true,
  basePath,
  assetPrefix: basePath,
  images: { unoptimized: true },
  pageExtensions: ["ts", "tsx", "md", "mdx"],
  turbopack: { root: import.meta.dirname },
});
