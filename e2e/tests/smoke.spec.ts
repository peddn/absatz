import { expect, test } from "@playwright/test";

test("app starts and renders the sample document with style-set CSS", async ({ page }) => {
  const errors: string[] = [];
  page.on("pageerror", (e) => errors.push(e.message));

  await page.goto("./?doc=1");
  await expect(page.getByRole("heading", { name: "Absatz", level: 1 })).toBeVisible();

  const doc = page.locator("article.absatz-doc");
  await expect(doc.locator("h1.p-h1").first()).toContainText("Kapitel 1");
  expect(await doc.locator("p.p-body").count()).toBeGreaterThan(20);

  // CSS comes from the style set tokens (StyleSet::to_css).
  const emphasis = doc.locator(".c-emphasis").first();
  await expect(emphasis).toHaveCSS("font-style", "italic");
  await expect(doc.locator("h2.p-h2").first()).toHaveCSS("color", "rgb(31, 78, 121)");

  expect(errors).toEqual([]);
});

test("typesetting worker produces a page count @smoke", async ({ page }) => {
  await page.goto("./?bench=1");
  await expect(page.locator("#bench-result")).toContainText("PDF-Export", { timeout: 50_000 });
  await expect(page.locator("#bench-result")).toContainText(/Seiten: \d+/);
});
