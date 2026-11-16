//! Deterministic sample documents for benchmarks, tests and the demo.

use crate::model::{Block, BlockContent, Document, Inline};

pub struct Sample {
    pub doc: Document,
    /// A body block in the middle of the document (typing simulation).
    pub mid_block: usize,
    /// A body block near the start (edits here reflow everything after it).
    pub early_block: usize,
}

const WORDS: &[&str] = &[
    "die",
    "der",
    "und",
    "Vorlage",
    "Absatz",
    "Dokument",
    "schnell",
    "Seite",
    "Satz",
    "Gestaltung",
    "Typografie",
    "wird",
    "nicht",
    "mit",
    "für",
    "eine",
    "Schrift",
    "Abstand",
    "Überschrift",
    "Zeilenumbruch",
    "Silbentrennung",
    "Textverarbeitung",
    "Browser",
    "Leistung",
    "Struktur",
    "Inhalt",
    "klar",
    "bleibt",
    "zwischen",
    "Kapitel",
    "Verweis",
    "automatisch",
    "Formatierung",
    "Rolle",
    "semantisch",
    "lesbar",
    "Spalte",
    "Tabelle",
    "Abbildung",
    "Beschriftung",
    "Fußnote",
    "Verzeichnis",
    "Rechtschreibung",
    "Ausgabe",
    "Vorschau",
    "Änderung",
    "Version",
    "gemeinsam",
    "einfach",
    "Werkzeug",
    "präzise",
];

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 33
    }

    fn word(&mut self) -> &'static str {
        WORDS[(self.next() as usize) % WORDS.len()]
    }

    fn sentence(&mut self) -> String {
        let n = 8 + (self.next() % 14) as usize;
        let mut s = String::new();
        for i in 0..n {
            let w = self.word();
            if i == 0 {
                let mut chars = w.chars();
                s.extend(chars.next().unwrap().to_uppercase());
                s.push_str(chars.as_str());
            } else {
                s.push(' ');
                s.push_str(w);
            }
        }
        s.push('.');
        s
    }

    fn paragraph(&mut self, sentences: usize) -> String {
        (0..sentences)
            .map(|_| self.sentence())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// About ten pages per chapter with the standard style set.
pub fn document(chapters: usize) -> Sample {
    let mut rng = Rng(42);
    let mut blocks = vec![Block {
        style: "body".into(),
        content: BlockContent::Outline,
    }];
    let (mut mid_block, mut early_block) = (0, 0);

    for c in 1..=chapters {
        let title = format!("Kapitel {c}: {}", rng.sentence().trim_end_matches('.'));
        blocks.push(Block::para("h1", title));
        for section in 1..=6 {
            blocks.push(Block::para("h2", format!("Abschnitt {c}.{section}")));
            for p in 0..8 {
                if c == chapters / 2 + 1 && section == 3 && p == 4 {
                    mid_block = blocks.len();
                }
                if c == 1 && section == 1 && p == 1 {
                    early_block = blocks.len();
                }
                let mut inlines = vec![Inline::plain(rng.paragraph(3))];
                if p % 2 == 0 {
                    inlines.push(Inline::plain(" "));
                    inlines.push(Inline::styled(rng.word(), "emphasis"));
                }
                inlines.push(Inline::plain(format!(" {}", rng.paragraph(2))));
                if p % 3 == 1 {
                    inlines.push(Inline::Footnote(vec![Inline::plain(rng.sentence())]));
                }
                blocks.push(Block::text("body", inlines));

                match p {
                    2 => (0..4).for_each(|_| blocks.push(Block::para("list", rng.sentence()))),
                    5 if section % 2 == 0 => blocks.push(Block::para("note", rng.paragraph(2))),
                    6 if section == 4 => {
                        let mut rows = vec![
                            ["Spalte A", "Spalte B", "Spalte C", "Spalte D"]
                                .map(|h| vec![Inline::plain(h)])
                                .to_vec(),
                        ];
                        for r in 0..6 {
                            rows.push(vec![
                                vec![Inline::plain(rng.word())],
                                vec![Inline::plain((r * 7).to_string())],
                                vec![Inline::plain(rng.word())],
                                vec![Inline::plain((r * 13).to_string())],
                            ]);
                        }
                        let caption = vec![Inline::plain(rng.sentence())];
                        blocks.push(Block {
                            style: "body".into(),
                            content: BlockContent::Table { rows, caption },
                        });
                    }
                    7 if section == 5 => {
                        blocks.push(Block {
                            style: "body".into(),
                            content: BlockContent::Math("sum_(i=1)^n i = (n(n+1))/2".into()),
                        });
                        blocks.push(Block::para("quote", rng.paragraph(2)));
                    }
                    _ => {}
                }
            }
        }
    }
    Sample {
        doc: Document { blocks },
        mid_block,
        early_block,
    }
}
