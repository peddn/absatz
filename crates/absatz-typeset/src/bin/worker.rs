//! Web Worker entry point (built by Trunk, see crates/absatz-web/index.html).
//! Receives `ToWorker` JSON, answers with `FromWorker` JSON (PDF: bytes).

#[cfg(target_arch = "wasm32")]
fn main() {
    use absatz_core::protocol::{FromWorker, ToWorker};
    use wasm_bindgen::{JsCast, JsValue, closure::Closure};
    use web_sys::{DedicatedWorkerGlobalScope, MessageEvent};

    console_error_panic_hook::set_once();
    let scope: DedicatedWorkerGlobalScope = js_sys::global().unchecked_into();
    let performance = scope
        .performance()
        .expect("performance available in workers");

    let reply_scope = scope.clone();
    let reply = move |msg: FromWorker| {
        let _ = reply_scope.post_message(&JsValue::from_str(&msg.to_json()));
    };

    // Created with the first message so `Ready` is not delayed by font loading.
    let typesetter = std::cell::RefCell::new(None::<absatz_typeset::Typesetter>);
    let binary_scope = scope.clone();

    let on_message = Closure::<dyn Fn(MessageEvent)>::new(move |event: MessageEvent| {
        let text = event.data().as_string().unwrap_or_default();
        let mut slot = typesetter.borrow_mut();
        let ts = slot
            .get_or_insert_with(|| absatz_typeset::Typesetter::new(&absatz_core::styles::STANDARD));
        let answer = match ToWorker::from_json(&text) {
            Ok(ToWorker::Update {
                source,
                first,
                last,
            }) => {
                if let Some(source) = source {
                    ts.set_source(&source);
                }
                let error = ts.compile().err();
                let svgs = match error {
                    None => (first..=last).map_while(|i| ts.page_svg(i)).collect(),
                    Some(_) => Vec::new(),
                };
                FromWorker::Pages {
                    count: ts.page_count(),
                    first,
                    svgs,
                    error,
                }
            }
            Ok(ToWorker::Pdf { source }) => {
                ts.set_source(&source);
                match ts.compile().map(|_| ()).and_then(|()| ts.pdf()) {
                    Ok(pdf) => {
                        let bytes = js_sys::Uint8Array::from(pdf.as_slice());
                        let _ = binary_scope.post_message(&bytes);
                        return;
                    }
                    Err(message) => FromWorker::Error { message },
                }
            }
            Ok(ToWorker::Bench { chapters }) => {
                FromWorker::Bench(absatz_typeset::bench::run(chapters, &|| performance.now()))
            }
            Err(e) => FromWorker::Error {
                message: format!("bad message: {e}"),
            },
        };
        reply(answer);
    });
    scope.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
    on_message.forget();

    // Messages sent before this point are lost; the app waits for `Ready`.
    let _ = scope.post_message(&JsValue::from_str(&FromWorker::Ready.to_json()));
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    eprintln!("The worker only runs in the browser. Natively, use: cargo run --example bench");
}
