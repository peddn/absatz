//! Typst code generation from the document model.
//!
//! All user text is emitted as Typst string literals (`#"…"`), so only `"`
//! and `\` need escaping and markup characters can never change meaning.
//! Each block becomes one call of the style set's `blk` function; the
//! template decides the element (heading, list item, …) from the style role.

use std::ops::Range;

use crate::model::{Block, BlockContent, Document, Inline};

/// Header of every generated document. `template.typ` and `styles.json`
/// are provided next to it by the typesetter.
pub const PRELUDE: &str = "#import \"template.typ\": apply, blk, cs\n#show: apply\n\n";

pub struct Generated {
    pub source: String,
    /// Byte range of each block in `source`, same order as `Document::blocks`.
    /// Used to map positions in the page preview back to blocks.
    pub block_ranges: Vec<Range<usize>>,
}

pub fn typst(doc: &Document) -> Generated {
    let mut out = String::with_capacity(doc.blocks.len() * 400 + PRELUDE.len());
    out.push_str(PRELUDE);
    let mut block_ranges = Vec::with_capacity(doc.blocks.len());
    for block in &doc.blocks {
        let start = out.len();
        push_block(&mut out, block);
        block_ranges.push(start..out.len());
        out.push_str("\n\n");
    }
    Generated {
        source: out,
        block_ranges,
    }
}

fn push_block(out: &mut String, block: &Block) {
    match &block.content {
        BlockContent::Text(inlines) => {
            out.push_str("#blk(");
            push_str_lit(out, &block.style);
            out.push_str(")[");
            push_inlines(out, inlines);
            out.push(']');
        }
        BlockContent::Math(src) => {
            out.push_str("$ ");
            out.push_str(src);
            out.push_str(" $");
        }
        BlockContent::Table { rows, caption } => {
            let columns = rows.iter().map(Vec::len).max().unwrap_or(1);
            out.push_str(&format!("#figure(table(columns: {columns}"));
            for row in rows {
                for cell in row {
                    out.push_str(", [");
                    push_inlines(out, cell);
                    out.push(']');
                }
            }
            out.push_str("), caption: [");
            push_inlines(out, caption);
            out.push_str("])");
        }
        BlockContent::Outline => out.push_str("#outline()"),
    }
}

fn push_inlines(out: &mut String, inlines: &[Inline]) {
    for inline in inlines {
        match inline {
            Inline::Text { text, style: None } => push_text(out, text),
            Inline::Text {
                text,
                style: Some(style),
            } => {
                out.push_str("#cs(");
                push_str_lit(out, style);
                out.push_str(")[");
                push_text(out, text);
                out.push(']');
            }
            Inline::Footnote(content) => {
                out.push_str("#footnote[");
                push_inlines(out, content);
                out.push(']');
            }
        }
    }
}

fn push_text(out: &mut String, text: &str) {
    if !text.is_empty() {
        out.push('#');
        push_str_lit(out, text);
    }
}

fn push_str_lit(out: &mut String, text: &str) {
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => {}
            c => out.push(c),
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Inline;

    #[test]
    fn escapes_markup_and_quotes() {
        let doc = Document {
            blocks: vec![Block::text(
                "body",
                vec![
                    Inline::plain("#set \"quoted\" \\ *not bold* $x$ "),
                    Inline::styled("betont", "emphasis"),
                    Inline::Footnote(vec![Inline::plain("Fußnote")]),
                ],
            )],
        };
        insta::assert_snapshot!(typst(&doc).source);
    }

    #[test]
    fn block_ranges_cover_blocks() {
        let doc = Document {
            blocks: vec![Block::para("h1", "Titel"), Block::para("body", "Text")],
        };
        let generated = typst(&doc);
        let first = &generated.source[generated.block_ranges[0].clone()];
        assert_eq!(first, "#blk(\"h1\")[#\"Titel\"]");
    }
}
