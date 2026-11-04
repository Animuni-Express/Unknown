import type { Metadata } from "next";
import { Header } from "@/components/header";
import "./globals.css";
const origin =
  process.env.NEXT_PUBLIC_SITE_URL || "https://animuni-express.github.io";
export const metadata: Metadata = {
  metadataBase: new URL(origin),
  title: {
    default: "Unknown Trying — Learn. Write. Run.",
    template: "%s | Unknown Trying",
  },
  description:
    "Learn Unknown and run real Unknown and USH programs in your browser. Documentation and a WebAssembly playground, powered by Rust.",
};
export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en" data-theme="dark" suppressHydrationWarning>
      <head>
        <script
          dangerouslySetInnerHTML={{
            __html:
              "try{document.documentElement.dataset.theme=localStorage.getItem('unknown-theme')==='light'?'light':'dark'}catch{}",
          }}
        />
      </head>
      <body>
        <a className="skip-link" href="#main-content">
          Skip to content
        </a>
        <Header />
        <div id="main-content">{children}</div>
      </body>
    </html>
  );
}
