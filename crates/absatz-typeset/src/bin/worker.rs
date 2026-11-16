//! Web Worker entry point (built by Trunk, see crates/absatz-web/index.html).
//! Receives `ToWorker` JSON, answers with `FromWorker` JSON.

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

    let on_message = Closure::<dyn Fn(MessageEvent)>::new(move |event: MessageEvent| {
        let text = event.data().as_string().unwrap_or_default();
        let answer = match ToWorker::from_json(&text) {
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
