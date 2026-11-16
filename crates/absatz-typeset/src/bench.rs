//! Typesetting benchmark over the real pipeline:
//! model → codegen → Typst → pages/PDF. Runs natively (example `bench`)
//! and in the browser worker, so both report the same measurements.

use absatz_core::codegen;
use absatz_core::model::{BlockContent, Document, Inline};
use absatz_core::protocol::BenchReport;
use absatz_core::sample;
use absatz_core::styles::{self, StyleSet};

use crate::Typesetter;

/// `now` returns milliseconds from any fixed origin.
pub fn run(chapters: usize, now: &dyn Fn() -> f64) -> BenchReport {
    let mut report = BenchReport::default();
    let mut sample = sample::document(chapters);

    let mut ts = None;
    report.push(
        "Typesetter anlegen (Schriften laden)",
        measure(now, || ts = Some(Typesetter::new(&styles::STANDARD))),
    );
    let mut ts = ts.unwrap();

    let mut source = String::new();
    report.push(
        "Codegenerierung (ganzes Dokument)",
        measure(now, || source = codegen::typst(&sample.doc).source),
    );
    report.source_kb = source.len() / 1024;

    report.push(
        "Kaltstart (erstes Setzen)",
        measure(now, || update(&mut ts, &sample.doc)),
    );
    report.pages = ts.page_count();
    report.push(
        "Unverändert (wird übersprungen)",
        measure(now, || update(&mut ts, &sample.doc)),
    );

    // Typing: 30 characters, one full pipeline run each.
    let mut typing = Vec::new();
    for (i, ch) in "schnelle Textverarbeitung im Browser"
        .chars()
        .take(30)
        .enumerate()
    {
        insert_char(&mut sample.doc.blocks[sample.mid_block].content, 40 + i, ch);
        typing.push(measure(now, || update(&mut ts, &sample.doc)));
    }
    report.push("Tippen Mitte: Median pro Zeichen", median(&typing));
    report.push(
        "Tippen Mitte: Maximum",
        typing.iter().copied().fold(0.0, f64::max),
    );

    // A sentence early in the document shifts every following line and page.
    let mut reflow = Vec::new();
    for i in 0..5 {
        let sentence = format!(" Eingefügter Satz {i} verschiebt alle folgenden Zeilen.");
        for (j, ch) in sentence.chars().enumerate() {
            insert_char(
                &mut sample.doc.blocks[sample.early_block].content,
                20 + j,
                ch,
            );
        }
        reflow.push(measure(now, || update(&mut ts, &sample.doc)));
    }
    report.push("Satz früh einfügen (Median)", median(&reflow));

    // Style changes use fresh values each time, so no cached layout is reused.
    let mut set = StyleSet::standard();
    let mut size_runs = Vec::new();
    for size in [11.5, 11.6, 11.7] {
        set.paragraph.get_mut("body").unwrap().look.size = Some(size);
        ts.set_styles(set.to_json());
        size_runs.push(measure(now, || compile(&mut ts)));
    }
    report.push("Vorlage: Grundschriftgröße (Median)", median(&size_runs));
    let mut color_runs = Vec::new();
    for color in ["#7a1f1f", "#1f7a3a", "#1f2a79"] {
        set.paragraph.get_mut("h2").unwrap().look.color = Some(color.into());
        ts.set_styles(set.to_json());
        color_runs.push(measure(now, || compile(&mut ts)));
    }
    report.push("Vorlage: Farbe Überschrift 2 (Median)", median(&color_runs));

    let middle = ts.page_count() / 2;
    let mut svg = String::new();
    report.push(
        "Eine Seite als SVG",
        measure(now, || svg = ts.page_svg(middle).unwrap()),
    );
    report.svg_kb = svg.len() / 1024;
    report.push(
        "Eine Seite als Raster (2 px/pt)",
        measure(now, || {
            ts.page_rgba(middle, 2.0).unwrap();
        }),
    );
    let mut pdf = Vec::new();
    report.push(
        "PDF-Export gesamt",
        measure(now, || pdf = ts.pdf().unwrap()),
    );
    report.pdf_kb = pdf.len() / 1024;
    report.memory_mb = crate::memory_mb();
    report
}

fn update(ts: &mut Typesetter, doc: &Document) {
    ts.set_source(&codegen::typst(doc).source);
    compile(ts);
}

fn compile(ts: &mut Typesetter) {
    ts.compile().expect("sample document compiles");
}

fn measure(now: &dyn Fn() -> f64, f: impl FnOnce()) -> f64 {
    let t = now();
    f();
    now() - t
}

fn insert_char(content: &mut BlockContent, at: usize, ch: char) {
    if let BlockContent::Text(inlines) = content
        && let Some(Inline::Text { text, .. }) = inlines.first_mut()
    {
        let byte = text.char_indices().nth(at).map_or(text.len(), |(i, _)| i);
        text.insert(byte, ch);
    }
}

fn median(xs: &[f64]) -> f64 {
    let mut v = xs.to_vec();
    v.sort_by(f64::total_cmp);
    v[v.len() / 2]
}
