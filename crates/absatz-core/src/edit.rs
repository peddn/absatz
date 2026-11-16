//! Editing operations on the document model.
//!
//! Positions count UTF-16 code units, like every browser API (DOM ranges,
//! EditContext) and Loro's UTF-16 text API. A footnote counts as one unit
//! (U+FFFC in [`Document::block_text`]). Only text blocks hold positions;
//! other blocks (tables, formulas, outline) are atomic.

use crate::model::{Block, BlockContent, Document, Inline};

/// Placeholder for an atomic inline (footnote) in plain block text.
pub const ATOM: char = '\u{FFFC}';

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Pos {
    pub block: usize,
    /// UTF-16 code units from the start of the block text.
    pub offset: usize,
}

impl Pos {
    pub fn new(block: usize, offset: usize) -> Self {
        Pos { block, offset }
    }
}

impl Document {
    /// Whether the block can hold the caret.
    pub fn is_text(&self, block: usize) -> bool {
        matches!(
            self.blocks.get(block),
            Some(Block {
                content: BlockContent::Text(_),
                ..
            })
        )
    }

    /// Length of a text block in UTF-16 units (0 for atomic blocks).
    pub fn block_len(&self, block: usize) -> usize {
        match self.blocks.get(block).map(|b| &b.content) {
            Some(BlockContent::Text(inlines)) => inlines.iter().map(inline_len).sum(),
            _ => 0,
        }
    }

    /// Plain text of a block; footnotes appear as [`ATOM`].
    pub fn block_text(&self, block: usize) -> String {
        let mut s = String::new();
        if let Some(BlockContent::Text(inlines)) = self.blocks.get(block).map(|b| &b.content) {
            for inline in inlines {
                match inline {
                    Inline::Text { text, .. } => s.push_str(text),
                    Inline::Footnote(_) => s.push(ATOM),
                }
            }
        }
        s
    }

    /// Insert text (without line breaks) at a position. The new text takes
    /// the character style of the text before it. Returns the position after it.
    pub fn insert_text(&mut self, at: Pos, text: &str) -> Pos {
        debug_assert!(!text.contains('\n'), "split blocks instead");
        let Some(inlines) = self.inlines_mut(at.block) else {
            return at;
        };
        let (mut left, right) = split_inlines(std::mem::take(inlines), at.offset);
        let style = match left.last() {
            Some(Inline::Text { style, .. }) => style.clone(),
            _ => None,
        };
        left.push(Inline::Text {
            text: text.to_string(),
            style,
        });
        left.extend(right);
        *inlines = normalize(left);
        Pos::new(at.block, at.offset + utf16_len(text))
    }

    /// Delete everything between two positions (any order). Blocks in between
    /// are removed; the end block's remainder joins the start block.
    pub fn delete(&mut self, a: Pos, b: Pos) {
        let (from, to) = if a <= b { (a, b) } else { (b, a) };
        if from == to || !self.is_text(from.block) || !self.is_text(to.block) {
            return;
        }
        let tail = {
            let inlines = self.inlines_mut(to.block).expect("text block");
            let (head, tail) = split_inlines(std::mem::take(inlines), to.offset);
            *inlines = head;
            tail
        };
        if from.block != to.block {
            self.blocks.drain(from.block + 1..=to.block);
        }
        let inlines = self.inlines_mut(from.block).expect("text block");
        let mut head = split_inlines(std::mem::take(inlines), from.offset).0;
        head.extend(tail);
        *inlines = normalize(head);
    }

    /// Split a text block; the second part gets `next_style`.
    /// Returns the start of the new block.
    pub fn split_block(&mut self, at: Pos, next_style: &str) -> Pos {
        let Some(inlines) = self.inlines_mut(at.block) else {
            return at;
        };
        let (left, right) = split_inlines(std::mem::take(inlines), at.offset);
        *inlines = normalize(left);
        let new = Block::text(next_style, normalize(right));
        self.blocks.insert(at.block + 1, new);
        Pos::new(at.block + 1, 0)
    }

    /// Insert text that may contain line breaks (paste): every further line
    /// becomes a new block with `new_style`; blank lines are dropped.
    /// Returns the position after the text and the number of blocks it spans.
    pub fn insert_lines(&mut self, at: Pos, text: &str, new_style: &str) -> (Pos, usize) {
        let lines: Vec<&str> = if text.contains('\n') {
            text.lines().filter(|l| !l.trim().is_empty()).collect()
        } else {
            vec![text]
        };
        let Some((first, rest)) = lines.split_first() else {
            return (at, 1);
        };
        let mut pos = self.insert_text(at, first);
        for line in rest {
            pos = self.split_block(pos, new_style);
            pos = self.insert_text(pos, line);
        }
        (pos, lines.len())
    }

    pub fn set_style(&mut self, block: usize, style: &str) {
        if let Some(b) = self.blocks.get_mut(block) {
            b.style = style.to_string();
        }
    }

    /// Nearest caret position at or after `block`, else before it.
    pub fn nearest_text_pos(&self, block: usize) -> Option<Pos> {
        (block..self.blocks.len())
            .find(|&b| self.is_text(b))
            .map(|b| Pos::new(b, 0))
            .or_else(|| {
                (0..block.min(self.blocks.len()))
                    .rev()
                    .find(|&b| self.is_text(b))
                    .map(|b| Pos::new(b, self.block_len(b)))
            })
    }

    /// Position one character before/after `at` (by Unicode scalar, never
    /// splitting surrogate pairs). Crosses block boundaries into text blocks.
    pub fn step(&self, at: Pos, forward: bool) -> Option<Pos> {
        let text = self.block_text(at.block);
        if forward {
            if at.offset < utf16_len(&text) {
                let ch = char_at_utf16(&text, at.offset)?;
                return Some(Pos::new(at.block, at.offset + ch.len_utf16()));
            }
            (at.block + 1..self.blocks.len())
                .find(|&b| self.is_text(b))
                .map(|b| Pos::new(b, 0))
        } else {
            if at.offset > 0 {
                let ch = char_before_utf16(&text, at.offset)?;
                return Some(Pos::new(at.block, at.offset - ch.len_utf16()));
            }
            (0..at.block)
                .rev()
                .find(|&b| self.is_text(b))
                .map(|b| Pos::new(b, self.block_len(b)))
        }
    }

    fn inlines_mut(&mut self, block: usize) -> Option<&mut Vec<Inline>> {
        match self.blocks.get_mut(block).map(|b| &mut b.content) {
            Some(BlockContent::Text(inlines)) => Some(inlines),
            _ => None,
        }
    }
}

pub fn utf16_len(s: &str) -> usize {
    s.chars().map(char::len_utf16).sum()
}

fn inline_len(inline: &Inline) -> usize {
    match inline {
        Inline::Text { text, .. } => utf16_len(text),
        Inline::Footnote(_) => 1,
    }
}

/// Byte index for a UTF-16 offset, rounded down to a char boundary.
fn byte_index(s: &str, offset: usize) -> usize {
    let mut units = 0;
    for (i, c) in s.char_indices() {
        if units + c.len_utf16() > offset {
            return i;
        }
        units += c.len_utf16();
    }
    s.len()
}

fn char_at_utf16(s: &str, offset: usize) -> Option<char> {
    s[byte_index(s, offset)..].chars().next()
}

fn char_before_utf16(s: &str, offset: usize) -> Option<char> {
    s[..byte_index(s, offset)].chars().next_back()
}

fn split_inlines(inlines: Vec<Inline>, offset: usize) -> (Vec<Inline>, Vec<Inline>) {
    let (mut left, mut right) = (Vec::new(), Vec::new());
    let mut pos = 0;
    for inline in inlines {
        let len = inline_len(&inline);
        if pos + len <= offset {
            left.push(inline);
        } else if pos >= offset {
            right.push(inline);
        } else if let Inline::Text { text, style } = inline {
            let cut = byte_index(&text, offset - pos);
            left.push(Inline::Text {
                text: text[..cut].to_string(),
                style: style.clone(),
            });
            right.push(Inline::Text {
                text: text[cut..].to_string(),
                style,
            });
        }
        pos += len;
    }
    (left, right)
}

/// Drop empty runs and merge neighbours with the same character style.
fn normalize(inlines: Vec<Inline>) -> Vec<Inline> {
    let mut out: Vec<Inline> = Vec::with_capacity(inlines.len());
    for inline in inlines {
        match inline {
            Inline::Text { text, .. } if text.is_empty() => {}
            Inline::Text { text, style } => match out.last_mut() {
                Some(Inline::Text {
                    text: prev,
                    style: prev_style,
                }) if *prev_style == style => {
                    prev.push_str(&text);
                }
                _ => out.push(Inline::Text { text, style }),
            },
            other => out.push(other),
        }
    }
    out
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

    #[test]
    fn insert_counts_utf16() {
        let mut d = doc(&["a😀b"]);
        assert_eq!(d.block_len(0), 4);
        let end = d.insert_text(Pos::new(0, 3), "ü");
        assert_eq!(d.block_text(0), "a😀üb");
        assert_eq!(end, Pos::new(0, 4));
    }

    #[test]
    fn insert_takes_style_of_preceding_text() {
        let mut d = Document {
            blocks: vec![Block::text(
                "body",
                vec![
                    Inline::plain("a "),
                    Inline::styled("betont", "emphasis"),
                    Inline::plain(" z"),
                ],
            )],
        };
        d.insert_text(Pos::new(0, 8), "!");
        let BlockContent::Text(inlines) = &d.blocks[0].content else {
            panic!()
        };
        assert_eq!(inlines[1], Inline::styled("betont!", "emphasis"));
    }

    #[test]
    fn delete_within_and_across_blocks() {
        let mut d = doc(&["Hallo Welt", "zweiter", "dritter Absatz"]);
        d.delete(Pos::new(0, 5), Pos::new(0, 10));
        assert_eq!(texts(&d), ["Hallo", "zweiter", "dritter Absatz"]);
        d.delete(Pos::new(2, 8), Pos::new(0, 3));
        assert_eq!(texts(&d), ["HalAbsatz"]);
    }

    #[test]
    fn split_and_merge_roundtrip() {
        let mut d = doc(&["Überschrift und Text"]);
        d.set_style(0, "h1");
        let start = d.split_block(Pos::new(0, 11), "body");
        assert_eq!(texts(&d), ["Überschrift", " und Text"]);
        assert_eq!(
            (d.blocks[0].style.as_str(), d.blocks[1].style.as_str()),
            ("h1", "body")
        );
        d.delete(Pos::new(0, 11), start);
        assert_eq!(texts(&d), ["Überschrift und Text"]);
    }

    #[test]
    fn insert_lines_makes_blocks_and_keeps_the_tail() {
        let mut d = doc(&["ABCD"]);
        let (end, count) = d.insert_lines(Pos::new(0, 2), "eins\r\n\nzwei\n", "body");
        assert_eq!(texts(&d), ["ABeins", "zweiCD"]);
        assert_eq!((end, count), (Pos::new(1, 4), 2));
        let (end, count) = d.insert_lines(Pos::new(0, 0), "x", "body");
        assert_eq!((end, count), (Pos::new(0, 1), 1));
    }

    #[test]
    fn footnote_is_one_atomic_unit() {
        let mut d = Document {
            blocks: vec![Block::text(
                "body",
                vec![
                    Inline::plain("ab"),
                    Inline::Footnote(vec![Inline::plain("Fn")]),
                    Inline::plain("cd"),
                ],
            )],
        };
        assert_eq!(d.block_text(0), format!("ab{ATOM}cd"));
        d.delete(Pos::new(0, 1), Pos::new(0, 4));
        assert_eq!(d.block_text(0), "ad");
    }

    #[test]
    fn step_skips_atomic_blocks_and_keeps_surrogates() {
        let d = Document {
            blocks: vec![
                Block::para("body", "x😀"),
                Block {
                    style: "body".into(),
                    content: BlockContent::Outline,
                },
                Block::para("body", "y"),
            ],
        };
        assert_eq!(d.step(Pos::new(0, 1), true), Some(Pos::new(0, 3)));
        assert_eq!(d.step(Pos::new(0, 3), true), Some(Pos::new(2, 0)));
        assert_eq!(d.step(Pos::new(2, 0), false), Some(Pos::new(0, 3)));
        assert_eq!(d.nearest_text_pos(1), Some(Pos::new(2, 0)));
    }
}
