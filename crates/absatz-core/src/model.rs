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
