//! Absatz web app.
//!
//! URL parameters (for tests and measurements):
//! `?doc=<chapters>` opens a generated sample document (≈10 pages per
//! chapter) instead of the saved one and turns autosave off;
//! `?bench=<chapters>` runs the typesetting benchmark in a worker.

mod editor;
mod preview;
mod storage;
mod worker;

use absatz_core::model::Document;
use absatz_core::styles::StyleSet;
use absatz_core::{codegen, sample};
use leptos::prelude::*;
use preview::{Preview, Snapshot};
use wasm_bindgen::{JsCast, closure::Closure};
use web_sys::{HtmlElement, HtmlSelectElement};

fn main() {
    console_error_panic_hook::set_once();
    wasm_bindgen_futures::spawn_local(async {
        let (doc, autosave) = match param("doc") {
            Some(chapters) => (sample::document(chapters).doc, false),
            None => {
                let saved = storage::load().await.as_string();
                let doc = saved.and_then(|json| Document::from_stored_json(&json));
                (doc.unwrap_or_else(sample::welcome), true)
            }
        };
        leptos::mount::mount_to_body(move || view! { <App doc autosave /> });
    });
}

#[component]
fn App(doc: Document, autosave: bool) -> impl IntoView {
    let styles = StyleSet::standard();
    let css = styles.to_css();
    let lang = styles.lang.clone();
    let style_options = styles
        .paragraph
        .iter()
        .map(|(id, style)| view! { <option value=id.clone()>{style.name.clone()}</option> })
        .collect_view();
    let bench = param("bench");

    let main_ref = NodeRef::<leptos::html::Main>::new();
    main_ref.on_load(move |main| start(&main, &doc, styles, autosave));

    view! {
        <style>{css}</style>
        <header class="topbar">
            <h1>"Absatz"</h1>
            <select id="style" aria-label="Absatzvorlage" title="Absatzvorlage">{style_options}</select>
            <button id="new-doc">"Neues Dokument"</button>
            <button id="pdf">"PDF"</button>
            <span id="preview-status" role="status"></span>
        </header>
        {(!editor::supported()).then(|| view! {
            <p class="notice" role="alert">
                "Bearbeiten geht nur in Chrome, Edge oder Chromium (ab Version 121). "
                "In diesem Browser kannst du das Dokument nur ansehen."
            </p>
        })}
        <main class="layout" node_ref=main_ref>
            <article class="absatz-doc" lang=lang></article>
            <section class="preview" aria-label="Seitenvorschau"></section>
        </main>
        {bench.map(|chapters| view! { <Bench chapters /> })}
    }
}

/// Mount the editor and connect it to the preview and the buttons.
/// Plain DOM from here on: the document area is not rendered by Leptos (#15).
fn start(main: &HtmlElement, doc: &Document, styles: StyleSet, autosave: bool) {
    let find = |selector: &str| {
        main.owner_document()?
            .query_selector(selector)
            .ok()
            .flatten()
    };
    let (Some(article), Some(pane), Some(status)) = (
        find(".absatz-doc"),
        find(".preview"),
        find("#preview-status"),
    ) else {
        return;
    };

    let handle = editor::mount(article.unchecked_into(), doc, styles);
    let snapshot: Box<dyn Fn() -> Snapshot> = match handle.clone() {
        Some(handle) => Box::new(move || {
            handle.with_doc(|doc| Snapshot {
                source: codegen::typst(doc).source,
                save: autosave.then(|| doc.to_stored_json()),
            })
        }),
        // Read-only browsers still get the pages.
        None => {
            let source = codegen::typst(doc).source;
            Box::new(move || Snapshot {
                source: source.clone(),
                save: None,
            })
        }
    };
    let Some(preview) = Preview::start(pane, status, snapshot) else {
        return;
    };
    if let Some(handle) = handle {
        let preview = preview.clone();
        handle.on_change(move || preview.schedule());
        // Style picker: shows the style at the caret and changes it.
        if let Some(select) = find("#style").and_then(|e| e.dyn_into::<HtmlSelectElement>().ok()) {
            let shown = select.clone();
            handle.on_style(move |id| shown.set_value(id));
            let chosen = select.clone();
            on(Some(select.into()), "change", move || {
                handle.set_style(&chosen.value())
            });
        }
    }

    on(find("#pdf"), "click", move || preview.pdf());
    on(find("#new-doc"), "click", || {
        wasm_bindgen_futures::spawn_local(async {
            let _ = storage::save(&Document::empty().to_stored_json()).await;
            if let Some(window) = web_sys::window() {
                let _ = window.location().set_search("");
            }
        });
    });
}

fn on(element: Option<web_sys::Element>, event: &str, f: impl Fn() + 'static) {
    let Some(element) = element else { return };
    let cb = Closure::<dyn Fn()>::new(f);
    let _ = element.add_event_listener_with_callback(event, cb.as_ref().unchecked_ref());
    cb.forget();
}

/// Typesetting benchmark, shown only with `?bench=<chapters>`.
#[component]
fn Bench(chapters: usize) -> impl IntoView {
    let (report, set_report) = signal(format!("Benchmark läuft ({chapters} Kapitel) …"));
    worker::run_bench(chapters, move |text| set_report.set(text));
    view! {
        <aside class="panel">
            <h2>"Satz-Benchmark"</h2>
            <pre id="bench-result">{move || report.get()}</pre>
        </aside>
    }
}

fn param(name: &str) -> Option<usize> {
    let search = web_sys::window()?.location().search().ok()?;
    let params = web_sys::UrlSearchParams::new_with_str(&search).ok()?;
    params.get(name)?.parse().ok()
}
