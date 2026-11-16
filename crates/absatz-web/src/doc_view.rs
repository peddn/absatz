//! Read-only rendering of the document model as semantic HTML.
//! Becomes the writing view in M1 (EditContext on top of this DOM).

use absatz_core::model::{Block, BlockContent, Document, Inline};
use absatz_core::styles::{Role, StyleSet};
use leptos::prelude::*;

pub fn blocks(doc: &Document, styles: &StyleSet) -> impl IntoView + use<> {
    doc.blocks.iter().map(|b| block(b, styles)).collect_view()
}

fn block(block: &Block, styles: &StyleSet) -> AnyView {
    let class = format!("p-{}", block.style);
    let style = styles.paragraph.get(&block.style);
    match &block.content {
        BlockContent::Text(inlines) => {
            let body = inline_views(inlines);
            match style.map(|s| (s.role, s.level.unwrap_or(1))) {
                Some((Role::Heading, 1)) => view! { <h1 class=class>{body}</h1> }.into_any(),
                Some((Role::Heading, 2)) => view! { <h2 class=class>{body}</h2> }.into_any(),
                Some((Role::Heading, _)) => view! { <h3 class=class>{body}</h3> }.into_any(),
                Some((Role::Quote, _)) => {
                    view! { <blockquote class=class>{body}</blockquote> }.into_any()
                }
                _ => view! { <p class=class>{body}</p> }.into_any(),
            }
        }
        BlockContent::Math(src) => view! { <div class="math">{src.clone()}</div> }.into_any(),
        BlockContent::Table { rows, caption } => {
            let rows = rows
                .iter()
                .map(|row| {
                    let cells = row.iter().map(|c| view! { <td>{inline_views(c)}</td> });
                    view! { <tr>{cells.collect_view()}</tr> }
                })
                .collect_view();
            view! {
                <figure>
                    <table>{rows}</table>
                    <figcaption>{inline_views(caption)}</figcaption>
                </figure>
            }
            .into_any()
        }
        BlockContent::Outline => {
            view! { <nav class="toc">"Inhaltsverzeichnis (erscheint in der Seitenansicht)"</nav> }
                .into_any()
        }
    }
}

fn inline_views(inlines: &[Inline]) -> AnyView {
    inlines
        .iter()
        .map(|inline| match inline {
            Inline::Text { text, style: None } => text.clone().into_any(),
            Inline::Text {
                text,
                style: Some(s),
            } => view! { <span class=format!("c-{s}")>{text.clone()}</span> }.into_any(),
            Inline::Footnote(content) => {
                view! { <span class="footnote">{inline_views(content)}</span> }.into_any()
            }
        })
        .collect_view()
        .into_any()
}
