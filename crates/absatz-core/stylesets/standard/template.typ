// Absatz – Vorlagensatz „standard“.
//
// Alle Gestaltungswerte kommen aus styles.json. Dieselbe Datei erzeugt das
// CSS der Schreibansicht (absatz-core::styles). Hier keine Zahlen eintragen!

#let s = json("styles.json")
#let pt(x) = x * 1pt
#let mm(x) = x * 1mm

#let text-args(t) = {
  let a = (:)
  if "size" in t { a.insert("size", pt(t.size)) }
  if "color" in t { a.insert("fill", rgb(t.color)) }
  if t.at("italic", default: false) { a.insert("style", "italic") }
  if t.at("bold", default: false) { a.insert("weight", "bold") }
  a
}

#let headings = s.paragraph.values().filter(t => t.role == "heading")

#let apply(doc) = {
  let body = s.paragraph.body
  set page(
    paper: s.page.paper,
    margin: (x: mm(s.page.margin_x_mm), y: mm(s.page.margin_y_mm)),
    header: context {
      let hs = query(heading.where(level: 1).before(here()))
      if hs.len() > 0 { align(right, text(size: 9pt, hs.last().body)) }
    },
    footer: context align(center, text(size: 9pt)[
      #counter(page).display() / #counter(page).final().first()
    ]),
  )
  set text(font: s.fonts.body, lang: s.lang, hyphenate: true, ..text-args(body))
  set par(
    justify: body.at("justify", default: false),
    leading: body.at("leading", default: 0.65) * 1em,
    spacing: pt(body.at("spacing", default: 8)),
  )
  set heading(numbering: "1.1")
  show heading: it => {
    let t = headings.find(h => h.level == it.level)
    if t == none { return it }
    if it.level == 1 { pagebreak(weak: true) }
    set text(..text-args(t))
    it
  }
  show raw: set text(font: s.fonts.mono)
  show table.cell.where(y: 0): strong
  doc
}

// Absatzvorlage anwenden. Die Rolle entscheidet über das Typst-Element.
#let blk(id, body) = {
  let t = s.paragraph.at(id)
  if t.role == "heading" {
    heading(level: t.level, body)
  } else if t.role == "list_item" {
    list.item(body)
  } else if t.role == "quote" {
    quote(block: true, text(..text-args(t), body))
  } else if id == "body" {
    body
  } else {
    block(inset: (left: pt(t.at("indent", default: 0))), text(..text-args(t), body))
  }
}

// Zeichenvorlage anwenden.
#let cs(id, body) = {
  let t = s.character.at(id)
  if t.role == "emphasis" {
    emph(body)
  } else if t.role == "strong" {
    strong(body)
  } else if t.role == "code" {
    text(font: s.fonts.mono, body)
  } else {
    text(..text-args(t), body)
  }
}
