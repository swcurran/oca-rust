# oca-rs

Rust workspace for implementation of the core OCA (Overlays Capture Architecture) specification.

## Scope and audience

This repository targets implementers of the OCA core specification. It provides
low-level crates that model, parse, and validate OCA artifacts. Most users should
prefer the higher-level `oca-sdk-rs`, which wraps these crates into a stable
developer-facing API.

## Crates in this workspace

- `oca-ast`: AST types and validation helpers for OCA bundles.
- `oca-file`: Parser and generator for the OCAfile DSL.
- `overlay-file`: Parser and registry for Overlayfile definitions.
- `oca-bundle`: Builder and validator for OCA bundles.

## Relationship to oca-sdk-rs

`oca-sdk-rs` is the recommended entry point for application developers. It
exposes the functionality of this workspace with a more stable API surface,
while `oca-rs` focuses on core-spec correctness and composition.

## License

EUPL 1.2. See `LICENSE`.
