# Absatz

Textverarbeitung, neu gedacht: läuft vollständig im Browser, ist in Rust geschrieben und richtig schnell.

- **Nur Vorlagen.** Es gibt keine Direktformatierung. Jeder Absatz hat eine Absatzvorlage, hervorgehobener Text eine Zeichenvorlage.
- **Seitenlos schreiben.** Geschrieben wird in einer fließenden Spalte; Seiten entstehen in einer Live-Vorschau.
- **Typografie von Typst.** Seitenumbruch, Fußnoten, Verzeichnisse und PDF übernimmt [Typst](https://typst.app) im Web Worker.

Status: **M0 – Gerüst.** Siehe [`docs/plan.md`](docs/plan.md).

## Schnellstart

```
just setup
just dev     # http://localhost:8080/absatz/
just test
just bench   # Satz-Benchmark nativ; `just bench-web` in Chromium
```

Zielbrowser ist Chromium (Chrome, Edge) in einer aktuellen Version.

## Dokumentation

- [`docs/plan.md`](docs/plan.md) – Ziele, Architektur, Leistungsbudgets, Meilensteine
- [`docs/decisions.md`](docs/decisions.md) – Entscheidungslog
- [`CONTRIBUTING.md`](CONTRIBUTING.md) – Git- und GitHub-Ablauf

## Lizenz

[AGPL-3.0-or-later](LICENSE)
