//! Absatz core: everything that is shared and platform independent.
//!
//! No web or Typst dependencies here – this crate must stay fast to build
//! and fully testable with plain `cargo test`.

pub mod codegen;
pub mod edit;
pub mod history;
pub mod model;
pub mod protocol;
pub mod sample;
pub mod styles;
