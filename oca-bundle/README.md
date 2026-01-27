# [![Rust Build Status]][Rust actions] [![Cargo version]][crates.io] [![WASM Build Status]][WASM actions] [![NPM version]][npmjs.com]

[Rust Build Status]: https://github.com/THCLab/oca-rs/actions/workflows/rust.yml/badge.svg?branch=main
[Rust actions]: https://github.com/THCLab/oca-rs/actions/workflows/rust.yml
[Cargo version]: https://img.shields.io/crates/v/oca-rs
[crates.io]: https://crates.io/crates/oca-rs
[WASM Build Status]: https://github.com/THCLab/oca-rs/actions/workflows/wasm.yml/badge.svg?branch=main
[WASM actions]: https://github.com/THCLab/oca-rs/actions/workflows/wasm.yml
[NPM version]: https://img.shields.io/npm/v/oca.js
[npmjs.com]: https://www.npmjs.com/package/oca.js
[Crates.io actions]: https://github.com/THCLab/oca-rs/actions/workflows/create.yml
[npmjs.com actions]: https://github.com/THCLab/oca-rs/actions/workflows/npm-publish.yml

# oca-bundle

Builder and validator for OCA bundles.

## What this crate provides

- Construction of OCA bundles from structured inputs.
- Validation of overlays and bundle content against definitions.
- Core bundle state and encoding helpers used by higher-level crates.

## Intended use

This crate targets implementers of the OCA core specification. Most application
developers should use `oca-sdk-rs` instead.

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
oca-bundle = "0.7.1"
```

## License

EUPL 1.2. See `LICENSE`.
