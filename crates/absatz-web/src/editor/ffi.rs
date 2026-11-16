//! Bindings for Chromium APIs that web-sys does not (stably) cover yet:
//! EditContext and the CSS Custom Highlight API.

use wasm_bindgen::prelude::*;
use web_sys::{DomRect, Event, EventTarget, Range};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(extends = EventTarget)]
    #[derive(Clone, Debug)]
    pub type EditContext;

    #[wasm_bindgen(constructor)]
    pub fn new() -> EditContext;

    #[wasm_bindgen(method, getter, js_name = text)]
    pub fn current_text(this: &EditContext) -> String;

    #[wasm_bindgen(method, js_name = updateText)]
    pub fn update_text(this: &EditContext, start: u32, end: u32, text: &str);

    #[wasm_bindgen(method, js_name = updateSelection)]
    pub fn update_selection(this: &EditContext, start: u32, end: u32);

    #[wasm_bindgen(method, js_name = updateControlBounds)]
    pub fn update_control_bounds(this: &EditContext, bounds: &DomRect);

    #[wasm_bindgen(method, js_name = updateSelectionBounds)]
    pub fn update_selection_bounds(this: &EditContext, bounds: &DomRect);

    #[wasm_bindgen(method, js_name = updateCharacterBounds)]
    pub fn update_character_bounds(this: &EditContext, range_start: u32, bounds: &js_sys::Array);

    #[wasm_bindgen(extends = Event)]
    pub type TextUpdateEvent;

    #[wasm_bindgen(method, getter, js_name = updateRangeStart)]
    pub fn update_range_start(this: &TextUpdateEvent) -> u32;

    #[wasm_bindgen(method, getter, js_name = updateRangeEnd)]
    pub fn update_range_end(this: &TextUpdateEvent) -> u32;

    #[wasm_bindgen(method, getter)]
    pub fn text(this: &TextUpdateEvent) -> String;

    #[wasm_bindgen(method, getter, js_name = selectionStart)]
    pub fn selection_start(this: &TextUpdateEvent) -> u32;

    #[wasm_bindgen(method, getter, js_name = selectionEnd)]
    pub fn selection_end(this: &TextUpdateEvent) -> u32;

    #[wasm_bindgen(extends = Event)]
    pub type CharacterBoundsUpdateEvent;

    #[wasm_bindgen(method, getter, js_name = rangeStart)]
    pub fn range_start(this: &CharacterBoundsUpdateEvent) -> u32;

    #[wasm_bindgen(method, getter, js_name = rangeEnd)]
    pub fn range_end(this: &CharacterBoundsUpdateEvent) -> u32;

    pub type Highlight;

    #[wasm_bindgen(constructor)]
    pub fn new(range: &Range) -> Highlight;
}

/// Attach an EditContext to an element (`element.editContext = ec`).
pub fn attach(element: &web_sys::Element, ec: &EditContext) {
    let _ = js_sys::Reflect::set(element, &"editContext".into(), ec);
}

/// Show `range` with the `::highlight(<name>)` style, or clear it.
pub fn set_highlight(name: &str, range: Option<&Range>) {
    let Some(registry) = js_sys::Reflect::get(&js_sys::global(), &"CSS".into())
        .and_then(|css| js_sys::Reflect::get(&css, &"highlights".into()))
        .ok()
    else {
        return;
    };
    let method = if range.is_some() { "set" } else { "delete" };
    if let Ok(f) = js_sys::Reflect::get(&registry, &method.into()) {
        let f: js_sys::Function = f.unchecked_into();
        let _ = match range {
            Some(range) => f.call2(&registry, &name.into(), &Highlight::new(range)),
            None => f.call1(&registry, &name.into()),
        };
    }
}
