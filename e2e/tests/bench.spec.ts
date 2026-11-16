import { expect, test } from "@playwright/test";

// Typesetting benchmark in Chromium. Run with: just bench-web [chapters]
test("typesetting benchmark @bench", async ({ page }, testInfo) => {
  test.setTimeout(10 * 60_000);
  const chapters = process.env.BENCH_CHAPTERS ?? "10";
  await page.goto(`./?bench=${chapters}`);
  const result = page.locator("#bench-result");
  await expect(result).toContainText("PDF-Export", { timeout: 9 * 60_000 });
  const text = (await result.textContent()) ?? "";
  console.log(`\nChromium ${testInfo.project.name}, ${chapters} Kapitel\n${text}`);
  await testInfo.attach("bench.txt", { body: text, contentType: "text/plain" });
});
