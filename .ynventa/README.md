# .ynventa — Ynventa protocol v1

The canonical subsystem every Ynventa repository carries, byte-identical, at `.ynventa/`.
Rust, zero dependencies, no network, no runtime link to any other repository.

    cargo run --manifest-path .ynventa/Cargo.toml -- <command> [--root <repo>]

`status` `verify` `audit` `census` `extinction` `graph` `metrics` `compact` `conformance`
`migrate` `prove` `protocol` `fact` `knowledge` `organism` — identical semantics everywhere;
`help` lists them.

Knowledge (architecture, decisions, milestones, gaps) is typed facts, never Markdown:
`fact add <kind> <subject> <key> <value> --provenance <p>` writes one fact (a newer value of the
same kind/subject/key supersedes the older one, which is kept); `fact list` and
`knowledge view` read; `knowledge extract <doc>...` extracts exactly the named documents (and
licence texts). Extraction is lossless: a document's BLOCK facts (headings with their levels,
paragraphs, lists, code, tables, in order) and a legacy file's `content` rebuild it byte for
byte, proven against its digest; `knowledge view --document <path> [--outline]` rebuilds it.
A decision's status is normalised (`status`, with `status-text` when it says more) and its
relations are facts (`supersedes`, `superseded-by`, `refines`, … = the referenced ids).
`status` and `context` show the current milestones and decisions.

Authoritative state (repository-specific, never hand-written Markdown):

- `declared/{repository,donors,migration,technologies,organism}.rs` — typed Rust declarations:
  nodes (including developmental MATERIAL and, in Norl only, ORGAN nodes), edges, donors,
  capabilities (with `maps_to` and their Norl relevance), proofs, waves, shims, technologies and
  the organism (capabilities, backends; empty outside Norl). `migrate schema` rewrites
  declarations of an earlier grammar; the current reader never accepts them silently.
- `evidence/`, `history/`, `knowledge/` — content-addressed binary records (`*.ynv`).

Everything else in this directory is the canonical subsystem; `protocol.snapshot` pins it.
Generated views go to `target/ynventa/` and are never truth. The census reads committed
(indexed) content: `git add` before judging new files.
