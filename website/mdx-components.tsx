import type { MDXComponents } from "mdx/types";
import { Example, Code } from "@/components/code";
import { slugify } from "@/lib/content-utils";
import { isValidElement, type ReactNode } from "react";
import Link from "next/link";
function Heading({ children, level }: { children: ReactNode; level: 2 | 3 }) {
  const Tag = level === 2 ? "h2" : "h3";
  return (
    <Tag id={slugify(String(children))}>
      <a href={`#${slugify(String(children))}`}>{children}</a>
    </Tag>
  );
}
export function useMDXComponents(components: MDXComponents): MDXComponents {
  return {
    a: ({ href, children, ...props }) =>
      href?.startsWith("/") ? (
        <Link href={href} {...props}>
          {children}
        </Link>
      ) : (
        <a href={href} {...props}>
          {children}
        </a>
      ),
    h2: (props) => <Heading level={2} {...props} />,
    h3: (props) => <Heading level={3} {...props} />,
    pre: (props) => {
      if (
        isValidElement<{ children?: string; className?: string }>(
          props.children,
        )
      ) {
        return (
          <Code
            source={String(props.children.props.children || "").replace(
              /\n$/,
              "",
            )}
            language={
              props.children.props.className?.replace("language-", "") || "text"
            }
          />
        );
      }
      return <pre {...props} />;
    },
    Example,
    Code,
    ...components,
  };
}
