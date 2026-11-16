# Absatz – all everyday commands. `just` lists them.

web := "crates/absatz-web"

# List recipes
default:
    @just --list

# Install tools (trunk, just, Playwright deps). Idempotent.
setup:
    scripts/setup.sh

# Format code
fmt:
    cargo fmt --all

# Formatting, clippy (native and wasm), no warnings allowed
lint:
    cargo fmt --all --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo clippy -p absatz-web -p absatz-typeset --target wasm32-unknown-unknown -- -D warnings

# Native unit tests (the web crate has none; it is covered by e2e)
test:
    cargo test --workspace --exclude absatz-web

# Accept changed insta snapshots after reviewing them
snapshots:
    INSTA_UPDATE=always cargo test --workspace --exclude absatz-web

# Dev server with reload: http://localhost:8080/absatz/
dev:
    cd {{web}} && trunk serve

# Release build into crates/absatz-web/dist
build:
    cd {{web}} && trunk build --release
    @just sizes

# Wasm sizes (raw and gzip) – keep an eye on them
sizes:
    #!/usr/bin/env bash
    for f in {{web}}/dist/*.wasm; do
      printf "%-44s %7d KB   gzip %6d KB\n" "$(basename "$f")" \
        $(( $(stat -c%s "$f") / 1024 )) $(( $(gzip -9c "$f" | wc -c) / 1024 ))
    done

# Browser tests against the release build (Chromium)
e2e: build
    cd e2e && npm test

# Native typesetting benchmark (≈10 pages per chapter)
bench chapters="10":
    cargo run --release -p absatz-typeset --example bench -- {{chapters}}

# Same benchmark in Chromium (wasm in a Web Worker)
bench-web chapters="10": build
    cd e2e && BENCH_CHAPTERS={{chapters}} npm run bench

# Everything CI checks
ci: lint test e2e
