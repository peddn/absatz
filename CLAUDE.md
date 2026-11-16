# CLAUDE.md – Leitfaden für Claude in diesem Repository

Absatz ist eine Textverarbeitung, die vollständig im Browser (Chromium) läuft und in Rust geschrieben ist.
Formatiert wird ausschließlich über Vorlagen. Man schreibt seitenlos; Seiten setzt Typst in einer Vorschau.

## Zuerst lesen

1. `docs/plan.md` – Ziele, Leistungsbudgets, Architektur, Meilensteine. **Aktuellen Meilenstein prüfen** und nur daran arbeiten.
2. `docs/decisions.md` – getroffene Entscheidungen. Nicht stillschweigend umwerfen; Änderungen als neuer Eintrag.
3. `CONTRIBUTING.md` – Git- und GitHub-Ablauf.

## Grundregeln

1. **KISS.** Die einfachste Lösung, die die Budgets einhält. Keine Abstraktionen, Feature-Flags oder Konfiguration „für später“.
2. **Keine Doppelentwicklung.** Gemeinsames gehört in `absatz-core`. Je Information genau eine Quelle:
   Vorlagen-Tokens → CSS *und* Typst · Modell → Codegen · Worker-Protokoll in `core::protocol` · Beispieldokument in `core::sample` für Tests, Benchmark und Demo.
3. **Chromium nutzen statt nachbauen.** Layout der Schreibansicht, Auswahl, Rechtschreibung, Suche, Barrierefreiheit, `content-visibility` – das macht der Browser. Keine Fallbacks für Firefox/Safari.
4. **Keine eigene Satz-Engine, kein Canvas für die Schreibansicht.** Seiten, Fußnoten, Verzeichnisse, PDF: Typst im Worker.
5. **Keine Direktformatierung**, auch nicht intern. Das Modell kennt nur Vorlagen-IDs.
6. **Neue Abhängigkeit oder Architekturänderung** → Eintrag in `docs/decisions.md`. Bei großen Abhängigkeiten vorher fragen.
7. **Widersprechen ist erwünscht.** Wenn eine Anforderung das Konzept verwässert oder den Code stark verkompliziert: sagen, begründen, Einfacheres vorschlagen.
8. **Messen statt vermuten.** Leistungsrelevante Änderungen mit `just bench` / `just bench-web` belegen.

## Architektur

| Crate | Aufgabe | Darf abhängen von |
|---|---|---|
| `absatz-core` | Modell, Vorlagensätze (`stylesets/`), Codegen (Typst, CSS), Worker-Protokoll, Beispieldokumente | nur serde |
| `absatz-typeset` | Typst-World, Setzen, SVG/Raster/PDF, Benchmark; `bin/worker.rs` ist der Web Worker | core, Typst |
| `absatz-web` | Leptos-App (CSR) für die Bedienung; `editor/` = Schreibansicht (eigener DOM-Renderer + EditContext), `preview.rs` = Seitenvorschau + PDF über den Worker, `storage.rs` = Autosave (OPFS) | core – **nie** typeset (Hauptthread bleibt klein) |

Datenfluss: Eingabe → Modell (Hauptthread) → Schreibansicht (DOM, sofort) · Modell → `codegen::typst` → Worker → Typst → Seiten/PDF (asynchron).

## Befehle

```
just setup       # Werkzeuge (läuft automatisch beim Sitzungsstart)
just lint        # fmt-Check + clippy nativ und wasm, -D warnings
just test        # native Tests (core, typeset)
just snapshots   # geänderte insta-Snapshots übernehmen – vorher Diff ansehen!
just dev         # Dev-Server http://localhost:8080/absatz/
just build       # Release-Build nach crates/absatz-web/dist + Wasm-Größen
just e2e         # Playwright gegen den Release-Build (Chromium)
just bench [n]   # Satz-Benchmark nativ, n Kapitel ≈ 10n Seiten
just bench-web [n]  # derselbe Benchmark in Chromium
```

Lokal ein vorhandenes Chrome für Playwright nutzen: `CHROME_PATH=/usr/bin/google-chrome just e2e`.

## Arbeitsweise

- Branch `claude/<thema>` von `main`, PR gegen `main`. Nie direkt auf `main` pushen, nie selbst mergen.
- Vor jedem PR: `just lint && just test`. Browserverhalten geändert → `just e2e`. Leistung betroffen → Benchmark vorher/nachher in die PR-Beschreibung.
- Commits/PR-Titel: Conventional Commits, Beschreibung deutsch – `feat(core): Zeichenvorlagen im Codegen`.
- **Sprache:** Code, Bezeichner, Kommentare Englisch. Doku, Commits, PRs, UI-Texte Deutsch.
- Tests: Logik in `core` mit Unit-Tests; erzeugter Text (Typst, CSS) mit insta-Snapshots; Browserverhalten mit Playwright (`e2e/tests`).
- Nach einem abgeschlossenen Schritt: Status in `docs/plan.md` (Meilenstein-Tabelle) aktualisieren.

## Leistungsbudgets (Kurzfassung, Details in `docs/plan.md`)

- Tastendruck → Schreibansicht aktualisiert: **≤ 16 ms** bei 100 Seiten. Der Hauptthread wartet nie auf Typst.
- Seitenvorschau nach Tippen: **≤ 250 ms** bei 100 Seiten (entprellt, im Worker).
- Vorlagenänderung: Schreibansicht sofort (CSS), Vorschau folgt asynchron mit Hinweis „wird aktualisiert“.
- Haupt-Wasm klein halten (Budget in `docs/plan.md`); Typst nur im Worker.

## Fallstricke

- Nutzertext immer über `codegen` als Typst-String-Literal (`#"…"`), nie als Markup erzeugen.
- `StyleSet::to_json` lässt fehlende Werte weg (kein `null`) – das Typst-Template prüft mit `in`. `skip_serializing_if` beibehalten.
- Rollen-Semantik steht in `template.typ` **und** `StyleSet::to_css`. Beide gemeinsam ändern.
- Jede Vorlagenänderung lässt Typst das ganze Dokument neu setzen (gemessen). Deshalb: CSS sofort, Vorschau asynchron.
- `Typesetter::compile` überspringt unveränderte Eingaben. Nicht entfernen: Ein reiner Cache-Treffer mit `comemo::evict(1)` verwirft alle Layout-Einträge, der nächste Tastendruck setzt dann alles neu (gemessen 1,7 s statt 70 ms).
- Worker-Nachrichten erst nach `FromWorker::Ready` senden – vorher gehen sie verloren.
- Positionen zählen UTF-16-Einheiten (`core::edit::Pos`), eine Fußnote zählt als 1. DOM-Struktur eines Blocks kennt nur `editor/dom.rs`.
- `selectionchange` kommt asynchron. Der `keydown`-Handler übernimmt deshalb zuerst die DOM-Auswahl; nicht entfernen, sonst landet Text nach schnellen Pfeiltasten an der falschen Stelle.
- Tippen ändert den Textknoten an Ort und Stelle (`dom::replace_text`); nur wenn das nicht zum Modell passt, wird der Block neu gerendert. Nicht pauschal neu rendern: Chromium hängt Rechtschreib-Markierungen an die Textknoten.
- Löschen innerhalb eines Blocks macht der Browser über die EditContext (Zeichen, Wörter, Emojis). Wir behandeln nur Löschen über Blockgrenzen.
- Rechtschreib-Verzögerung messen: `e2e/tools/spellcheck-latency.mjs` (braucht eine Chrome-Wörterbuchdatei, deshalb nicht in der Testsuite).
- Latenzmessung läuft nur, wenn `window.__absatzLatency` ein Array ist (Konsole/Test: `__absatzLatency = []`).
- Während einer IME-Komposition (`composing`) die EditContext nie per `update_text` zurücksetzen.
- Der `on_change`-Callback des Editors läuft, während der Editor geliehen ist: dort nur einplanen (`Preview::schedule`), nie auf das Dokument zugreifen.
- Vorschau: höchstens eine Anfrage an den Worker gleichzeitig (`busy`/`again` in `preview.rs`); nie Anfragen stapeln.
- Tests, die feste Positionen im Dokument brauchen, laden `?doc=1` (ohne Autosave); `./` zeigt das gespeicherte bzw. das Willkommensdokument.
- `web-sys` braucht für jede Web-API ein Feature in `Cargo.toml`.
- Release-Builds (LTO) dauern lange; zum Iterieren `just dev` (Dev-Profil mit optimierten Abhängigkeiten).
