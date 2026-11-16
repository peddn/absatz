//! Autosave in the browser's private file system (OPFS).
//! The File System API is all promises, so the few calls live in JS.

use wasm_bindgen::prelude::*;

#[wasm_bindgen(inline_js = r#"
const NAME = "dokument.json";
export async function save(text) {
  const dir = await navigator.storage.getDirectory();
  const file = await dir.getFileHandle(NAME, { create: true });
  const out = await file.createWritable();
  await out.write(text);
  await out.close();
}
export async function load() {
  try {
    const dir = await navigator.storage.getDirectory();
    const file = await (await dir.getFileHandle(NAME)).getFile();
    return await file.text();
  } catch {
    return undefined;
  }
}
"#)]
extern "C" {
    /// Replace the autosaved document.
    #[wasm_bindgen(catch)]
    pub async fn save(text: &str) -> Result<(), JsValue>;

    /// The autosaved document, if there is one.
    pub async fn load() -> JsValue;
}
