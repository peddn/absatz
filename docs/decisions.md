# Entscheidungslog

Kurz und fortlaufend. Jede Entscheidung: Nummer, Datum, Status, Entscheidung, Begründung.
Status: **gilt** · **geplant** (für einen späteren Meilenstein beschlossen, noch nicht umgesetzt) · **ersetzt durch #n**.
Neue Abhängigkeiten und Architekturänderungen bekommen hier einen Eintrag.

---

**#1 · 2026-11-16 · gilt — Formatierung nur über Vorlagen**
Das Modell kennt nur Vorlagen-IDs (Absatz- und Zeichenvorlagen), keine Direktformatierung.
*Warum:* Eine Quelle für das Aussehen; saubere Semantik für Barrierefreiheit, PDF-Tags und Export; kleineres Modell und kleinere Oberfläche.

**#2 · 2026-11-16 · gilt — Nur Chromium**
Zielbrowser sind aktuelle Chromium-Versionen. Keine Ausweichlösungen für Firefox/Safari.
*Warum:* EditContext, `appearance: base-select`, File System Access und File Handling stehen nur dort bereit und sparen viel eigenen Code.

**#3 · 2026-11-16 · gilt — Seitenlos schreiben, Seiten nur in der Vorschau**
Die Schreibansicht ist semantisches DOM mit CSS aus den Vorlagen. Seiten entstehen in einer Vorschau, die Typst im Worker setzt.
*Warum:* Kein eigener Zeilen-/Seitenumbruch, kein Canvas, Auswahl/Rechtschreibung/Suche/Barrierefreiheit vom Browser. Messung (#4) zeigt: Seitensatz ist für Tipp-Latenz zu langsam, für eine Vorschau schnell genug.

**#4 · 2026-11-16 · gilt — Typst 0.15 als Satz-Engine**
Seiten, Fußnoten, Verzeichnisse, Silbentrennung, PDF (getaggt, PDF/UA-1) über die Typst-Crates.
*Warum:* Ausgereift, Rust, inkrementell, läuft als Wasm. Zahlen: `docs/benchmarks/2026-11-typst.md`.
*Folge:* Kein Textumfluss um Bilder (Typst kann keine schwebenden Abbildungen mit Umfluss).

**#5 · 2026-11-16 · gilt — Leptos (CSR) und Trunk**
UI mit Leptos 0.8 im Client-Rendering, gebaut mit Trunk 0.22 (inkl. Modul-Worker).
*Warum:* Rust durchgängig, feingranulare Reaktivität ohne virtuelles DOM; Trunk baut App und Worker in einem Schritt.

**#6 · 2026-11-16 · gilt — Vorlagensatz = `tokens.json` + `template.typ`**
`tokens.json` ist die einzige Quelle für Gestaltungswerte. Typst liest sie als `styles.json`, `StyleSet::to_css` erzeugt daraus das CSS der Schreibansicht. Die Rollen-Semantik (z. B. „emphasis ⇒ kursiv“) steht genau zweimal: in `template.typ` und in `to_css` – beide nebeneinander halten.

**#7 · 2026-11-16 · gilt — Nutzertext als Typst-String-Literale**
Codegen schreibt jeden Text als `#"…"`. Nur `"` und `\` müssen maskiert werden; Markup-Zeichen können nie Bedeutung bekommen.

**#8 · 2026-11-16 · gilt — Gemeinsame Typen in `absatz-core`**
Modell, Vorlagen, Codegen, Worker-Protokoll und Beispieldokumente liegen in `absatz-core` (ohne Web-/Typst-Abhängigkeiten). Benchmark, Tests und Demo nutzen dasselbe Beispieldokument.

**#9 · 2026-11-16 · gilt — Keine Wasm-Threads**
Kein `SharedArrayBuffer`, daher keine COOP/COEP-Header nötig; statisches Hosting (GitHub Pages) genügt. Parallelität entsteht durch Worker mit Nachrichten.

**#10 · 2026-11-16 · gilt — Einheitliche Public-URL `/absatz/`**
Dev-Server, Browser-Tests und GitHub Pages nutzen denselben Pfad. Bei eigener Domain: eine Zeile in `Trunk.toml`.

**#11 · 2026-11-16 · gilt — Lizenz AGPL-3.0-or-later**
Vom Maintainer gewählt. Verträglich mit den Apache/MIT-Abhängigkeiten und mit Pandoc (GPL-2.0-or-later, #13).

**#12 · 2026-11-16 · geplant (M1) — Loro als Dokumentspeicher**
Das Dokument ist ein Loro-CRDT: Undo/Redo, Versionen, Speicherformat (Snapshot) und spätere Zusammenarbeit aus einer Hand. `model::Document` bleibt die Lesesicht für Codegen.

**#13 · 2026-11-16 · geplant (M5) — Pandoc.wasm für Import/Export**
DOCX/ODT/Markdown über Pandoc 3.9+ in einem eigenen, erst bei Bedarf geladenen Worker. DOCX-Vorlagennamen kommen über `+styles` als `custom-style`.

**#14 · 2026-11-16 · gilt (vorläufig) — Schriften aus `typst-assets`**
M0 bettet die Typst-Standardschriften in den Worker ein. Ab M3 laden Vorlagensätze ihre Schriftdateien selbst; dieselben Bytes dienen Typst und `@font-face` (einmal laden, zweimal nutzen).
