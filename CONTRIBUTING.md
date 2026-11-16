# Mitarbeiten an Absatz

Dieser Ablauf gilt für Menschen und für Claude gleichermaßen.

## Rollen

- **Maintainer** (Produktverantwortung): setzt Prioritäten über Issues und Meilensteine, prüft und merged Pull Requests.
- **Claude** (Umsetzung): arbeitet Issues in Branches ab, öffnet Pull Requests, merged nie selbst und pusht nie auf `main`.

## Git-Ablauf (trunk-based)

1. `main` ist immer grün und auslieferbar. Jeder Push auf `main` wird nach GitHub Pages ausgeliefert.
2. Pro Thema ein kurzlebiger Branch von `main`:
   `feat/…`, `fix/…`, `perf/…`, `docs/…`, `chore/…` – Claude nutzt `claude/<thema>`.
3. Kleine Pull Requests (Richtwert: unter 400 geänderte Zeilen ohne Snapshots), ein Thema je PR.
4. Merge per **Squash**. Der PR-Titel wird zur Commit-Nachricht auf `main`.
5. Branch nach dem Merge löschen.

## Commit- und PR-Titel

[Conventional Commits](https://www.conventionalcommits.org/de/), Typ auf Englisch, Beschreibung auf Deutsch:

```
feat(core): Zeichenvorlagen im Typst-Codegen
fix(web): Auswahl nach Absatzwechsel erhalten
perf(typeset): Seiten-Rendering nur für sichtbare Seiten
docs: Meilenstein M1 abgeschlossen
```

Bereiche: `core`, `typeset`, `web`, `e2e`, `ci`, `docs`.

## Issues, Meilensteine, Labels

- **Meilensteine** entsprechen den Phasen in `docs/plan.md` (M0, M1, …).
- **Labels**: `typ:feature`, `typ:fehler`, `typ:aufräumen`, `perf`, `entscheidung`, `bereich:core|typeset|web`.
- Ein Issue beschreibt das *Was* und das *Woran merken wir, dass es fertig ist*. Das *Wie* steht im PR.
- Offene Architekturfragen bekommen das Label `entscheidung`; das Ergebnis landet in `docs/decisions.md`.

## Prüfungen

| Wann | Befehl |
|---|---|
| immer | `just lint` und `just test` |
| Browserverhalten geändert | `just e2e` |
| Leistung betroffen | `just bench` bzw. `just bench-web`, Zahlen vorher/nachher in den PR |

Die CI (`.github/workflows/ci.yml`) führt Lint, Tests, Release-Build und Browser-Tests aus. Der Benchmark läuft wöchentlich oder manuell (`bench.yml`) und blockiert nichts.

## GitHub-Einstellungen (einmalig)

- Branch-Schutz für `main`: PR erforderlich, CI-Jobs „Lint und Tests“ und „Web-Build und Browser-Tests“ müssen grün sein, nur Squash-Merge erlaubt, Branches nach Merge automatisch löschen.
- Pages: Quelle „GitHub Actions“.

## Releases

Ein abgeschlossener Meilenstein wird als `v0.<n>.0` getaggt; die Release-Notizen erzeugt GitHub aus den PR-Titeln.

## Einrichtung

```
just setup      # trunk, just, Playwright-Abhängigkeiten
just dev        # http://localhost:8080/absatz/
```

Rust-Version und Wasm-Ziel kommen aus `rust-toolchain.toml`.
