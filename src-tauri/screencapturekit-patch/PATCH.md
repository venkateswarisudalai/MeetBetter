# Vendored screencapturekit 1.5.0

This is [`screencapturekit` 1.5.0](https://crates.io/crates/screencapturekit/1.5.0) from crates.io
(MIT or Apache-2.0, see the license files here), used through `[patch.crates-io]` in
`src-tauri/Cargo.toml`.

**One change, in `build.rs`:** the Swift bridge is built with `--triple` matching Cargo's `TARGET`
(`x86_64-apple-macosx12.3` or `arm64-apple-macosx12.3`), so the universal DMG can cross-compile both
architectures. Upstream builds the Swift bridge for the host architecture only.

To upgrade: download the new crate, re-apply that `build.rs` change, and bump the version in
`Cargo.toml`.
