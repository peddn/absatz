//! Absatz web app.
//!
//! URL parameters: `?doc=<chapters>` loads a sample document of that size
//! (≈10 pages per chapter, default 1); `?bench=<chapters>` runs the
//! typesetting benchmark in the worker right away.

mod editor;
mod worker;

use absatz_core::sample;
use absatz_core::styles::StyleSet;
use leptos::prelude::*;

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(App);
}

#[component]
fn App() -> impl IntoView {
    let styles = StyleSet::standard();
    let css = styles.to_css();
    let doc = sample::document(param("doc").unwrap_or(1)).doc;
    let doc_ref = NodeRef::<leptos::html::Article>::new();
    let editor_styles = styles.clone();
    doc_ref.on_load(move |el| editor::mount(el, doc, editor_styles));
    let (report, set_report) = signal(None::<String>);

    let run_bench = move |chapters: usize| {
        set_report.set(Some(format!("Benchmark läuft ({chapters} Kapitel) …")));
        worker::run_bench(chapters, move |text| set_report.set(Some(text)));
    };
    if let Some(chapters) = param("bench") {
        run_bench(chapters);
    }

    view! {
        <style>{css}</style>
        <header class="topbar">
            <h1>"Absatz"</h1>
            <span>"M1 · Schreibkern (Spike)"</span>
        </header>
        {(!editor::supported()).then(|| view! {
            <p class="notice" role="alert">
                "Bearbeiten geht nur in Chrome, Edge oder Chromium (ab Version 121). "
                "In diesem Browser kannst du das Dokument nur ansehen."
            </p>
        })}
        <main class="layout">
            <article class="absatz-doc" lang=styles.lang.clone() node_ref=doc_ref></article>
            <aside class="panel">
                <h2>"Satz-Benchmark"</h2>
                <p>"Setzt ein generiertes Dokument mit Typst im Web Worker."</p>
                <button on:click=move |_| run_bench(10)>"100 Seiten messen"</button>
                <pre id="bench-result">{move || report.get()}</pre>
            </aside>
        </main>
    }
}

fn param(name: &str) -> Option<usize> {
    let search = web_sys::window()?.location().search().ok()?;
    let params = web_sys::UrlSearchParams::new_with_str(&search).ok()?;
    params.get(name)?.parse().ok()
}
