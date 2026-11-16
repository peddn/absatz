// Measures how long Chromium takes to underline a misspelled word.
//
//   DICT=/path/en-US-10-1.bdic CHROME_PATH=/usr/bin/google-chrome \
//     node tools/spellcheck-latency.mjs <url | plain>
//
// `plain` measures an ordinary contenteditable for comparison (≈40 ms).
// Needs a Chrome dictionary file (copy it from a Chrome profile's
// `Dictionaries` folder); headless Chrome cannot download one in a sandbox.
// Not part of the test suite because the dictionary is not in the repo.
//
// For an app URL the tool types a 60-paragraph English document, reloads
// (like opening a saved document), clicks into paragraph BLOCK (default 30)
// and types misspelled words. Optional: PRE="Enter,Home" presses keys first,
// EVAL="<js>" runs in the page first, SHOT=<png> saves a screenshot.
// Measured 2026-11: plain 40 ms, Absatz 45–65 ms (also after Enter, style
// change, joining paragraphs).
import { chromium } from "@playwright/test";
import fs from "fs";
const target = process.argv[2];
const dir = fs.mkdtempSync("/tmp/absatz-spell-");
fs.mkdirSync(dir + "/Default", { recursive: true });
fs.mkdirSync(dir + "/Dictionaries", { recursive: true });
fs.copyFileSync(process.env.DICT, dir + "/Dictionaries/en-US-10-1.bdic");
fs.writeFileSync(dir + "/Default/Preferences", JSON.stringify({ browser: { enable_spellchecking: true }, spellcheck: { dictionaries: ["en-US"], use_spelling_service: false } }));
const ctx = await chromium.launchPersistentContext(dir, { executablePath: process.env.CHROME_PATH, viewport: { width: 900, height: 500 } });
const page = ctx.pages()[0] ?? await ctx.newPage();
let sel;
if (target === "plain") {
  await page.setContent(`<div id=e contenteditable spellcheck=true style="font:20px serif;padding:20px"><p>Hello world.</p><p>Second paragraph here.</p></div>`);
  sel = "#e p >> nth=1";
} else {
  // A long document of correct English, so only the typed words are wrong.
  await page.goto(target);
  await page.locator("#new-doc").click();
  sel = "article.absatz-doc > p >> nth=-1";
  await page.locator(sel).click();
  // Typed, not pasted, so the tool also works on builds without paste.
  for (let i = 0; i < 60; i++) {
    await page.keyboard.type("Hello world this is a paragraph.");
    await page.keyboard.press("Enter");
  }
  await page.locator("article.absatz-doc > p >> nth=59").waitFor();
  // Reload so the measurement starts like a user opening a saved document.
  await page.waitForTimeout(1500);
  await page.reload();
  await page.locator("article.absatz-doc > p >> nth=59").waitFor();
  if (process.env.EVAL) await page.evaluate(process.env.EVAL);
  sel = `article.absatz-doc > p >> nth=${process.env.BLOCK ?? 30}`;
  await page.locator(sel).scrollIntoViewIfNeeded();
  await page.waitForTimeout(3000);
}
await page.locator(sel).click();
await page.keyboard.press("End");
const clip = { x: 0, y: 0, width: 900, height: 500 };
async function red() {
  const b64 = (await page.screenshot({ clip })).toString("base64");
  return await page.evaluate(async (b64) => {
    const img = new Image(); img.src = "data:image/png;base64," + b64; await img.decode();
    const c = new OffscreenCanvas(img.width, img.height); const g = c.getContext("2d"); g.drawImage(img, 0, 0);
    const d = g.getImageData(0, 0, img.width, img.height).data; let n = 0;
    for (let i = 0; i < d.length; i += 4) if (d[i] > 200 && d[i + 1] < 90 && d[i + 2] < 90) n++;
    return n;
  }, b64);
}
for (const key of (process.env.PRE ?? "").split(",").filter(Boolean)) await page.keyboard.press(key);
const results = [];
for (const word of [" wrongg", " misstakke", " speling", " korrect", " abzurd"]) {
  const base = await red();
  await page.keyboard.type(word + " ", { delay: 40 });
  const t0 = Date.now(); let t = null;
  while (Date.now() - t0 < 6000) { if ((await red()) > base + 5) { t = Date.now() - t0; break; } await page.waitForTimeout(30); }
  results.push(t === null ? "nie" : t + " ms");
}
if (process.env.SHOT) await page.screenshot({ path: process.env.SHOT });
console.log(target, "→", results.join(", "));
await ctx.close();
