import { expect, type Page, test } from "@playwright/test";

const blocks = (page: Page) => page.locator("article.absatz-doc > *");

/** Focus the editor and put the caret at the end of block `index` (or select a range). */
async function caret(page: Page, index: number, offset: "end" | number = "end", toIndex?: number, toOffset?: number) {
  await page.evaluate(
    ([index, offset, toIndex, toOffset]) => {
      const root = document.querySelector("article.absatz-doc") as HTMLElement;
      root.focus();
      const point = (i: number, o: "end" | number): [Node, number] => {
        const el = root.children[i];
        const walker = document.createTreeWalker(el, NodeFilter.SHOW_TEXT);
        const texts: Text[] = [];
        while (walker.nextNode()) texts.push(walker.currentNode as Text);
        if (o === "end") {
          const last = texts[texts.length - 1];
          return [last, last.length];
        }
        for (const t of texts) {
          if (o <= t.length) return [t, o];
          o -= t.length;
        }
        return [el, el.childNodes.length];
      };
      const [an, ao] = point(index as number, offset as "end" | number);
      const [fnode, fo] =
        toIndex === undefined ? [an, ao] : point(toIndex as number, toOffset as number);
      getSelection()!.setBaseAndExtent(an, ao, fnode, fo);
    },
    [index, offset, toIndex, toOffset] as const,
  );
  // selectionchange is async; let the editor pick it up.
  await page.waitForTimeout(50);
}

test.beforeEach(async ({ page }) => {
  await page.goto("./");
  await expect(blocks(page).nth(2)).toBeVisible();
});

// Sample document: 0 outline, 1 h1 "Kapitel 1: …", 2 h2 "Abschnitt 1.1", 3 body, 4 body, …

test("typing goes through EditContext into the model", async ({ page }) => {
  const before = await blocks(page).nth(3).textContent();
  await caret(page, 3);
  await page.keyboard.type(" Hallo Welt");
  await expect(blocks(page).nth(3)).toHaveText(`${before} Hallo Welt`);
  await expect(page.locator("article.absatz-doc")).not.toHaveAttribute("contenteditable");
});

test("arrow keys move the caret natively", async ({ page }) => {
  await caret(page, 2); // "Abschnitt 1.1"
  await page.keyboard.press("ArrowLeft");
  await page.keyboard.press("ArrowLeft");
  await page.keyboard.type("|");
  await expect(blocks(page).nth(2)).toHaveText("Abschnitt 1|.1");
});

test("Enter splits; after a heading comes body text", async ({ page }) => {
  await caret(page, 2, 9); // "Abschnitt| 1.1"
  await page.keyboard.press("Enter");
  await expect(blocks(page).nth(2)).toHaveText("Abschnitt");
  await expect(blocks(page).nth(3)).toHaveText(" 1.1");
  await expect(blocks(page).nth(3)).toHaveClass("p-body");
  await page.keyboard.type("X");
  await expect(blocks(page).nth(3)).toHaveText("X 1.1");
});

test("Backspace at block start merges with the previous block", async ({ page }) => {
  const second = (await blocks(page).nth(4).textContent()) ?? "";
  const first = (await blocks(page).nth(3).textContent()) ?? "";
  const count = await blocks(page).count();
  await caret(page, 4, 0);
  await page.keyboard.press("Backspace");
  await expect(blocks(page)).toHaveCount(count - 1);
  await expect(blocks(page).nth(3)).toHaveText(first + second);
});

test("typing over a selection across blocks replaces it", async ({ page }) => {
  await caret(page, 2, 3, 3, 5); // "Abs|chnitt 1.1" … "Tabel|le …"
  const rest = ((await blocks(page).nth(3).textContent()) ?? "").slice(5);
  await page.keyboard.type("X");
  await expect(blocks(page).nth(2)).toHaveText(`AbsX${rest}`);
});

test("Ctrl+Alt+1 applies heading style", async ({ page }) => {
  await caret(page, 3, 0);
  await page.keyboard.press("Control+Alt+Digit1");
  await expect(page.locator("article.absatz-doc > h1.p-h1")).toHaveCount(2);
});

test("IME composition (CDP) commits text", async ({ page }) => {
  await caret(page, 2); // end of "Abschnitt 1.1"
  const cdp = await page.context().newCDPSession(page);
  await cdp.send("Input.imeSetComposition", { text: "にほ", selectionStart: 2, selectionEnd: 2 });
  await expect(blocks(page).nth(2)).toHaveText("Abschnitt 1.1にほ");
  await cdp.send("Input.imeSetComposition", { text: "にほん", selectionStart: 3, selectionEnd: 3 });
  await cdp.send("Input.insertText", { text: "日本" });
  await expect(blocks(page).nth(2)).toHaveText("Abschnitt 1.1日本");
  await page.keyboard.type("!");
  await expect(blocks(page).nth(2)).toHaveText("Abschnitt 1.1日本!");
});

test("undo and redo restore text, blocks and caret", async ({ page }) => {
  await caret(page, 2); // end of "Abschnitt 1.1"
  await page.keyboard.type(" neu");
  await page.keyboard.press("Enter");
  await page.keyboard.type("Zeile");
  await expect(blocks(page).nth(3)).toHaveText("Zeile");

  await page.keyboard.press("Control+z"); // typing "Zeile"
  await expect(blocks(page).nth(3)).toHaveText("");
  await page.keyboard.press("Control+z"); // Enter
  await expect(blocks(page).nth(2)).toHaveText("Abschnitt 1.1 neu");
  await expect(blocks(page).nth(3)).toHaveClass("p-body");
  await page.keyboard.press("Control+z"); // typing " neu"
  await expect(blocks(page).nth(2)).toHaveText("Abschnitt 1.1");

  await page.keyboard.press("Control+Shift+z");
  await expect(blocks(page).nth(2)).toHaveText("Abschnitt 1.1 neu");
  await page.keyboard.press("Control+y");
  await page.keyboard.type("!");
  await expect(blocks(page).nth(3)).toHaveText("!");
});

test("undo reverts a style change", async ({ page }) => {
  await caret(page, 3, 0);
  await page.keyboard.press("Control+Alt+Digit2");
  await expect(blocks(page).nth(3)).toHaveClass("p-h2");
  await page.keyboard.press("Control+z");
  await expect(blocks(page).nth(3)).toHaveClass("p-body");
});

test("keystroke latency on 100 pages @perf", async ({ page }) => {
  await page.goto("./?doc=10");
  await expect(blocks(page).nth(800)).toBeAttached();
  const count = await blocks(page).count();
  const target = await page.evaluate(() => {
    const children = [...document.querySelector("article.absatz-doc")!.children];
    return children.findIndex((e, i) => i >= children.length / 2 && e.matches("p.p-body"));
  });
  await caret(page, target);
  await page.evaluate(() => ((window as any).__absatzLatency = []));
  await page.keyboard.type("Schnell tippen in einem langen Dokument ", { delay: 20 });
  await page.keyboard.press("Enter");
  await page.keyboard.press("Backspace");
  await page.waitForTimeout(200);
  const samples: { handler: number; frame: number }[] = await page.evaluate(() => (window as any).__absatzLatency);
  const pct = (xs: number[], p: number) => xs.sort((a, b) => a - b)[Math.min(xs.length - 1, Math.floor(xs.length * p))];
  const handler = samples.map((s) => s.handler);
  const frame = samples.map((s) => s.frame).filter((x) => x !== undefined);
  const report = `Blöcke: ${count}, Eingaben: ${samples.length}
Handler    p50 ${pct(handler, 0.5).toFixed(2)} ms   p95 ${pct(handler, 0.95).toFixed(2)} ms   max ${Math.max(...handler).toFixed(2)} ms
Bis Frame  p50 ${pct(frame, 0.5).toFixed(1)} ms   p95 ${pct(frame, 0.95).toFixed(1)} ms   max ${Math.max(...frame).toFixed(1)} ms`;
  console.log(`\nTipp-Latenz (100 Seiten)\n${report}`);
  test.info().annotations.push({ type: "latency", description: report });
  expect(pct(handler, 0.95)).toBeLessThan(16);
});
