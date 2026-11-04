import { describe, it, expect } from "vitest";
import {
  encodeSource,
  decodeSource,
  validFiles,
  diagnosticLink,
} from "../lib/runtime";
import { tokens } from "../lib/language";
describe("source sharing and workspace validation", () => {
  it("roundtrips Unicode and rejects oversized sources", () => {
    const source = 'print("Unknown 🦀 नमस्ते")';
    expect(decodeSource(encodeSource(source))).toBe(source);
    expect(() => encodeSource("x".repeat(65537))).toThrow();
    expect(() => decodeSource("!invalid")).toThrow();
  });
  it("rejects malformed and escaping file snapshots", () => {
    expect(
      validFiles([
        ["/", null],
        ["/main.unk", "print(1)"],
      ]),
    ).toBe(true);
    expect(validFiles([["/../host", "secret"]])).toBe(false);
    expect(validFiles([["C:\\file", "data"]])).toBe(false);
    expect(validFiles([["/a", 42]])).toBe(false);
  });
  it("rejects duplicate and noncanonical aliases", () => {
    expect(
      validFiles([
        ["/a", "1"],
        ["//a", "2"],
      ]),
    ).toBe(false);
    expect(
      validFiles([
        ["/a", "1"],
        ["/a", "2"],
      ]),
    ).toBe(false);
    expect(validFiles([["/./a", "1"]])).toBe(false);
  });
  it("preserves source through shared highlighting", () => {
    const source = '// fn\nfn add(int a) { print("hi", 2) }';
    expect(
      tokens(source)
        .map((t) => t.text)
        .join(""),
    ).toBe(source);
    expect(tokens(source).find((t) => t.text === "fn")?.kind).toBe("keyword");
  });
  it("links original diagnostic codes to documentation", () => {
    expect(
      diagnosticLink({
        severity: "error",
        code: "E2001",
        message: "undefined name",
      }),
    ).toContain("/scope/");
  });
});
