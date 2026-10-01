# .ynventa — Ynventa protocol v1

The canonical subsystem every Ynventa repository carries, byte-identical, at `.ynventa/`.
Rust, zero dependencies, no network, no runtime link to any other repository.

    cargo run --manifest-path .ynventa/Cargo.toml -- <command> [--root <repo>]

`status` `verify` `audit` `census` `extinction` `graph` `metrics` `compact` `conformance`
`migrate` `prove` `protocol` `fact` `knowledge` — identical semantics everywhere; `help` lists
them.

Knowledge (architecture, decisions, milestones, gaps) is typed facts, never Markdown:
`fact add <kind> <subject> <key> <value> --provenance <p>` writes one fact (a newer value of the
same kind/subject/key supersedes the older one, which is kept); `fact list` and
`knowledge view` read; `knowledge extract <doc>...` extracts exactly the named documents.
`status` and `context` show the current milestones and decisions.

Authoritative state (repository-specific, never hand-written Markdown):

- `declared/{repository,donors,migration}.rs` — typed Rust declarations: nodes, edges, donors,
  capabilities, proofs, waves, shims.
- `evidence/`, `history/`, `knowledge/` — content-addressed binary records (`*.ynv`).

Everything else in this directory is the canonical subsystem; `protocol.snapshot` pins it.
Generated views go to `target/ynventa/` and are never truth. The census reads committed
(indexed) content: `git add` before judging new files.
