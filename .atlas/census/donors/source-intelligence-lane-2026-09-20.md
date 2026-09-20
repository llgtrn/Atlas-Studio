---
id: atlas.census.donors.source-intelligence-lane-2026-09-20
type: donor-census
status: active
canonical: true
---
# Source Intelligence Lane Deep Census

## Scope

Lane members:

- Tree-sitter: `.atlas/temporary/donors/tree-sitter`
- rust-analyzer: `.atlas/temporary/donors/rust-analyzer`
- SCIP: `.atlas/temporary/donors/scip`
- Kythe: `.atlas/temporary/donors/kythe`
- Glean: `.atlas/temporary/donors/glean`
- Joern: `.atlas/temporary/donors/joern`

Target Atlas owners:

- `adapter/source`: static source scanning, syntax extraction, source-index exchange readers.
- `core/identity`: stable source, symbol and occurrence identity.
- `core/model`: typed source facts, symbols, relations, provenance and epistemic class.
- `runtime/ingest`: admitted repository ingestion into source facts.
- `runtime/link`: symbol, definition, reference and cross-repository linking.

All inspected donor source is untrusted, read-only evidence. No donor code was executed.

## Donor: Tree-sitter

Pin:

- Commit: `5b951eff4f8b1431e933ed0fe45e48fcd4036a38`
- Clone: `.atlas/temporary/donors/tree-sitter`

Observed topology:

- `lib/src`: C runtime parser, tree, node, lexer, query and changed-range implementation.
- `lib/binding_rust/lib.rs`: Rust API for `Parser`, `Tree`, `Node`, `TreeCursor`, `Query`, `QueryCursor`, `QueryMatch` and `QueryCapture`.
- `crates/generate/src`: grammar preparation, parser table generation, deduplication and node-type emission.
- `crates/highlight/src` and `crates/tags/src`: query-based projections over syntax trees.
- `crates/loader/src`: language loading and parser/runtime mechanics.
- `crates/cli/src`: CLI wrapper, parse/query/highlight/test helpers.
- `test`: grammar and runtime fixtures.

Central mechanisms:

- Incremental concrete syntax tree: README describes Tree-sitter as a parser generator and incremental parsing library; `lib/src/tree.c` and `lib/src/get_changed_ranges.c` implement tree edit and changed-range tracking.
- Parser boundary: `lib/binding_rust/lib.rs` exposes `Parser` and `Tree`; `Parser::set_included_ranges` maps directly to `ts_parser_set_included_ranges`.
- Query model: `lib/binding_rust/lib.rs` defines `Query`, `QueryCursor`, `QueryMatch`, `QueryCapture`, query predicates and query properties.
- Language metadata: `Language::name`, `Language::abi_version` and `Language::metadata` expose grammar identity and ABI compatibility.
- Projection examples: `crates/highlight/src/highlight.rs` and `crates/tags/src/tags.rs` apply queries to produce semantic-ish views such as highlights and tags.

KEEP:

- Treat parser output as observed syntax with byte ranges and grammar identity.
- Preserve changed-range and included-range concepts for bounded reparse.
- Preserve query-as-projection, not query-as-canonical semantics.

ADAPT:

- Atlas `adapter/source` should define a `SyntaxExtractor` boundary inspired by `Parser`, `Tree`, `Node`, `Query`, and `QueryCapture`, but the core model should not depend on Tree-sitter types.
- Atlas should store parser/language ABI metadata as provenance on observed syntax facts.

REWRITE:

- Atlas canonical source facts must be Rust-owned typed records, not Tree-sitter C structs or query captures.
- Parser execution must be security-admitted and resource-limited before it can feed `corpus.atlas`.

REJECT:

- Do not make Tree-sitter queries the Atlas semantic graph language.
- Do not execute generated grammars or loaders from donor source during ingestion without an explicit sandbox/admission path.

## Donor: rust-analyzer

Pin:

- Commit: `aaddfb73fd95f2c0bf001b474dca91ae28bcce3a`
- Clone: `.atlas/temporary/donors/rust-analyzer`

Observed topology:

- `crates/vfs`: virtual file system, interned `FileId`, file changes and source-root partitioning.
- `crates/base-db`: salsa-backed `SourceDatabase`, file text/source-root inputs and crate graph data.
- `crates/hir`, `crates/hir-def`, `crates/hir-ty`: HIR, name resolution and type inference surfaces.
- `crates/ide`, `crates/ide-db`, `crates/ide-diagnostics`: semantic services over `RootDatabase`.
- `crates/project-model`, `crates/load-cargo`: Cargo/rust-project ingestion into crate graph.
- `crates/syntax`, `crates/parser`, `crates/syntax-bridge`: Rust syntax bridge.

Central mechanisms:

- File identity and change stream: `crates/vfs/src/lib.rs` defines `FileId(u32)`, `Vfs`, `ChangedFile`, `Change`, `FileState`, path interning and `take_changes`.
- Source database: `crates/base-db/src/lib.rs` defines `SourceDatabase` with file text, source roots, file-source-root mapping, crate maps, revision and line-column lookup.
- Incremental recomputation: the VFS docs state that changes are pushed to salsa and trigger incremental recomputation.
- Workspace/crate graph: `CrateGraphBuilder::set_in_db` writes crate graph state into the database.
- Semantic API: `crates/hir/src/semantics.rs` defines `Semantics`, while IDE diagnostics use `Semantics<'_, RootDatabase>` for source-to-HIR reasoning.

KEEP:

- Stable file IDs backed by an interner, plus explicit file-state transitions.
- Distinction between filesystem loader/watch mechanics and the semantic database.
- Crate/workspace graph as observed source context, not just a manifest string.
- Query/input durability categories for stable library roots versus frequently changing local roots.

ADAPT:

- Atlas `core/identity` should expose `SourceFileId` or reuse typed IDs for admitted file identity instead of raw path strings.
- Atlas `adapter/source/rust` should derive Rust package/source-root/crate relationships into typed observed facts.
- Atlas `runtime/ingest` should consume `ChangedFile`-like deltas for future incremental corpus builds.

REWRITE:

- Do not import rust-analyzer as an Atlas backend. Atlas needs a smaller source intelligence boundary for multi-language, multi-repository ingestion.
- Rust-specific HIR should become one source adapter's output, not a universal Atlas semantic model.

REJECT:

- No proc-macro execution during normal donor/source ingestion.
- No assumption that a `FileId(u32)` position is durable across corpus rebuilds.

## Donor: SCIP

Pin:

- Commit: `4f50fbbd0ed945405cd8a4196af6477ea2a9f8b5`
- Clone: `.atlas/temporary/donors/scip`

Observed topology:

- `scip.proto`: canonical protobuf schema.
- `bindings/go/scip`: parser/formatter/canonicalizer/symbol table helpers.
- `bindings/rust/src/symbol.rs`: Rust symbol helper surface.
- `bindings/typescript`: TypeScript generated binding.
- `cmd/scip`: CLI for print, lint, convert, stats and snapshot.
- `docs`: format, design and CLI docs.

Central mechanisms:

- Streaming index model: `scip.proto` describes `Index` as a potentially large payload where `metadata` must appear first and documents may stream field-by-field.
- Workspace/document model: `Metadata.project_root`, `Document.relative_path`, language, occurrences, symbols and position encoding.
- Symbol grammar: `Symbol` defines scheme, package and descriptor syntax, with local symbols explicitly document-scoped.
- Occurrence roles: `SymbolRole` bitset distinguishes definition, import, write, read, generated, test and forward definition.
- Cross-symbol relationships: `Relationship` models reference, implementation, type definition and definition relationships.

KEEP:

- Use SCIP as the exchange-shape reference for symbol occurrences and definition/reference roles.
- Preserve explicit position encodings and canonical relative path constraints.
- Preserve local-vs-global symbol identity distinction.

ADAPT:

- Atlas `adapter/exchange/source_index` should read SCIP-like inputs into Atlas typed source facts.
- Atlas should model `Occurrence`, `SymbolInformation`, `Relationship` and `Package` as typed concepts, with provenance and trust class.

REWRITE:

- Atlas's canonical source graph must not be SCIP protobuf. SCIP is an import/export adapter and donor principle.
- Atlas symbol IDs should be stable typed IDs with derivation details, not only SCIP string grammar.

REJECT:

- Do not store optional raw document text from source indexes in `corpus.atlas` until secret and sensitivity classification passes.
- Do not trust `project_root` or `relative_path` without Atlas path admission.

## Donor: Kythe

Pin:

- Commit: `69141f022689a611e8a4a1d9b08a3783a2e8a9ed`
- Clone: `.atlas/temporary/donors/kythe`

Observed topology:

- `kythe/proto/storage.proto`: `VName`, `Entry`, read/write/scan/shard requests and VName rewrite rules.
- `kythe/proto/graph.proto`: fast node and edge lookups by ticket.
- `kythe/proto/xref.proto`: decorations, cross-references and documentation service.
- `kythe/proto/analysis.proto`: compilation-unit/source-file analysis model.
- `kythe/go/indexer`, `kythe/go/serving/xrefs`, `kythe/go/util/vnameutil`: Go indexer and serving utilities.
- `kythe/cxx/verifier`: verifier and Souffle-backed assertion support.
- `kythe/data/vnames*.json`: VName rewrite examples.

Central mechanisms:

- Vector name: `VName` separates signature, corpus, root, path and language; revision/time are explicitly facts, not name axes.
- Graph storage unit: `Entry` associates a fact with a source node or edge using source VName, edge kind, target VName, fact name and fact value.
- Serving model: `GraphService` provides fast node/edge lookup; `XRefService` provides decorations, references, definitions, callers and documentation.
- Query scalability: `EdgesRequest` paginates ordered edge sets and lets callers request target facts by filters.
- Corpus identity: corpus/root/path gives Atlas a concrete precedent for repository partitions and source identity.

KEEP:

- Distinguish semantic identity from revision/time evidence.
- Represent facts and edges as first-class, provenance-bearing records.
- Preserve batch lookup, pagination and filter-by-fact concepts for ATLASX queries.

ADAPT:

- Atlas `core/identity` should use Kythe's VName separation as input to `SourceIdentity`/`SymbolIdentity` design.
- Atlas `runtime/link` should map observed source facts into fast lookup projections that resemble graph/xref serving, but with Atlas types.

REWRITE:

- Atlas should not adopt Kythe tickets as the canonical user-facing ID format.
- Atlas needs security/trust and epistemic class per fact; Kythe storage is not sufficient on its own.

REJECT:

- Do not require Bazel/Kythe extraction as the Atlas ingestion substrate.
- Do not treat source-control revision as part of a symbol's stable semantic identity.

## Donor: Glean

Pin:

- Commit: `2a48dea4cddb316d3b8bd65b54965cb9a7855c52`
- Clone: `.atlas/temporary/donors/glean`

Observed topology:

- `glean/angle`: Angle schema, parser, type resolver, schema evolution and query language.
- `glean/db/Glean/Query`: query typecheck, transform, optimize, flatten, codegen and incremental logic.
- `glean/backend-local`, `glean/backend-api`, `glean/write`: local backend and write queues.
- `glean/storage`, `glean/rocksdb`, `glean/lmdb-clib`: storage backends.
- `glean/bench`: query, compile, fact-set and server benchmarks.
- `glean/example`: example schemas and facts.

Central mechanisms:

- Typed fact schema: `glean/angle/Glean/Angle/Types.hs` defines schema references, source spans, source queries, source statements and source patterns.
- Typechecked query IR: `glean/db/Glean/Query/Typecheck/Types.hs` lowers parsed patterns into typed query terms such as `TcFactGen`, `TcQueryGen`, `TcNegation`, field selection and typed primitive calls.
- Storage and write model: write queues and local backend separate ingestion/writes from query serving.
- Performance discipline: bench suite explicitly measures query/compile/fact-set/server behavior.

KEEP:

- Typed query IR after parsing and before execution.
- Explicit schema evolution and versioned predicate/type references.
- Benchmark-first treatment of fact-store and query performance.

ADAPT:

- Atlas `core/model` should move away from generic `kind: String`/attribute maps toward typed facts, typed values and schema-versioned predicates.
- Atlas `runtime/query` should eventually compile query syntax/projections into typed query terms before execution.

REWRITE:

- Atlas will not embed Glean's Haskell service or storage stack.
- Atlas `corpus.atlas` and ATLASX are binary/semantic artifacts, not Glean databases.

REJECT:

- Do not copy Angle as ADL.
- Do not make fact query results authoritative unless their epistemic origin and provenance are preserved.

## Donor: Joern

Pin:

- Commit: `b381922638ae436fcb6862a90241c8f7ce508894`
- Clone: `.atlas/temporary/donors/joern`

Observed topology:

- `console/src/main/scala/io/joern/console/cpgcreation`: language CPG generator wrappers.
- `semanticcpg`: semantic code-property-graph traversal layer.
- `dataflowengineoss`: data-flow engine.
- `querydb`: query collections.
- `tests`: script and language frontend smoke tests.
- `changelog/4.0.0-flatgraph.md`: flatgraph migration and performance rationale.

Central mechanisms:

- Code property graph as unified source projection over AST, CFG, call graph, data flow and semantic layers.
- Language-specific frontends generate CPG artifacts. `CpgGenerator.scala` invokes external commands to produce `cpg.bin.zip`.
- Flatgraph migration: `changelog/4.0.0-flatgraph.md` records reduced memory, faster traversal, columnar layout and smaller file size compared with overflowdb.
- Security boundary: CPG generation is process/external-command oriented, which is evidence that Atlas must not execute donor/tooling code during ingestion.

KEEP:

- Unified code graph projection as a useful semantic view over source facts.
- Columnar graph layout principles for future ATLAS/ATLASX performance work.
- Query packs as reusable analysis patterns, but only after translation to Atlas-native rule/query semantics.

ADAPT:

- Atlas `runtime/link` and `runtime/query` should provide CPG-like projections from the same typed source facts, not a separate graph universe.
- Atlas benchmarks should measure graph traversal and storage size as Joern did for flatgraph.

REWRITE:

- Do not embed Joern's Scala runtime or CPG generators.
- Atlas materialization must use Rust engine APIs and admitted source facts, not shelling out to donor frontends.

REJECT:

- No automatic external command execution during source ingestion.
- Do not treat vulnerability/data-flow query packs as verified Atlas rules without evidence.

## Lane Synthesis

Required Atlas-native source-intelligence contract:

1. `SourceIdentity`: repository partition, source path, content fingerprint, integrity digest, language and revision provenance.
2. `SourceUnit`: admitted source file metadata with sensitivity/trust classification and no raw secret persistence by default.
3. `SyntaxObservation`: parser/language identity, tree node ranges, optional changed ranges and parser diagnostics.
4. `SymbolIdentity`: stable symbol ID derived from language-aware name components, package/module context and provenance.
5. `SymbolOccurrence`: file, range, symbol, role bitset and position encoding.
6. `SourceRelationship`: definition/reference/implementation/type/call/import relationships with source spans.
7. `SourceFact`: typed fact with declared/observed/inferred/candidate/verified epistemic class.
8. `SourceIndexImport`: adapter boundary for SCIP/Kythe-like exchange formats.
9. `CodeGraphProjection`: derived AST/CFG/call/data-flow/query projection over typed source facts.
10. `SourceIngestDelta`: change-set input for incremental corpus rebuilds.

Immediate implementation target:

- Build Atlas-native source ingestion for repository files first: path admission, language classification, source identity, content fingerprint/digest, and observed `SourceFile` graph nodes.
- Then add symbol occurrence exchange import using SCIP-like records.
- Then add Rust-specific semantic enrichment using a rust-analyzer-inspired adapter boundary.
- Then add CPG-like projections after ATLAS/ATLASX storage and query foundations exist.

Open gaps:

- Parser sandbox and resource limits must exist before executing parsers over untrusted donors.
- Raw source persistence into `corpus.atlas` requires secret/sensitivity handling.
- Typed source identity exists only partially today; current graph still carries bootstrap string IDs and path strings.
- No ATLAS binary source corpus exists yet.

Decision:

- All six source-intelligence donors are `DEEP_CENSUSED` for the initial source-intelligence lane.
- Runtime status remains `REFERENCE_ONLY`; principles are approved for Atlas-native reimplementation.
- Target mapping is `adapter/source`, `adapter/exchange`, `core/identity`, `core/model`, `runtime/ingest`, `runtime/link`, and future `runtime/query`.
