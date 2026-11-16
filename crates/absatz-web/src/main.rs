//! Absatz web app. M0: renders a sample document with style-set CSS and
//! runs the typesetting benchmark in the worker (`?bench=<chapters>`).

mod doc_view;
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
    let doc = sample::document(1).doc;
    let (report, set_report) = signal(None::<String>);

    let run_bench = move |chapters: usize| {
        set_report.set(Some(format!("Benchmark läuft ({chapters} Kapitel) …")));
        worker::run_bench(chapters, move |text| set_report.set(Some(text)));
    };
    if let Some(chapters) = bench_param() {
        run_bench(chapters);
    }

    view! {
        <style>{css}</style>
        <header class="topbar">
            <h1>"Absatz"</h1>
            <span>"M0 · Gerüst"</span>
        </header>
        <main class="layout">
            <article class="absatz-doc" lang=styles.lang.clone()>
                {doc_view::blocks(&doc, &styles)}
            </article>
            <aside class="panel">
                <h2>"Satz-Benchmark"</h2>
                <p>"Setzt ein generiertes Dokument mit Typst im Web Worker."</p>
                <button on:click=move |_| run_bench(10)>"100 Seiten messen"</button>
                <pre id="bench-result">{move || report.get()}</pre>
            </aside>
        </main>
    }
}

/// `?bench=<chapters>` starts the benchmark right away (used by e2e tests).
fn bench_param() -> Option<usize> {
    let search = web_sys::window()?.location().search().ok()?;
    let params = web_sys::UrlSearchParams::new_with_str(&search).ok()?;
    params.get("bench")?.parse().ok()
}
