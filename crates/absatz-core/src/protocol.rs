//! Messages between the main thread and the typesetting worker.
//! Sent as JSON strings via `postMessage`; only the PDF comes back as bytes.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ToWorker {
    /// Typeset `source` (if given, else keep the last one) and render the
    /// pages `first..=last` for the preview.
    Update {
        source: Option<String>,
        first: usize,
        last: usize,
    },
    /// Typeset `source` and answer with the PDF bytes (a binary message).
    Pdf { source: String },
    /// Run the typesetting benchmark on a generated sample document.
    Bench { chapters: usize },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum FromWorker {
    Ready,
    /// Answer to `Update`. On `error` the page count and pages are those of
    /// the last successful run (no `svgs`).
    Pages {
        count: usize,
        first: usize,
        svgs: Vec<String>,
        error: Option<String>,
    },
    Bench(BenchReport),
    Error {
        message: String,
    },
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct BenchReport {
    pub pages: usize,
    pub source_kb: usize,
    pub svg_kb: usize,
    pub pdf_kb: usize,
    /// Wasm linear memory, or peak RSS natively.
    pub memory_mb: f64,
    pub entries: Vec<Measurement>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Measurement {
    pub name: String,
    pub ms: f64,
}

impl BenchReport {
    pub fn push(&mut self, name: &str, ms: f64) {
        self.entries.push(Measurement {
            name: name.into(),
            ms,
        });
    }

    /// Human-readable table, used by the CLI and the browser bench page.
    pub fn to_text(&self) -> String {
        let mut s = format!(
            "Seiten: {}   Quelltext: {} KB   SVG/Seite: {} KB   PDF: {} KB   Speicher: {:.0} MB\n",
            self.pages, self.source_kb, self.svg_kb, self.pdf_kb, self.memory_mb
        );
        for m in &self.entries {
            s.push_str(&format!("{:<44} {:>9.1} ms\n", m.name, m.ms));
        }
        s
    }
}

macro_rules! json_codec {
    ($t:ty) => {
        impl $t {
            pub fn to_json(&self) -> String {
                serde_json::to_string(self).expect("serializable")
            }

            pub fn from_json(json: &str) -> Result<Self, String> {
                serde_json::from_str(json).map_err(|e| e.to_string())
            }
        }
    };
}

json_codec!(ToWorker);
json_codec!(FromWorker);
