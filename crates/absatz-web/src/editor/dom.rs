//! The document surface in the DOM.
//!
//! Child `i` of the editor root renders block `i`. Inside a text block the
//! children are text nodes, `span.c-<style>` (one text node each) and
//! `sup.fn` footnote markers that count as one position. This module is the
//! only place that knows this structure.

use absatz_core::edit::{Pos, utf16_len};
use absatz_core::model::{Block, BlockContent, Document as Doc, Inline};
use absatz_core::styles::{Role, StyleSet};
use wasm_bindgen::JsCast;
use web_sys::{CharacterData, Document, Element, Node};

pub fn render_block(dom: &Document, block: &Block, styles: &StyleSet) -> Element {
    let el = |tag: &str| dom.create_element(tag).expect("valid tag");
    match &block.content {
        BlockContent::Text(inlines) => {
            let tag = match styles
                .paragraph
                .get(&block.style)
                .map(|s| (s.role, s.level))
            {
                Some((Role::Heading, Some(1))) => "h1",
                Some((Role::Heading, Some(2))) => "h2",
                Some((Role::Heading, _)) => "h3",
                Some((Role::Quote, _)) => "blockquote",
                _ => "p",
            };
            let e = el(tag);
            e.set_class_name(&format!("p-{}", block.style));
            for inline in inlines {
                let child: Node = match inline {
                    Inline::Text { text, style: None } => dom.create_text_node(text).into(),
                    Inline::Text {
                        text,
                        style: Some(s),
                    } => {
                        let span = el("span");
                        span.set_class_name(&format!("c-{s}"));
                        span.set_text_content(Some(text));
                        span.into()
                    }
                    Inline::Footnote(content) => {
                        let sup = el("sup");
                        sup.set_class_name("fn");
                        sup.set_text_content(Some("*"));
                        let _ = sup.set_attribute("title", &plain(content));
                        sup.into()
                    }
                };
                let _ = e.append_child(&child);
            }
            if inlines.is_empty() {
                // An empty line still needs height and a caret position.
                let _ = e.append_child(&el("br"));
            }
            e
        }
        BlockContent::Math(src) => {
            let e = el("div");
            e.set_class_name("math atomic");
            e.set_text_content(Some(src));
            e
        }
        BlockContent::Table { rows, caption } => {
            let fig = el("figure");
            fig.set_class_name("atomic");
            let table = el("table");
            for row in rows {
                let tr = el("tr");
                for cell in row {
                    let td = el("td");
                    td.set_text_content(Some(&plain(cell)));
                    let _ = tr.append_child(&td);
                }
                let _ = table.append_child(&tr);
            }
            let cap = el("figcaption");
            cap.set_text_content(Some(&plain(caption)));
            let _ = fig.append_child(&table);
            let _ = fig.append_child(&cap);
            fig
        }
        BlockContent::Outline => {
            let e = el("nav");
            e.set_class_name("toc atomic");
            e.set_text_content(Some("Inhaltsverzeichnis (erscheint in der Seitenansicht)"));
            e
        }
    }
}

fn plain(inlines: &[Inline]) -> String {
    inlines
        .iter()
        .map(|i| match i {
            Inline::Text { text, .. } => text.clone(),
            Inline::Footnote(c) => plain(c),
        })
        .collect()
}

/// Length of a direct child of a block element in positions.
fn child_len(child: &Node) -> usize {
    match child.dyn_ref::<Element>() {
        Some(e) if e.class_name() == "fn" => 1,
        Some(e) if e.tag_name() == "BR" => 0,
        _ => utf16_len(&child.text_content().unwrap_or_default()),
    }
}

/// Index of the block element containing `node`.
pub fn block_index(root: &Element, node: &Node) -> Option<usize> {
    let root: &Node = root.as_ref();
    let mut cur = node.clone();
    while cur.parent_node().as_ref() != Some(root) {
        cur = cur.parent_node()?;
    }
    let mut index = 0;
    let mut sibling = cur.dyn_ref::<Element>()?.previous_element_sibling();
    while let Some(s) = sibling {
        index += 1;
        sibling = s.previous_element_sibling();
    }
    Some(index)
}

/// DOM point → model position (caret positions only in text blocks).
pub fn pos_from_dom(root: &Element, doc: &Doc, node: &Node, offset: u32) -> Option<Pos> {
    let root_node: &Node = root.as_ref();
    let block = if node == root_node {
        (offset as usize).min(doc.blocks.len().saturating_sub(1))
    } else {
        block_index(root, node)?
    };
    if !doc.is_text(block) {
        return doc.nearest_text_pos(block);
    }
    if node == root_node {
        return Some(Pos::new(block, 0));
    }
    let block_el: Node = root.children().item(block as u32)?.into();
    let children = block_el.child_nodes();
    let mut units = 0;
    for i in 0..children.length() {
        let child = children.item(i)?;
        if block_el == *node && i == offset {
            return Some(Pos::new(block, units));
        }
        let len = child_len(&child);
        if child.contains(Some(node)) {
            let in_text =
                child.node_type() == Node::TEXT_NODE || child.first_child().as_ref() == Some(node);
            let inner = if in_text {
                (offset as usize).min(len)
            } else if offset > 0 {
                len
            } else {
                0
            };
            return Some(Pos::new(block, units + inner));
        }
        units += len;
    }
    Some(Pos::new(block, units))
}

/// Model position → DOM point. At run boundaries the earlier run wins, so
/// typed text continues the style of the text before the caret.
pub fn dom_point(root: &Element, pos: Pos) -> Option<(Node, u32)> {
    let block_el: Node = root.children().item(pos.block as u32)?.into();
    let children = block_el.child_nodes();
    let mut units = 0;
    for i in 0..children.length() {
        let child = children.item(i)?;
        let len = child_len(&child);
        let is_atom = child
            .dyn_ref::<Element>()
            .is_some_and(|e| e.class_name() == "fn");
        if is_atom {
            if pos.offset == units {
                return Some((block_el, i));
            }
        } else if len > 0 && pos.offset <= units + len {
            let text = if child.node_type() == Node::TEXT_NODE {
                child
            } else {
                child.first_child()?
            };
            return Some((text, (pos.offset - units) as u32));
        }
        units += len;
    }
    Some((block_el.clone(), children.length()))
}

/// Make the DOM selection show `anchor`..`focus`.
pub fn set_selection(root: &Element, anchor: Pos, focus: Pos) {
    let (Some((an, ao)), Some((fnode, fo))) = (dom_point(root, anchor), dom_point(root, focus))
    else {
        return;
    };
    if let Some(sel) = web_sys::window().and_then(|w| w.get_selection().ok().flatten()) {
        let _ = sel.set_base_and_extent(&an, ao, &fnode, fo);
    }
}

/// DOM range covering model positions `from..to` of one block.
pub fn range(root: &Element, from: Pos, to: Pos) -> Option<web_sys::Range> {
    let (sn, so) = dom_point(root, from)?;
    let (en, eo) = dom_point(root, to)?;
    let r = root.owner_document()?.create_range().ok()?;
    r.set_start(&sn, so).ok()?;
    r.set_end(&en, eo).ok()?;
    Some(r)
}

/// Replace positions `from..to` of a block by editing one text node in place.
/// Keeping the node keeps what the browser attached to it (spellcheck markers).
/// Returns false when the edit is not a plain in-node change that matches the
/// model; the caller then re-renders the block.
pub fn replace_text(root: &Element, block: usize, from: usize, to: usize, text: &str) -> bool {
    let (Some((start, so)), Some((end, eo))) = (
        dom_point(root, Pos::new(block, from)),
        dom_point(root, Pos::new(block, to)),
    ) else {
        return false;
    };
    let Some(data) = start.dyn_ref::<CharacterData>() else {
        return false;
    };
    if start != end || eo < so {
        return false;
    }
    // At the very start of a styled run the model inserts unstyled text.
    let styled = start.parent_node().is_some_and(|p| p.node_name() == "SPAN");
    // An emptied run disappears from the model (and an empty block needs <br>).
    let new_len = data.length() - (eo - so) + utf16_len(text) as u32;
    if (so == 0 && styled) || new_len == 0 {
        return false;
    }
    data.replace_data(so, eo - so, text).is_ok()
}
