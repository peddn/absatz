//! Runs the typesetting benchmark in its own worker (`?bench=`).

use absatz_core::protocol::{FromWorker, ToWorker};
use wasm_bindgen::{JsCast, JsValue, closure::Closure};
use web_sys::{MessageEvent, Worker, WorkerOptions, WorkerType};

use crate::preview::WORKER_URL;

pub fn run_bench(chapters: usize, on_done: impl Fn(String) + 'static) {
    let options = WorkerOptions::new();
    options.set_type(WorkerType::Module);
    let worker = match Worker::new_with_options(WORKER_URL, &options) {
        Ok(worker) => worker,
        Err(e) => return on_done(format!("Worker startet nicht: {e:?}")),
    };

    let sender = worker.clone();
    let on_message = Closure::<dyn Fn(MessageEvent)>::new(move |event: MessageEvent| {
        let text = event.data().as_string().unwrap_or_default();
        match FromWorker::from_json(&text) {
            Ok(FromWorker::Ready) => {
                let msg = ToWorker::Bench { chapters }.to_json();
                let _ = sender.post_message(&JsValue::from_str(&msg));
            }
            Ok(FromWorker::Bench(report)) => {
                on_done(report.to_text());
                sender.terminate();
            }
            Ok(FromWorker::Error { message }) => on_done(format!("Fehler im Worker: {message}")),
            Ok(FromWorker::Pages { .. }) => {}
            Err(e) => on_done(format!("Unlesbare Nachricht: {e}")),
        }
    });
    worker.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
    on_message.forget();
}
