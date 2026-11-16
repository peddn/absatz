# Absatz – Projektplan

Leitfaden für die Umsetzung. Lebendes Dokument: Änderungen per PR, Entscheidungen zusätzlich in [`decisions.md`](decisions.md).
Stand: 16.11.2026 · Aktueller Meilenstein: **Durchstich steht (Tippen → Vorschau → PDF, Autosave); Vorlagenauswahl und Zwischenablage fertig; als Nächstes Worker-Größe messen und aufräumen**

---

## 1. Vision

Absatz denkt Textverarbeitung neu: Man schreibt Inhalt mit Struktur, das Aussehen kommt aus Vorlagen, die Seiten setzt ein echtes Satzsystem.
Die Anwendung läuft vollständig im Browser, ist in Rust geschrieben und **richtig schnell** – gemessen an festen Budgets (Abschnitt 4), nicht am Gefühl.

Drei Grundsätze:

1. **Nur Vorlagen.** Jeder Absatz hat eine Absatzvorlage, hervorgehobener Text eine Zeichenvorlage. Es gibt keine Direktformatierung.
2. **Seitenlos schreiben, Seiten als Ergebnis.** Die Schreibansicht ist eine fließende Spalte. Seiten entstehen in einer Live-Vorschau und im PDF.
3. **Der Browser ist die Plattform.** Was Chromium kann, bauen wir nicht nach.

## 2. Leitprinzipien für jede Entscheidung

| Prinzip | Bedeutet konkret |
|---|---|
| KISS | Die einfachste Lösung, die die Budgets einhält. Keine Abstraktion auf Vorrat. |
| Eine Quelle je Information | Tokens → CSS + Typst. Modell → Codegen. Protokoll, Beispieldokumente: einmal, in `absatz-core`. |
| Browser nutzen statt nachbauen | Layout, Auswahl, Rechtschreibung, Suche, Barrierefreiheit der Schreibansicht macht Chromium. |
| Semantik vor Aussehen | Jede Vorlage hat eine Rolle. Daraus folgen HTML-Elemente, PDF-Tags und Export. |
| Messen statt vermuten | Leistungsfragen werden mit `just bench` / `just bench-web` entschieden. |

## 3. Umfang

### Version 1.0

- **Absatzvorlagen:** Textkörper, Überschrift 1–3, Aufzählung und nummerierte Liste (mit Ebenen), Zitat, Hinweis, Code.
- **Zeichenvorlagen:** Betont, Stark, Code, Verweis (Link).
- **Inhalte:** Fußnoten, Abbildungen mit Beschriftung, Tabellen (Tabellenvorlagen), Formeln (Typst-Mathe), Inhaltsverzeichnis.
- **Vorlagensätze:** 3–4 gestaltete Sätze (Standard, Brief, Bericht, Abschlussarbeit) mit wenigen Stellschrauben (Grundschrift, Grundgröße, Akzentfarbe). Kein freier Vorlageneditor in 1.0.
- **Seiten:** Live-Vorschau, PDF-Export (getaggt; optional PDF/UA-1).
- **Dateien:** eigenes Format `.absatz`, automatisches Speichern, Versionen; Import/Export DOCX, ODT, Markdown.
- **App:** installierbar (PWA), offline nutzbar, öffnet `.absatz`-Dateien aus dem Dateimanager.

### Bewusst nicht (Streichliste)

| Nicht in 1.0 | Stattdessen |
|---|---|
| Direktformatierung (Schrift, Größe, Farbe am Text) | Vorlagen |
| Freie Bildposition, Textumfluss | Abbildung mit Beschriftung, automatisch platziert |
| Änderungsverfolgung im Word-Stil | Versionsvergleich und Kommentare (nach 1.0) |
| Zellformatierung in Tabellen | Tabellenvorlagen |
| Freie Kopf-/Fußzeilen | Teil des Vorlagensatzes |
| Seriendruck, Makros, Zeichenwerkzeuge | – |
| Eigene Druckfunktion | PDF |
| Firefox, Safari | – (Entscheidung #2) |
| Server/Backend | Alles lokal; Zusammenarbeit nach 1.0 |

### Technisch ausgeschlossen

Eigene Satz-Engine · Canvas-Rendering der Schreibansicht · eigene Rechtschreibprüfung · Wasm-Threads/`SharedArrayBuffer` · Typst im Hauptthread.

## 4. Leistungsbudgets

Referenz: 100-seitiges Dokument (`core::sample::document(10)`) in Chromium auf dem **Referenzgerät** (Rechner des Maintainers, Arch Linux).
Werte mit „VM“ stammen von der langsameren 2-Kern-Cloud-VM. Details: [`benchmarks/2026-11-typst.md`](benchmarks/2026-11-typst.md).

| Messgröße | Budget | Gemessen (Referenzgerät) | Folge |
|---|---|---|---|
| Tastendruck → Schreibansicht aktualisiert | **≤ 16 ms** | 2,3 ms p50 / 3,0 ms p95 (mit laufender Vorschau) | Hauptthread wartet nie auf Typst |
| Tastendruck → Seitenvorschau aktuell | **≤ 250 ms** | Satz 49 ms + SVG 18 ms (200 S.: 97 + 21 ms) | Entprellen ~100 ms, nur sichtbare Seiten rendern |
| Vorlagenänderung → Schreibansicht | **≤ 50 ms** | – (M1, nur CSS) | sofort sichtbar |
| Vorlagenänderung → Seitenvorschau | **≤ 2 s** | 1,1–1,2 s (200 S.: 2,2–2,3 s) | Hinweis „wird aktualisiert“, alte Seiten bleiben stehen |
| Erstes Öffnen: App bedienbar | **≤ 1 s** | Haupt-Wasm 113 KB gzip | Worker lädt im Hintergrund |
| Erstes Öffnen: Vorschau da (100 S.) | **≤ 4 s** | Satz 1,3 s + Laden Worker | Worker verkleinern (M2), Cache per Service Worker (M4) |
| PDF-Export (100 S.) | **≤ 1 s** | 0,2 s | – |
| Speicher Worker (100 S.) | **≤ 500 MB** | 292 MB (200 S.: 553 MB) | `comemo::evict(1)`, unveränderte Eingaben überspringen |
| Haupt-Wasm | **≤ 400 KB gzip** | 139 KB | Typst nie im Hauptthread |
| Worker-Wasm | **≤ 8 MB gzip** | 16,5 MB (inkl. 9,6 MB Schriften roh) | Schriften auslagern, Größe optimieren (M2/M3) |

Wichtigste Erkenntnisse aus M0:

- Typst setzt nach einer Textänderung inkrementell neu (49 ms für 100 Seiten auf dem Referenzgerät, 97 ms für 200; linear wachsend) – für die Vorschau gut, für Tipp-Latenz zu langsam. **Das bestätigt die seitenlose Schreibansicht.**
- **Jede Vorlagenänderung lässt Typst das ganze Dokument neu setzen** (Show-/Set-Regeln stecken in jeder Stilkette). Deshalb zeigt die Schreibansicht die Änderung sofort per CSS; die Vorschau folgt.
- Der Satz-Cache dominiert den Speicher. `evict(1)` statt `evict(10)` senkt ihn im Browser von 947 auf 292 MB ohne messbaren Zeitverlust – aber nur, wenn unveränderte Eingaben gar nicht erst gesetzt werden (sonst 1,7 s Ausreißer).
- Sehr lange Dokumente (200+ Seiten) reißen knapp die Budgets für Vorlagenänderung (2,2 s) und Speicher (553 MB). Für 1.0 akzeptiert; danach prüfen: Vorschau kapitelweise setzen.
- Schnellere Hardware hilft beim Neusatz nach dem Tippen wenig (VM 72 ms → Referenzgerät 49 ms), bei PDF und Kaltstart viel (Faktor 2–3). Typst gehört deshalb nie zwischen Tastendruck und Bildschirm.

## 5. Architektur

```
 Hauptthread (absatz-web, klein)                    Worker (absatz-typeset, groß)
 ┌──────────────────────────────────────┐           ┌───────────────────────────────┐
 │ Leptos-Oberfläche                    │           │ Typst-World im Speicher        │
 │ Schreibansicht: semantisches DOM     │  Quelltext│  main.typ   ← Codegen          │
 │   + EditContext (Eingabe/IME)        │ ────────► │  template.typ ← Vorlagensatz   │
 │   + CSS aus Vorlagen-Tokens          │  Tokens   │  styles.json  ← Tokens         │
 │ Dokument + Verlauf (absatz-core)     │ ────────► │ compile → Seiten               │
 │ codegen::typst (absatz-core)         │ ◄──────── │ SVG je sichtbarer Seite, PDF   │
 │ Seitenvorschau (SVG-Bilder)          │  Seiten   │                                │
 └──────────────────────────────────────┘           └───────────────────────────────┘
          ▲ beide nutzen absatz-core: Modell · Vorlagen · Codegen · Protokoll · Beispiele
```

### 5.1 Crates und Abhängigkeitsregeln

| Crate | Inhalt | Abhängigkeiten |
|---|---|---|
| `absatz-core` | `model`, `styles` (+ `stylesets/`), `codegen`, `protocol`, `sample` | nur serde – schnell gebaut, nativ testbar |
| `absatz-typeset` | `Typesetter` (World, compile, SVG, Raster, PDF), `bench`, `bin/worker.rs` | core, Typst |
| `absatz-web` | Leptos-App, Schreibansicht, Worker-Anbindung | core – **nie** typeset |

Neue Crates nur, wenn eine dieser Regeln sonst bricht (z. B. ein eigener Pandoc-Worker in M5).

### 5.2 Wer besitzt was

- **Hauptthread besitzt das Dokument.** Er wendet Eingaben an, aktualisiert das DOM synchron und erzeugt den Typst-Quelltext (`codegen::typst`, 3 ms für 100 Seiten).
- **Der Worker ist eine Funktion „Quelltext + Tokens → Seiten“.** Er hält nur die Typst-World, kein Dokumentmodell.
- Protokoll (`core::protocol`): JSON-Nachrichten. Zuerst immer den ganzen Quelltext senden (Typst diffed selbst, `Source::replace`). Erst wenn gemessen nötig: nur geänderte Blöcke (`Generated::block_ranges` existiert dafür).

### 5.3 Vorlagensystem

- Vorlagensatz = `stylesets/<id>/tokens.json` + `template.typ`.
- `tokens.json` ist die **einzige** Quelle für Werte. Typst liest sie als `styles.json`; `StyleSet::to_css` erzeugt das CSS der Schreibansicht.
- Jede Absatzvorlage hat eine **Rolle** (`body`, `heading`, `list_item`, `quote`, `note`, …). Die Rolle bestimmt das Typst-Element (`blk()` im Template) und das HTML-Element in der Schreibansicht.
- Eigenschaften bleiben bewusst wenige: Größe, Farbe, kursiv, fett, Einzug, Zeilenabstand, Absatzabstand, Blocksatz (+ später Nummerierung). Was beide Ausgaben nicht gleich können, kommt nicht hinein.
- Das Dokument speichert den verwendeten Vorlagensatz vollständig (Tokens + Template), damit es auch mit künftigen App-Versionen gleich aussieht.

### 5.4 Schreibansicht (M1)

- Jeder Block ist ein semantisches Element (`h1`–`h3`, `p`, `blockquote`, Listen) mit `data-block="<id>"` und Klasse `p-<vorlage>`; Zeichenvorlagen als `span.c-<vorlage>`.
- **EditContext** am Dokument-Container: `textupdate` → Operation aufs Modell → gezieltes DOM-Update. Native Auswahl über die Selection-API; Zuordnung DOM ↔ (Block, Offset) über `data-block`.
- Befehle selbst: Enter (Block teilen, Folgevorlage), Rückschritt (Blöcke verbinden), Tab (Listenebene), Vorlagenwechsel per Tastenkürzel (Strg+Alt+1–3 Überschriften, Strg+Alt+0 Textkörper, Strg+B/I für Stark/Betont), Rückgängig.
- `content-visibility: auto` für Blöcke außerhalb des Sichtbereichs – große Dokumente ohne eigene Virtualisierung.
- **Spike entschieden (#15):** Der Dokumentbereich wird direkt gerendert (`editor/dom.rs`), nicht mit Leptos. Die EditContext spiegelt nur den fokussierten Block. Offen: Rechtschreibprüfung manuell im echten Chrome bestätigen.

### 5.5 Seitenvorschau (M2)

- Nach Eingaben entprellt (~100 ms) Quelltext an den Worker; der Worker setzt und rendert **nur sichtbare Seiten** als SVG.
- Seiten-Cache: Hash je Seiten-Frame; unveränderte Seiten werden nicht neu gerendert oder übertragen.
- Während der Worker rechnet, bleiben alte Seiten sichtbar (Hinweis „wird aktualisiert“). Neue Anfragen ersetzen wartende (nie Warteschlangen aufbauen).
- Klick in die Vorschau → Typst-Span → `block_ranges` → Block in der Schreibansicht (und umgekehrt).
- PDF-Export: `typst-pdf`, getaggt; Speichern über File System Access.

### 5.6 Dokument und Speicherung (M1, M4)

- Das Dokument ist `model::Document` im Hauptthread; Änderungen laufen über `core::edit`.
- **Rückgängig/Wiederholen** über `core::history` (#16): Jede Änderung ersetzt einen zusammenhängenden Blockbereich; ein Schritt speichert ihn vorher und nachher. Tippen im selben Block wird bis zu einer Pause von 1 s zusammengefasst.
- Versionen (M4) sind gespeicherte Stände des Dokuments, kein CRDT-Verlauf.
- Invarianten: jeder Block hat eine gültige Absatzvorlage; Zeichenvorlagen überlappen nicht (höchstens eine je Zeichen); unbekannte Vorlagen-IDs fallen auf `body` zurück und werden gemeldet.
- Datei `.absatz` = ZIP: `document.json` (Modell), `styleset/`, `assets/<hash>` (Bilder), `manifest.json` mit Formatversion.
- Autosave im OPFS; Öffnen/Speichern über File System Access; File Handling API für die installierte PWA.

### 5.7 Import und Export (M5)

- Pandoc 3.9+ als Wasm in einem eigenen, erst bei Bedarf geladenen Worker (≈ 16 MB).
- DOCX-Import mit `+styles`: Word-Vorlagennamen kommen als `custom-style` an → Zuordnungstabelle auf unsere Vorlagen; Rest per Rolle (Überschrift, Liste, Zitat) oder auf `body`.
- Einfügen aus der Zwischenablage braucht kein Pandoc: kleine HTML-Zuordnung (`h1–h3`, `p`, `li`, `em`, `strong`, `blockquote`) in `absatz-core`, **dieselbe** Zuordnungstabelle wie beim Import.
- Export DOCX/ODT/Markdown überträgt Struktur und Vorlagennamen, nicht das exakte Aussehen. Exaktes Aussehen: PDF.

### 5.8 Schriften (M3)

- Vorlagensätze nennen ihre Schriftdateien. Die App lädt sie einmal und nutzt dieselben Bytes zweimal: im Worker für Typst, im Hauptthread über die `FontFace`-API für die Schreibansicht. So stimmen Schreibansicht und Seiten überein.
- Nur freie Schriften (OFL). M0 bettet übergangsweise `typst-assets` ein (9,6 MB).

### 5.9 Barrierefreiheit

- Schreibansicht: echte Überschriften/Listen aus den Rollen; der Editor-Container mit `role="textbox"` und `aria-multiline`.
- PDF: getaggt (Typst-Standard), Export als PDF/UA-1 wählbar; Dokumenttitel aus den Metadaten.
- Prüfung mit axe in den Playwright-Tests ab M3.

## 6. Meilensteine

Jeder Meilenstein endet mit grüner CI, aktualisiertem Plan und Tag `v0.<n>.0`.

| | Ziel | Fertig, wenn … | Status |
|---|---|---|---|
| **M0** Gerüst | Repo, Werkzeuge, CI, Benchmark, Plan | Native und Browser-Tests grün; Benchmark nativ + Chromium dokumentiert; Pages-Deploy | ✅ fertig |
| **M1** Schreibkern | Tippen, Absätze, Vorlagen wechseln, Rückgängig | Spike entschieden (5.4); Tippen/Enter/Rückschritt/Auswahl/IME/Einfügen (Klartext) funktionieren; Rückgängig/Wiederholen; ≤ 16 ms pro Taste bei 100 Seiten; e2e-Tests inkl. IME über CDP | 🔄 Spike ✅, Undo ✅ |
| **M2** Seiten | Live-Vorschau und PDF | Vorschau nur sichtbarer Seiten, entprellt, Cache; Klick-Sprung in beide Richtungen; PDF-Download; Budgets 4 eingehalten; Worker ≤ 12 MB gzip | offen |
| **M3** Inhalte & Vorlagensätze | Alles aus 3. „Version 1.0“ außer Dateien/Austausch | Listen mit Ebenen, Fußnoten, Abbildungen, Tabellen, Formeln, Verzeichnis; 3 Vorlagensätze; Vorlagenwahl mit `appearance: base-select`; Schriften-Pipeline; axe ohne Fehler | offen |
| **M4** Dateien & App | Speichern wie eine echte App | `.absatz`-Format, Autosave (OPFS), Öffnen/Speichern, Dateizuordnung, installierbar, offline; Versionsliste | offen |
| **M5** Austausch | DOCX/ODT/Markdown | Import/Export über Pandoc-Worker; Einfügen von HTML normalisiert; Zuordnungstabelle geteilt | offen |
| **M6** 1.0 | Feinschliff | Budgets auf Referenzgerät, Fehlerbehandlung, Einstieg/Hilfe, Doku | offen |

Nach 1.0: Kommentare, Versionsvergleich, gemeinsames Bearbeiten (CRDT wie Loro, nur dann nachgeladen – siehe #16), freier Vorlageneditor.

### Reihenfolge nach der kritischen Durchsicht (#17)

Die Meilensteine bleiben als Themen bestehen, die Reihenfolge der nächsten Schritte ändert sich:

1. ✅ Tippen ändert nur den Textknoten; Löschen im Block macht der Browser. Rechtschreibprüfung: Markierungen stimmen, erscheinen aber etwa 1 s nach dem letzten Tastendruck (Chromium-Verhalten mit EditContext, #18).
2. ✅ **Durchstich:** Tippen → Live-Vorschau (entprellt 100 ms, nur sichtbare Seiten, alte Seiten bleiben bei Satzfehlern stehen) → PDF-Download; automatisches Speichern im OPFS; „Neues Dokument“; Willkommensdokument. Unbekannte Vorlagen fallen auf Textkörper zurück.
   Bewusst noch nicht: Fehlerabfang für Formeln (erst wenn Formeln editierbar sind), Klick-Sprung Vorschau ↔ Text, Seiten-Cache (erst wenn gemessen nötig).
3. ✅ **Sichtbare Vorlagenauswahl** (zeigt die Vorlage an der Schreibmarke, ändert sie für die Auswahl), Einfügen als Klartext (Zeilen werden Absätze), Ausschneiden. Die Auswahl ist ein schlichtes `<select>`; die gestaltete Variante (`base-select`) folgt mit den Vorlagensätzen.
4. Worker-Größe messen (Typst ohne Schriften), Budget danach festlegen; Vorratscode entfernen (Raster-Export, Benchmark-Knopf in der App).
5. Danach: Zeichenvorlagen, weitere Inhalte.

Bekannte Abweichungen Schreibansicht ↔ Seiten (bewusst „ähnlich“, nicht „gleich“): Dokumentschrift wird in der Schreibansicht noch nicht geladen, Überschriften-Nummern fehlen dort, Zeilenabstand nur angenähert.

### M1 im Detail

1. ✅ **Spike:** EditContext + direkter DOM-Renderer, Editieroperationen in `core::edit`, e2e-Tests für Tippen, Pfeiltasten, Enter, Rückschritt, Auswahl über Blöcke, Vorlagenkürzel, IME, Latenz bei 100 Seiten. Entscheidung #15. Offen: Rechtschreibprüfung manuell bestätigen.
2. ✅ **Rückgängig/Wiederholen** im Kern (`core::history`, Strg+Z, Strg+Umschalt+Z, Strg+Y). Loro verworfen (#16).
3. Befehle: ✅ Enter, Rückschritt, Löschen über Absatzgrenzen, Vorlagenwechsel; offen: Zeichenvorlage umschalten (Strg+B/I).
4. Zwischenablage: Kopieren als Klartext + HTML (aus Rollen); Einfügen als Klartext.
5. e2e-Tests für jeden Befehl; Leistungstest „100 Seiten tippen“.

## 7. Teststrategie

| Ebene | Werkzeug | Was |
|---|---|---|
| Logik | `cargo test` in `absatz-core` | Modell-Operationen, Invarianten, Zuordnungen |
| Erzeugter Text | insta-Snapshots | Typst-Codegen, CSS, später HTML-Kopie |
| Satz | `cargo test` in `absatz-typeset` | Dokumente kompilieren fehlerfrei, Seitenzahlen plausibel |
| Browser | Playwright (Chromium) | Start, Tippen, IME, Auswahl, Zwischenablage, Vorschau, Dateien |
| Leistung | `just bench`, `just bench-web` | Budgets aus Abschnitt 4; wöchentlich in CI (nicht blockierend) |

Grundsatz: Für jeden behobenen Fehler zuerst ein Test, der ihn zeigt.

## 8. Risiken

| Risiko | Wirkung | Gegenmaßnahme |
|---|---|---|
| EditContext-Randfälle (Auswahl, IME, Rechtschreibung) | Kern der Bedienung | Spike zuerst (M1.1); e2e mit CDP-IME; Google Docs nutzt die API produktiv |
| Rechtschreib-Linien kommen ~1 s verspätet (#18) | Wirkt träge gegenüber normalen Textfeldern | Vorerst hingenommen; vor 1.0 neu bewerten (eigene Prüfung oder Chromium-Fix) |
| Vorlagenänderung = Neusatz von allem | Vorschau 1–2 s bei großen Dokumenten | Schreibansicht sofort per CSS; Vorschau asynchron; Größe im Blick behalten |
| Worker-Größe (16 MB gzip) | Erster Start langsam | Schriften auslagern, `opt-level = "s"` für den Worker testen, Raster-Export entfernen falls ungenutzt, Service-Worker-Cache |
| Speicher des Typst-Caches | Tab-Absturz bei sehr großen Dokumenten | `evict(1)`, Spitzenwert messen, Budget 500 MB |
| Typst-Updates ändern das Template-Verhalten | Dokumente sehen anders aus | Typst-Version fest pinnen; Vorlagensatz im Dokument speichern; Snapshot-Tests der Vorlagensätze |
| Pandoc-Größe und GPL | Ladezeit, Lizenzpflichten | Nur bei Bedarf laden; als separates Modul mit Lizenztext und Quellverweis ausliefern |
| Zusammenarbeit braucht später ein CRDT | Umbau der Speicherung | Alle Änderungen laufen schon über wenige Operationen in `core::edit` – die lassen sich auf ein CRDT abbilden; CRDT-Bibliothek dann nur beim gemeinsamen Bearbeiten nachladen |

## 9. Offene Fragen an den Maintainer

1. ~~GitHub-Repository~~ → `peddn/absatz`, Pages aktiv.
2. ~~Referenzgerät~~ → Rechner des Maintainers (Arch Linux, Chromium).
3. Sollen eigene Vorlagen in 1.0 gar nicht (Plan) oder in einer einfachen Form (Duplizieren + Stellschrauben) möglich sein?
