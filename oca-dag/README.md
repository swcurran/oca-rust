# OCA DAG

Rust implementation of OCA DAG

**`oca-dag-rs`** — model your Overlays Capture Architecture as a versioned,
traceable graph.

It lib for managing **Directed Acyclic Graphs (DAGs)** that represent the
structure and evolution of **Overlays Capture Architecture (OCA)** bundles. It
serves as a foundational layer for versioning, history tracking, and dependency
management in systems built on the OCA model.

## Features

- **DAG-based Model Management**
  Represents OCA data (capture bases, overlays, bundles etc.) as a Directed
Acyclic Graph, enabling clear and scalable relationships between versions and
components.

- **Powerful Versioning System**
  Every change to an OCA bundle—whether it's a new overlay, update, or
branching—forms a new node in the DAG, supporting sophisticated version
control.

- **Full History Tracking**
  Enables tracking of the complete change history of an OCA bundle, including
merges and branches/forks, akin to Git-like semantics.

- **Dependency Resolution**
  Naturally handles dependencies between capture bases, overlays, and other OCA
elements via parent-child relationships in the graph.

- **Current State Representation**
  Maintains a clear representation of the current state of a bundle

- **Graph Construction**
 Allows for the construction and updating
of the DAG from a stream of input commands, effectively modeling the evolution
of OCA bundles over time.

## Use Cases

- Versioning of OCA bundles
- Collaborative OCA editing with conflict resolution
- Auditing and provenance of overlays and capture bases
- Programmatic traversal and analysis of OCA dependencies

## Related Projects

- [`oca-rs`](https://github.com/THCLab/oca-rs) — Core implementation of the Overlays Capture Architecture
- [`oca-ast`](https://github.com/THCLab/oca-rs/tree/main/semantics/oca-ast) — AST structures for OCA semantics

---


