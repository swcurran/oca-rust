# overlay-file

Parser and registry for Overlayfile definitions.

Overlayfile Specification: [https://oca.colossi.network/specification/overlayfile.html](https://oca.colossi.network/specification/overlayfile.html)

## What this crate provides

- Parsing the Overlayfile DSL into overlay definitions.
- Registry utilities for loading and resolving overlay definitions.
- Support for the core overlay set used by other crates in this workspace.

## Intended use

This crate is a low-level component for OCA core specification implementers.
Most application developers should use `oca-sdk-rs` instead.

## License

EUPL 1.2. See `LICENSE`.
