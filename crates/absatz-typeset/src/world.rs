use absatz_core::styles::{StyleSet, StyleSetSource};
use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime, Duration};
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World};

/// Everything lives in memory; there is no file system in the browser.
pub struct AbsatzWorld {
    library: LazyHash<Library>,
    book: LazyHash<FontBook>,
    fonts: Vec<Font>,
    main: Source,
    template: Source,
    styles: Bytes,
    styles_id: FileId,
}

fn file_id(path: &str) -> FileId {
    RootedPath::new(
        VirtualRoot::Project,
        VirtualPath::new(path).expect("valid path"),
    )
    .intern()
}

impl AbsatzWorld {
    pub fn new(style_set: &StyleSetSource) -> Self {
        // TODO(M3): load only the fonts a style set needs, fetched lazily.
        let fonts: Vec<Font> = typst_assets::fonts()
            .flat_map(|data| Font::iter(Bytes::new(data)))
            .collect();
        let styles = StyleSet::parse(style_set.tokens_json)
            .expect("valid tokens")
            .to_json();
        Self {
            library: LazyHash::new(Library::default()),
            book: LazyHash::new(FontBook::from_fonts(&fonts)),
            fonts,
            main: Source::new(file_id("main.typ"), String::new()),
            template: Source::new(file_id("template.typ"), style_set.template_typ.into()),
            styles: Bytes::from_string(styles),
            styles_id: file_id("styles.json"),
        }
    }

    /// Returns whether the text changed.
    pub fn set_main(&mut self, text: &str) -> bool {
        let changed = self.main.text() != text;
        if changed {
            self.main.replace(text);
        }
        changed
    }

    /// Returns whether the tokens changed.
    pub fn set_styles(&mut self, json: String) -> bool {
        let changed = self.styles.as_slice() != json.as_bytes();
        self.styles = Bytes::from_string(json);
        changed
    }
}

impl World for AbsatzWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &self.book
    }

    fn main(&self) -> FileId {
        self.main.id()
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.main.id() {
            Ok(self.main.clone())
        } else if id == self.template.id() {
            Ok(self.template.clone())
        } else {
            Err(FileError::NotFound(Default::default()))
        }
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        if id == self.styles_id {
            return Ok(self.styles.clone());
        }
        self.source(id)
            .map(|s| Bytes::from_string(s.text().to_string()))
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.get(index).cloned()
    }

    fn today(&self, _offset: Option<Duration>) -> Option<Datetime> {
        // Deterministic output; the document date comes from metadata later.
        None
    }
}
