//! Style sets: the single source of truth for all formatting.
//!
//! A style set is a `tokens.json` plus a Typst `template.typ`. The tokens
//! drive both outputs: the Typst template reads them as `styles.json`, and
//! [`StyleSet::to_css`] turns them into CSS for the writing view.

use std::collections::BTreeMap;
use std::fmt::Write;

use serde::{Deserialize, Serialize};

/// The raw files of a style set as shipped with the app.
#[derive(Clone, Copy, Debug)]
pub struct StyleSetSource {
    pub tokens_json: &'static str,
    pub template_typ: &'static str,
}

pub const STANDARD: StyleSetSource = StyleSetSource {
    tokens_json: include_str!("../stylesets/standard/tokens.json"),
    template_typ: include_str!("../stylesets/standard/template.typ"),
};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StyleSet {
    pub id: String,
    pub name: String,
    pub lang: String,
    pub page: Page,
    pub fonts: Fonts,
    pub paragraph: BTreeMap<String, ParagraphStyle>,
    pub character: BTreeMap<String, CharacterStyle>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Page {
    pub paper: String,
    pub margin_x_mm: f64,
    pub margin_y_mm: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Fonts {
    pub body: String,
    pub mono: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Body,
    Heading,
    ListItem,
    Quote,
    Note,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CharRole {
    Emphasis,
    Strong,
    Code,
    Custom,
}

/// Visual properties shared by paragraph and character styles.
/// Absent values are omitted from JSON so the Typst template can test with `in`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Look {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub italic: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bold: Option<bool>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ParagraphStyle {
    pub name: String,
    pub role: Role,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level: Option<u8>,
    #[serde(flatten)]
    pub look: Look,
    /// Left indent in pt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub indent: Option<f64>,
    /// Space between lines in em (Typst `par.leading`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub leading: Option<f64>,
    /// Space between paragraphs in pt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spacing: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub justify: Option<bool>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CharacterStyle {
    pub name: String,
    pub role: CharRole,
    #[serde(flatten)]
    pub look: Look,
}

impl StyleSet {
    pub fn parse(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    pub fn standard() -> Self {
        Self::parse(STANDARD.tokens_json).expect("bundled style set is valid")
    }

    /// Style of the block created by Enter: headings are followed by body text,
    /// everything else continues with the same style.
    pub fn next_style<'a>(&self, id: &'a str) -> &'a str {
        match self.paragraph.get(id).map(|s| s.role) {
            Some(Role::Heading) => "body",
            _ => id,
        }
    }

    /// JSON as read by the Typst template (`styles.json`).
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("style set serializes")
    }

    /// CSS for the writing view. Scoped to `.absatz-doc`; paragraph styles
    /// map to `.p-<id>`, character styles to `.c-<id>`.
    pub fn to_css(&self) -> String {
        let mut css = format!(
            ".absatz-doc {{ font-family: \"{}\", serif; hyphens: auto; {}}}\n",
            self.fonts.body,
            self.paragraph
                .get("body")
                .map(inherited_decls)
                .unwrap_or_default()
        );
        for (id, style) in &self.paragraph {
            let _ = writeln!(css, ".absatz-doc .p-{id} {{ {}}}", paragraph_decls(style));
        }
        for (id, style) in &self.character {
            let mut decls = look_decls(&style.look);
            // The role implies the look, exactly like `cs()` in template.typ.
            match style.role {
                CharRole::Emphasis => decls.push_str("font-style: italic; "),
                CharRole::Strong => decls.push_str("font-weight: 700; "),
                CharRole::Code => {
                    let _ = write!(decls, "font-family: \"{}\", monospace; ", self.fonts.mono);
                }
                CharRole::Custom => {}
            }
            let _ = writeln!(css, ".absatz-doc .c-{id} {{ {decls}}}");
        }
        css
    }
}

/// Declarations of the body style that every block inherits.
fn inherited_decls(body: &ParagraphStyle) -> String {
    let mut d = look_decls(&body.look);
    if let Some(leading) = body.leading {
        // Typst's leading is the gap between lines; CSS needs the full line box.
        let _ = write!(d, "line-height: {}; ", 1.0 + leading);
    }
    d
}

fn look_decls(look: &Look) -> String {
    let mut d = String::new();
    if let Some(size) = look.size {
        let _ = write!(d, "font-size: {size}pt; ");
    }
    if let Some(color) = &look.color {
        let _ = write!(d, "color: {color}; ");
    }
    if let Some(italic) = look.italic {
        d.push_str(if italic {
            "font-style: italic; "
        } else {
            "font-style: normal; "
        });
    }
    if let Some(bold) = look.bold {
        d.push_str(if bold {
            "font-weight: 700; "
        } else {
            "font-weight: 400; "
        });
    }
    d
}

fn paragraph_decls(style: &ParagraphStyle) -> String {
    let mut d = inherited_decls(style);
    if let Some(indent) = style.indent {
        let _ = write!(d, "padding-inline-start: {indent}pt; ");
    }
    if let Some(spacing) = style.spacing {
        let _ = write!(d, "margin-block-end: {spacing}pt; ");
    }
    if let Some(justify) = style.justify {
        d.push_str(if justify {
            "text-align: justify; "
        } else {
            "text-align: start; "
        });
    }
    if style.role == Role::ListItem {
        // `inside`: blocks use `content-visibility`, which clips outside markers.
        d.push_str("display: list-item; list-style: disc inside; ");
    }
    d
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_roundtrips_through_json() {
        let set = StyleSet::standard();
        let again = StyleSet::parse(&set.to_json()).unwrap();
        assert_eq!(set, again);
    }

    #[test]
    fn json_omits_absent_values() {
        let json = StyleSet::standard().to_json();
        assert!(
            !json.contains("null"),
            "Typst template relies on absent keys"
        );
    }

    #[test]
    fn css_snapshot() {
        insta::assert_snapshot!(StyleSet::standard().to_css());
    }
}
