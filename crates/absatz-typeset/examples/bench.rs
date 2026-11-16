//! Native typesetting benchmark.
//!
//!     cargo run --release -p absatz-typeset --example bench -- [chapters]
//!
//! About ten pages per chapter; default 10 (≈100 pages).

use std::time::Instant;

fn main() {
    let chapters = std::env::args()
        .nth(1)
        .and_then(|a| a.parse().ok())
        .unwrap_or(10);
    let start = Instant::now();
    let now = || start.elapsed().as_secs_f64() * 1000.0;
    let report = absatz_typeset::bench::run(chapters, &now);
    print!("{}", report.to_text());
}
