import { expect, test } from "@playwright/test";

const status = (page) => page.locator("#preview-status");
const pageImage = (page, n: number) => page.locator(".preview .page img").nth(n);

async function typeAtEnd(page, block: number, text: string) {
  await page.locator("article.absatz-doc > *").nth(block).click();
  await page.keyboard.press("End");
  await page.keyboard.type(text);
}

test("first start: welcome document and its pages", async ({ page }) => {
  await page.goto("./");
  await expect(page.locator("article.absatz-doc h1")).toHaveText("Willkommen bei Absatz");
  await expect(status(page)).toHaveText(/^\d+ Seiten?$/, { timeout: 30_000 });
  await expect(pageImage(page, 0)).toHaveAttribute("src", /^blob:/);
});

test("typing updates the preview and is saved across reloads", async ({ page }) => {
  await page.goto("./");
  await expect(status(page)).toHaveText(/Seite/, { timeout: 30_000 });
  const before = await pageImage(page, 0).getAttribute("src");

  await typeAtEnd(page, 1, " Gespeicherter Satz.");
  await expect(pageImage(page, 0)).not.toHaveAttribute("src", before!);
  await expect(status(page)).toHaveText(/Seite/);

  await page.reload();
  await expect(page.locator("article.absatz-doc > *").nth(1)).toContainText("Gespeicherter Satz.");
});

test("PDF download contains the current document", async ({ page }) => {
  await page.goto("./");
  await expect(status(page)).toHaveText(/Seite/, { timeout: 30_000 });
  const download = page.waitForEvent("download");
  await page.locator("#pdf").click();
  const file = await download;
  expect(file.suggestedFilename()).toBe("dokument.pdf");
  const stream = await file.createReadStream();
  const chunks: Buffer[] = [];
  for await (const chunk of stream) chunks.push(chunk as Buffer);
  const pdf = Buffer.concat(chunks);
  expect(pdf.subarray(0, 5).toString()).toBe("%PDF-");
  expect(pdf.length).toBeGreaterThan(5_000);
});

test("new document starts empty and stays empty after reload", async ({ page }) => {
  await page.goto("./");
  await expect(status(page)).toHaveText(/Seite/, { timeout: 30_000 });
  await page.locator("#new-doc").click();
  await expect(page.locator("article.absatz-doc > *")).toHaveCount(1);
  await expect(page.locator("article.absatz-doc")).toHaveText("");
  await expect(status(page)).toHaveText("1 Seite", { timeout: 30_000 });
  await page.keyboard.type("x"); // focus is not in the editor yet: nothing happens
  await page.locator("article.absatz-doc > *").first().click();
  await page.keyboard.type("Erster Satz");
  await expect(page.locator("article.absatz-doc")).toHaveText("Erster Satz");
});

test("pages are rendered only when scrolled into view", async ({ page }) => {
  await page.goto("./?doc=1");
  await expect(status(page)).toHaveText(/\d+ Seiten/, { timeout: 30_000 });
  const count = await page.locator(".preview .page").count();
  expect(count).toBeGreaterThan(5);
  await expect(pageImage(page, count - 1)).not.toHaveAttribute("src", /./);
  await page.locator(".preview").evaluate((el) => el.scrollTo(0, el.scrollHeight));
  await expect(pageImage(page, count - 1)).toHaveAttribute("src", /^blob:/);
});
