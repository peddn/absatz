//! Writing view: semantic DOM + EditContext.
//!
//! The browser lays out text, draws caret and selection, and runs the IME.
//! We own the model: every input event becomes a model operation, after
//! which the affected block elements are re-rendered and the selection is
//! restored. Every operation is recorded in the history (undo/redo).
//! The EditContext mirrors the text of the focused block only.

mod dom;
mod ffi;

use std::cell::RefCell;
use std::rc::Rc;

use absatz_core::edit::{Pos, utf16_len};
use absatz_core::history::{Change, Edit, History, Kind, Selection};
use absatz_core::model::{Block, Document};
use absatz_core::styles::StyleSet;
use wasm_bindgen::{JsCast, JsValue, closure::Closure};
use web_sys::{Element, Event, EventTarget, HtmlElement, KeyboardEvent};

use ffi::{CharacterBoundsUpdateEvent, EditContext, TextUpdateEvent};

struct Editor {
    doc: Document,
    styles: StyleSet,
    root: Element,
    dom: web_sys::Document,
    ec: EditContext,
    anchor: Pos,
    focus: Pos,
    /// Block whose text the EditContext currently holds.
    ec_block: usize,
    composing: bool,
    history: History,
}

/// Whether this browser can edit: EditContext exists only in Chromium (≥ 121).
pub fn supported() -> bool {
    js_sys::Reflect::has(&js_sys::global(), &"EditContext".into()).unwrap_or(false)
}

/// Render the document into `root` and make it editable if [`supported`].
pub fn mount(root: HtmlElement, doc: Document, styles: StyleSet) {
    let dom = root.owner_document().expect("attached to a document");
    for block in &doc.blocks {
        let _ = root.append_child(&dom::render_block(&dom, block, &styles));
    }
    if !supported() {
        return; // read-only; the app shows a notice (decision #2: no fallbacks)
    }
    let _ = root.set_attribute("role", "textbox");
    let _ = root.set_attribute("aria-multiline", "true");
    root.set_spellcheck(true);

    let ec = EditContext::new();
    ffi::attach(&root, &ec);
    let start = doc.nearest_text_pos(0).unwrap_or_default();
    let editor = Rc::new(RefCell::new(Editor {
        doc,
        styles,
        root: root.into(),
        dom: dom.clone(),
        ec: ec.clone(),
        anchor: start,
        focus: start,
        ec_block: usize::MAX,
        composing: false,
        history: History::default(),
    }));
    editor.borrow_mut().sync_edit_context(true);

    listen(&ec, "textupdate", &editor, |ed, ev| {
        ed.on_text_update(ev.unchecked_ref())
    });
    listen(&ec, "characterboundsupdate", &editor, |ed, ev| {
        ed.on_character_bounds(ev.unchecked_ref())
    });
    listen(&ec, "compositionstart", &editor, |ed, _| {
        ed.composing = true
    });
    listen(&ec, "compositionend", &editor, |ed, _| {
        ed.composing = false;
        ffi::set_highlight("ime", None);
    });
    let root = editor.borrow().root.clone();
    listen(&root, "keydown", &editor, |ed, ev| {
        ed.on_keydown(ev.unchecked_ref())
    });
    listen(&dom, "selectionchange", &editor, |ed, _| {
        ed.on_selection_change()
    });
}

fn listen(
    target: &EventTarget,
    name: &str,
    editor: &Rc<RefCell<Editor>>,
    f: fn(&mut Editor, Event),
) {
    let editor = editor.clone();
    let cb = Closure::<dyn Fn(Event)>::new(move |ev: Event| {
        if let Ok(mut ed) = editor.try_borrow_mut() {
            f(&mut ed, ev);
        }
    });
    let _ = target.add_event_listener_with_callback(name, cb.as_ref().unchecked_ref());
    cb.forget();
}

impl Editor {
    fn ordered(&self) -> (Pos, Pos) {
        if self.anchor <= self.focus {
            (self.anchor, self.focus)
        } else {
            (self.focus, self.anchor)
        }
    }

    fn on_text_update(&mut self, ev: &TextUpdateEvent) {
        let started = now();
        let text = ev.text().replace(['\n', '\r'], " ");
        let sel_before = (self.anchor, self.focus);
        let (from, to) = self.ordered();
        if !self.composing && from.block != to.block {
            // The EditContext only knows the focused block; replace the whole selection.
            let before = self.blocks(from.block, to.block);
            self.doc.delete(from, to);
            let caret = self.doc.insert_text(from, &text);
            self.rerender_range(from.block, to.block - from.block + 1, 1);
            self.select(caret, caret);
            self.record(from.block, before, 1, sel_before, Kind::Other);
        } else {
            let b = self.ec_block;
            let before = self.blocks(b, b);
            let (rs, re) = (
                ev.update_range_start() as usize,
                ev.update_range_end() as usize,
            );
            self.doc.delete(Pos::new(b, rs), Pos::new(b, re));
            self.doc.insert_text(Pos::new(b, rs), &text);
            if !dom::replace_text(&self.root, b, rs, re, &text) {
                self.rerender_range(b, 1, 1);
            }
            self.anchor = Pos::new(b, ev.selection_start() as usize);
            self.focus = Pos::new(b, ev.selection_end() as usize);
            dom::set_selection(&self.root, self.anchor, self.focus);
            if self.composing {
                let end = Pos::new(b, rs + utf16_len(&text));
                ffi::set_highlight("ime", dom::range(&self.root, Pos::new(b, rs), end).as_ref());
            }
            self.update_bounds();
            self.record(b, before, 1, sel_before, Kind::Typing);
        }
        record_latency(started, ev.time_stamp());
    }

    fn on_keydown(&mut self, ev: &KeyboardEvent) {
        if ev.is_composing() || self.composing {
            return;
        }
        let started = now();
        // `selectionchange` is async: after fast arrow keys the EditContext may
        // still hold an old caret. keydown always precedes the text input, so
        // take the DOM selection over synchronously here.
        self.on_selection_change();
        let ctrl = ev.ctrl_key() || ev.meta_key();
        let sel_before = (self.anchor, self.focus);
        let (from, to) = self.ordered();
        match ev.key().as_str() {
            "Enter" => {
                ev.prevent_default();
                if !self.doc.is_text(from.block) {
                    return;
                }
                let before = self.blocks(from.block, to.block);
                self.doc.delete(from, to);
                let next = self
                    .styles
                    .next_style(&self.doc.blocks[from.block].style)
                    .to_string();
                let new = self.doc.split_block(from, &next);
                self.rerender_range(from.block, to.block - from.block + 1, 2);
                self.select(new, new);
                self.record(from.block, before, 2, sel_before, Kind::Other);
            }
            key @ ("Backspace" | "Delete") => {
                // Inside one block the browser deletes via the EditContext
                // (characters, words, grapheme clusters). We only handle
                // deletions that cross a block boundary.
                let (from, to) = if from != to {
                    if from.block == to.block {
                        return;
                    }
                    (from, to)
                } else {
                    let Some(other) = self.doc.step(self.focus, key == "Delete") else {
                        return;
                    };
                    if other.block == self.focus.block {
                        return;
                    }
                    if other.block.abs_diff(self.focus.block) > 1 {
                        ev.prevent_default();
                        return; // never swallow a table or formula in between
                    }
                    if other < self.focus {
                        (other, self.focus)
                    } else {
                        (self.focus, other)
                    }
                };
                ev.prevent_default();
                let before = self.blocks(from.block, to.block);
                self.doc.delete(from, to);
                self.rerender_range(from.block, to.block - from.block + 1, 1);
                self.select(from, from);
                self.record(from.block, before, 1, sel_before, Kind::Other);
            }
            "z" | "Z" | "y" if ctrl && !ev.alt_key() => {
                ev.prevent_default();
                let redo = ev.key() == "y" || ev.shift_key();
                let change = if redo {
                    self.history.redo(&mut self.doc)
                } else {
                    self.history.undo(&mut self.doc)
                };
                if let Some(change) = change {
                    self.apply(change);
                }
            }
            _ if ctrl && ev.alt_key() => {
                let style = match ev.code().as_str() {
                    "Digit0" => "body",
                    "Digit1" => "h1",
                    "Digit2" => "h2",
                    "Digit3" => "h3",
                    _ => return,
                };
                ev.prevent_default();
                let before = self.blocks(from.block, to.block);
                for b in from.block..=to.block {
                    if self.doc.is_text(b) {
                        self.doc.set_style(b, style);
                    }
                }
                let count = to.block - from.block + 1;
                self.rerender_range(from.block, count, count);
                dom::set_selection(&self.root, self.anchor, self.focus);
                self.record(from.block, before, count, sel_before, Kind::Other);
            }
            _ => return,
        }
        record_latency(started, ev.time_stamp());
    }

    fn on_selection_change(&mut self) {
        if self.composing {
            return;
        }
        let Some(sel) = self.dom.get_selection().ok().flatten() else {
            return;
        };
        let (Some(an), Some(fnode)) = (sel.anchor_node(), sel.focus_node()) else {
            return;
        };
        if !self.root.contains(Some(&an)) || !self.root.contains(Some(&fnode)) {
            return;
        }
        let anchor = dom::pos_from_dom(&self.root, &self.doc, &an, sel.anchor_offset());
        let focus = dom::pos_from_dom(&self.root, &self.doc, &fnode, sel.focus_offset());
        if let (Some(anchor), Some(focus)) = (anchor, focus)
            && (anchor, focus) != (self.anchor, self.focus)
        {
            self.anchor = anchor;
            self.focus = focus;
            self.sync_edit_context(false);
        }
    }

    fn on_character_bounds(&mut self, ev: &CharacterBoundsUpdateEvent) {
        let b = self.ec_block;
        let bounds = js_sys::Array::new();
        for i in ev.range_start()..ev.range_end() {
            let (i, j) = (Pos::new(b, i as usize), Pos::new(b, i as usize + 1));
            if let Some(r) = dom::range(&self.root, i, j) {
                bounds.push(&r.get_bounding_client_rect());
            }
        }
        self.ec.update_character_bounds(ev.range_start(), &bounds);
    }

    /// Copy of blocks `first..=last`, taken before an edit for the history.
    fn blocks(&self, first: usize, last: usize) -> Vec<Block> {
        self.doc.blocks[first..=last].to_vec()
    }

    /// Record an already applied edit; the current selection is the one after it.
    fn record(
        &mut self,
        at: usize,
        before: Vec<Block>,
        inserted: usize,
        sel_before: Selection,
        kind: Kind,
    ) {
        let sel_after = (self.anchor, self.focus);
        let edit = Edit {
            at,
            before,
            inserted,
            sel_before,
            sel_after,
            kind,
            time: now(),
        };
        self.history.record(&self.doc, edit);
    }

    /// Show the result of undo/redo.
    fn apply(&mut self, change: Change) {
        self.rerender_range(change.at, change.removed, change.inserted);
        self.select(change.selection.0, change.selection.1);
    }

    fn select(&mut self, anchor: Pos, focus: Pos) {
        self.anchor = anchor;
        self.focus = focus;
        dom::set_selection(&self.root, anchor, focus);
        self.sync_edit_context(true);
    }

    /// Replace `removed` block elements starting at `first` with renders of
    /// `inserted` model blocks starting at `first`.
    fn rerender_range(&self, first: usize, removed: usize, inserted: usize) {
        let children = self.root.children();
        let reference = children.item((first + removed) as u32);
        for _ in 0..removed {
            if let Some(old) = children.item(first as u32) {
                old.remove();
            }
        }
        for block in &self.doc.blocks[first..first + inserted] {
            let el = dom::render_block(&self.dom, block, &self.styles);
            let _ = self.root.insert_before(&el, reference.as_deref());
        }
    }

    /// Mirror the focused block and the selection into the EditContext.
    fn sync_edit_context(&mut self, force_text: bool) {
        let b = self.focus.block;
        if force_text || b != self.ec_block {
            let old_len = utf16_len(&self.ec.current_text()) as u32;
            self.ec.update_text(0, old_len, &self.doc.block_text(b));
            self.ec_block = b;
        }
        let (s, e) = if self.anchor.block == b {
            (
                self.anchor.offset.min(self.focus.offset),
                self.anchor.offset.max(self.focus.offset),
            )
        } else {
            (self.focus.offset, self.focus.offset)
        };
        self.ec.update_selection(s as u32, e as u32);
        self.update_bounds();
    }

    /// Tell the IME where the editor and the caret are on screen.
    fn update_bounds(&self) {
        self.ec
            .update_control_bounds(&self.root.get_bounding_client_rect());
        if let Some(r) = dom::range(&self.root, self.focus, self.focus) {
            self.ec
                .update_selection_bounds(&r.get_bounding_client_rect());
        }
    }
}

fn now() -> f64 {
    web_sys::window()
        .and_then(|w| w.performance())
        .map_or(0.0, |p| p.now())
}

/// Latency per input, collected only while `window.__absatzLatency` is an
/// array (set `__absatzLatency = []` in the console or a test):
/// `handler` = our work, `frame` = event creation → next animation frame.
fn record_latency(started: f64, event_time: f64) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let key = JsValue::from_str("__absatzLatency");
    let Some(list) = js_sys::Reflect::get(&window, &key)
        .ok()
        .and_then(|v| v.dyn_into::<js_sys::Array>().ok())
    else {
        return;
    };
    let entry = js_sys::Object::new();
    let _ = js_sys::Reflect::set(&entry, &"handler".into(), &(now() - started).into());
    list.push(&entry);
    let frame = Closure::once_into_js(move || {
        let _ = js_sys::Reflect::set(&entry, &"frame".into(), &(now() - event_time).into());
    });
    let _ = window.request_animation_frame(frame.unchecked_ref());
}
