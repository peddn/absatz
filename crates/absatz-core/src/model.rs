//! Plain document model.
//!
//! This is the shape every other part works with: code generation reads it,
//! storage (Loro, milestone M1) maps onto it, import/export produces it.
//! Formatting exists only as style ids – there is no direct formatting.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Document {
    pub blocks: Vec<Block>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Block {
    /// Id of a paragraph style in the active style set.
    pub style: String,
    pub content: BlockContent,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum BlockContent {
    Text(Vec<Inline>),
    /// Display formula in Typst math syntax.
    Math(String),
    Table {
        rows: Vec<Vec<Vec<Inline>>>,
        caption: Vec<Inline>,
    },
    /// Table of contents.
    Outline,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Inline {
    Text {
        text: String,
        /// Id of a character style, if any.
        style: Option<String>,
    },
    Footnote(Vec<Inline>),
}

impl Inline {
    pub fn plain(text: impl Into<String>) -> Self {
        Inline::Text {
            text: text.into(),
            style: None,
        }
    }

    pub fn styled(text: impl Into<String>, style: impl Into<String>) -> Self {
        Inline::Text {
            text: text.into(),
            style: Some(style.into()),
        }
    }
}

impl Block {
    pub fn text(style: impl Into<String>, inlines: Vec<Inline>) -> Self {
        Block {
            style: style.into(),
            content: BlockContent::Text(inlines),
        }
    }

    /// Shorthand for a block with a single unstyled text run.
    pub fn para(style: impl Into<String>, text: impl Into<String>) -> Self {
        Self::text(style, vec![Inline::plain(text)])
    }
}

/// Stored form of a document (autosave now, `document.json` in the file later).
#[derive(Serialize, Deserialize)]
struct Stored {
    version: u32,
    document: Document,
}

const FORMAT_VERSION: u32 = 1;

impl Document {
    /// A new document: one empty body paragraph.
    pub fn empty() -> Self {
        Document {
            blocks: vec![Block::text("body", Vec::new())],
        }
    }

    pub fn to_stored_json(&self) -> String {
        let stored = Stored {
            version: FORMAT_VERSION,
            document: self.clone(),
        };
        serde_json::to_string(&stored).expect("document serializes")
    }

    /// `None` for unreadable data or a format from a newer version.
    pub fn from_stored_json(json: &str) -> Option<Self> {
        let stored: Stored = serde_json::from_str(json).ok()?;
        (stored.version <= FORMAT_VERSION && !stored.document.blocks.is_empty())
            .then_some(stored.document)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stored_json_roundtrip_and_rejects_garbage() {
        let doc = crate::sample::welcome();
        assert_eq!(Document::from_stored_json(&doc.to_stored_json()), Some(doc));
        assert_eq!(Document::from_stored_json("{}"), None);
        assert_eq!(
            Document::from_stored_json(r#"{"version":99,"document":{"blocks":[]}}"#),
            None
        );
    }
}
