import { test, expect, type Page } from "@playwright/test";
const prefix = process.env.TEST_BASE_PATH || "";
const route = (path: string) => `${prefix}${path}`;
async function editorReady(page: Page) {
  await expect(page.locator(".monaco-editor").first()).toBeVisible();
  await expect(page.getByRole("button", { name: /^Run/ })).toBeEnabled();
  await expect(page.getByRole('button',{name:'Copy source',exact:true})).toBeVisible();
  await expect(page.getByRole('button',{name:'Share source',exact:true})).toBeVisible();
}
async function setCode(page: Page, source: string) {
  const input = page.locator(".monaco-editor textarea").first();
  await input.focus();
  await page.keyboard.press("ControlOrMeta+A");
  await page.keyboard.type(source);
}
test("static documentation, section search, and Try it integrate with Monaco", async ({
  page,
}) => {
  const errors: string[] = [];
  page.on("pageerror", (e) => errors.push(e.message));
  await page.goto(route("/"));
  await expect(
    page.getByRole("heading", { name: "Unknown.", exact: true }),
  ).toBeVisible();
  await page.screenshot({
    path: test.info().outputPath("homepage.png"),
    fullPage: true,
  });
  await page.goto(route("/docs/language/functions/"));
  await expect(
    page.getByRole("heading", { name: "Functions", exact: true }),
  ).toBeVisible();
  const search = page.getByRole("textbox", { name: "Search documentation" });
  await search.fill("E2001");
  await expect(
    page.getByRole("region", { name: "Search results" }),
  ).toContainText("Errors and diagnostics");
  await page.getByRole("button", { name: "Close search" }).click();
  await page.getByRole("link", { name: "Try it", exact: true }).click();
  await editorReady(page);
  await expect(page.getByTestId("stdout")).toHaveCount(0);
  await page.getByRole("button", { name: /^Run/ }).click();
  await expect(page.getByTestId("stdout")).toHaveText("30\n");
  expect(errors).toEqual([]);
});
test("both interpreters share files, diagnose, stop, recover, and persist", async ({
  page,
}) => {
  await page.goto(route("/playground/"));
  await editorReady(page);
  await setCode(
    page,
    'import "file"\nfile.write("shared.txt", "from Rust")\nprint(file.read("shared.txt"))',
  );
  await page.getByRole("button", { name: /^Run/ }).click();
  await expect(page.getByTestId("stdout")).toHaveText("from Rust\n");
  await page.getByRole("button", { name: "session.ush", exact: true }).click();
  await setCode(page, "read shared.txt");
  await page.getByRole("button", { name: /^Run/ }).click();
  await expect(page.getByTestId("stdout")).toHaveText("from Rust");
  await page.getByRole("button", { name: "main.unk", exact: true }).click();
  await setCode(page, "print(missing)");
  await page.getByRole("button", { name: /^Run/ }).click();
  await expect(page.getByTestId("stderr")).toContainText("E2001");
  await expect(page.getByRole("button", { name: /Go to error/ })).toBeVisible();
  await setCode(page, "while (true):\n    x = 1");
  await page.getByRole("button", { name: /^Run/ }).click();
  await page.getByRole("button", { name: "Stop", exact: true }).click();
  await expect(page.locator(".statusbar")).toContainText("Stopped");
  await setCode(page, 'print("recovered")');
  await page.getByRole("button", { name: /^Run/ }).click();
  await expect(page.getByTestId("stdout")).toHaveText("recovered\n");
  await page.reload();
  await editorReady(page);
  await page.getByRole("button", { name: /^Run/ }).click();
  await expect(page.getByTestId("stdout")).toHaveText("recovered\n");
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
  await page.screenshot({
    path: test.info().outputPath("playground.png"),
    fullPage: true,
  });
});
test("shared Unicode source waits for user and timeout permits a fresh run", async ({
  page,
}) => {
  const code = Buffer.from('print("shared 🦀")').toString("base64url");
  await page.goto(route(`/playground/#code=${code}&runtime=unk`));
  await editorReady(page);
  await expect(page.getByTestId("stdout")).toHaveCount(0);
  await page.getByRole("button", { name: /^Run/ }).click();
  await expect(page.getByTestId("stdout")).toHaveText("shared 🦀\n");
  await setCode(page, "while (true):\n    x = 1");
  await page.getByRole("button", { name: /^Run/ }).click();
  await expect(page.locator(".statusbar")).toContainText("Timed out", {
    timeout: 10000,
  });
  await setCode(page, "print(42)");
  await page.getByRole("button", { name: /^Run/ }).click();
  await expect(page.getByTestId("stdout")).toHaveText("42\n");
});
test("oversized edits preserve source and saved workspace", async ({
  page,
}) => {
  await page.goto(route("/playground/"));
  await editorReady(page);
  await setCode(page, 'print("keep me")');
  await page.locator(".monaco-editor textarea").first().focus();
  await page.keyboard.press("ControlOrMeta+A");
  await page.keyboard.insertText("x".repeat(65537));
  await expect(page.getByRole("status")).toContainText("Edit rejected");
  await page.reload();
  await editorReady(page);
  await page.getByRole("button", { name: /^Run/ }).click();
  await expect(page.getByTestId("stdout")).toHaveText("keep me\n");
});
