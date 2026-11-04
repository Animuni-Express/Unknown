export const keywords = [
  "fn",
  "return",
  "if",
  "else",
  "for",
  "in",
  "while",
  "break",
  "continue",
  "import",
  "as",
  "not",
  "true",
  "false",
  "none",
];
export const types = ["int", "str", "bool"];
export function tokens(code: string) {
  const pattern =
    /(\/\*[\s\S]*?\*\/|\/\/[^\n]*|"(?:\\.|[^"\\])*"|'(?:\\.|[^'\\])*'|\b\d+\b|\b[A-Za-z_][A-Za-z_0-9]*\b)/g;
  let start = 0;
  const chunks: { text: string; kind: string }[] = [];
  for (const match of code.matchAll(pattern)) {
    const index = match.index!;
    if (index > start)
      chunks.push({ text: code.slice(start, index), kind: "" });
    const text = match[0];
    chunks.push({
      text,
      kind:
        text.startsWith("//") || text.startsWith("/*")
          ? "comment"
          : /^['"]/.test(text)
            ? "string"
            : /^\d/.test(text)
              ? "number"
              : keywords.includes(text)
                ? "keyword"
                : types.includes(text)
                  ? "type"
                  : "",
    });
    start = index + text.length;
  }
  if (start < code.length) chunks.push({ text: code.slice(start), kind: "" });
  return chunks;
}
