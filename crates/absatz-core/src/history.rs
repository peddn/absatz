//! Undo and redo.
//!
//! Every edit replaces a contiguous range of blocks. A [`Step`] stores that
//! range before and after, plus the selection on both sides. Undo puts the
//! old blocks back – no inverse operations to keep in sync.

use crate::edit::Pos;
use crate::model::{Block, Document};

/// Consecutive typing in one block within this time forms one undo step.
const TYPING_PAUSE_MS: f64 = 1000.0;

pub type Selection = (Pos, Pos);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Text input; merges with directly preceding typing in the same block.
    Typing,
    /// Everything else (Enter, delete, style change, …); never merges.
    Other,
}

#[derive(Clone, Debug)]
struct Step {
    at: usize,
    before: Vec<Block>,
    after: Vec<Block>,
    sel_before: Selection,
    sel_after: Selection,
    kind: Kind,
    time: f64,
}

/// What the view has to re-render after undo/redo: replace `removed` blocks
/// at `at` with `inserted` blocks, then show `selection`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Change {
    pub at: usize,
    pub removed: usize,
    pub inserted: usize,
    pub selection: Selection,
}

#[derive(Debug, Default)]
pub struct History {
    undo: Vec<Step>,
    redo: Vec<Step>,
}

/// Arguments of [`History::record`].
pub struct Edit {
    /// First affected block.
    pub at: usize,
    /// The affected blocks as they were before the edit.
    pub before: Vec<Block>,
    /// Number of blocks that replaced them.
    pub inserted: usize,
    pub sel_before: Selection,
    pub sel_after: Selection,
    pub kind: Kind,
    /// Milliseconds from any fixed origin.
    pub time: f64,
}

impl History {
    /// Record an edit that has already been applied to `doc`.
    pub fn record(&mut self, doc: &Document, edit: Edit) {
        let after = doc.blocks[edit.at..edit.at + edit.inserted].to_vec();
        self.redo.clear();
        if let Some(last) = self.undo.last_mut()
            && edit.kind == Kind::Typing
            && last.kind == Kind::Typing
            && last.at == edit.at
            && last.after.len() == 1
            && edit.before.len() == 1
            && edit.inserted == 1
            && edit.time - last.time < TYPING_PAUSE_MS
        {
            last.after = after;
            last.sel_after = edit.sel_after;
            last.time = edit.time;
            return;
        }
        self.undo.push(Step {
            at: edit.at,
            before: edit.before,
            after,
            sel_before: edit.sel_before,
            sel_after: edit.sel_after,
            kind: edit.kind,
            time: edit.time,
        });
    }

    pub fn undo(&mut self, doc: &mut Document) -> Option<Change> {
        let step = self.undo.pop()?;
        let change = replace(
            doc,
            step.at,
            step.after.len(),
            &step.before,
            step.sel_before,
        );
        self.redo.push(step);
        Some(change)
    }

    pub fn redo(&mut self, doc: &mut Document) -> Option<Change> {
        let step = self.redo.pop()?;
        let change = replace(doc, step.at, step.before.len(), &step.after, step.sel_after);
        // A redone step must never merge with the typing that follows.
        self.undo.push(Step {
            kind: Kind::Other,
            ..step
        });
        Some(change)
    }
}

fn replace(
    doc: &mut Document,
    at: usize,
    removed: usize,
    blocks: &[Block],
    selection: Selection,
) -> Change {
    doc.blocks.splice(at..at + removed, blocks.iter().cloned());
    Change {
        at,
        removed,
        inserted: blocks.len(),
        selection,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(paragraphs: &[&str]) -> Document {
        Document {
            blocks: paragraphs.iter().map(|p| Block::para("body", *p)).collect(),
        }
    }

    fn texts(d: &Document) -> Vec<String> {
        (0..d.blocks.len()).map(|b| d.block_text(b)).collect()
    }

    fn type_text(d: &mut Document, h: &mut History, at: Pos, text: &str, time: f64) -> Pos {
        let before = d.blocks[at.block..=at.block].to_vec();
        let end = d.insert_text(at, text);
        let edit = Edit {
            at: at.block,
            before,
            inserted: 1,
            sel_before: (at, at),
            sel_after: (end, end),
            kind: Kind::Typing,
            time,
        };
        h.record(d, edit);
        end
    }

    #[test]
    fn typing_coalesces_until_pause() {
        let (mut d, mut h) = (doc(&["a"]), History::default());
        let p = type_text(&mut d, &mut h, Pos::new(0, 1), "b", 0.0);
        let p = type_text(&mut d, &mut h, p, "c", 300.0);
        type_text(&mut d, &mut h, p, "d", 2000.0);
        assert_eq!(texts(&d), ["abcd"]);
        h.undo(&mut d);
        assert_eq!(texts(&d), ["abc"]);
        let change = h.undo(&mut d).unwrap();
        assert_eq!(texts(&d), ["a"]);
        assert_eq!(change.selection, (Pos::new(0, 1), Pos::new(0, 1)));
        assert!(h.undo(&mut d).is_none());
    }

    #[test]
    fn undo_redo_split_restores_block_count() {
        let (mut d, mut h) = (doc(&["Hallo Welt"]), History::default());
        let before = d.blocks[0..1].to_vec();
        let new = d.split_block(Pos::new(0, 5), "body");
        let sel = (Pos::new(0, 5), Pos::new(0, 5));
        let edit = Edit {
            at: 0,
            before,
            inserted: 2,
            sel_before: sel,
            sel_after: (new, new),
            kind: Kind::Other,
            time: 0.0,
        };
        h.record(&d, edit);
        assert_eq!(texts(&d), ["Hallo", " Welt"]);

        let change = h.undo(&mut d).unwrap();
        assert_eq!((change.at, change.removed, change.inserted), (0, 2, 1));
        assert_eq!(texts(&d), ["Hallo Welt"]);

        let change = h.redo(&mut d).unwrap();
        assert_eq!((change.at, change.removed, change.inserted), (0, 1, 2));
        assert_eq!(texts(&d), ["Hallo", " Welt"]);
    }

    #[test]
    fn new_edit_clears_redo() {
        let (mut d, mut h) = (doc(&["x"]), History::default());
        type_text(&mut d, &mut h, Pos::new(0, 1), "y", 0.0);
        h.undo(&mut d);
        type_text(&mut d, &mut h, Pos::new(0, 1), "z", 5000.0);
        assert!(h.redo(&mut d).is_none());
        assert_eq!(texts(&d), ["xz"]);
    }
}
