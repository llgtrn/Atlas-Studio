---
id: atlas.decision.0008.extraction-derivation-cache
type: decision
status: accepted
canonical: true
---
# ADR 0008: Content-keyed semantic extraction cache

## Context

R5 requires that a cached derivation be reused only when it is provably unaffected by a change. ADR 0005 and ADR 0006 gave every artifact a content identity and made changes detectable. Two prerequisites were recorded:

- **Parsed bytes must be the digested bytes.** The semantic extractor re-reads each file after inventory.
- **A cache key must identify the extractor.** `RUST_SEMANTIC_EXTRACTOR_VERSION` has been `0.1.0` across dozens of behaviour-changing commits (deferred item `extractor-version-stale`).

Salsa's `cheapest_falsification` asked for exactly this experiment: cache one derivation and measure it against full recompute.

## Decision

1. **`runtime::census::extraction::ExtractionCache`** is a persisted memo of per-artifact `ExtractionBatch`es, written by write-then-rename. Its key is BLAKE3 over length-prefixed fields:
   - the key schema;
   - the canonical JSON of the full `ExtractionInput` minus its source text (this includes repository, revision, artifact, path, frontend, language and dimensions);
   - the BLAKE3 digest of the bytes read;
   - the extractor id and version;
   - `adapter::EXTRACTOR_BUILD_DIGEST`.

   The revision is part of the key because batches embed it. The cache therefore serves the uncommitted edit loop and never relabels an older batch.
2. **`adapter/build.rs`** computes `EXTRACTOR_BUILD_DIGEST` with Atlas's own BLAKE3 over every file in `adapter/src` and `core/src`, plus `Cargo.lock`. Any source or pinned-dependency change invalidates every entry. This closes `extractor-version-stale` for cache keying.
3. **A cache entry is read or written only when the bytes read equal the inventory's `content_digest`.** Otherwise the batch is computed and counted `bypassed`. This closes the parsed-equals-digested prerequisite.
   - A batch from a panicking extractor is never cached.
   - A corrupt entry is a miss and is rewritten.
   - The cache directory is trusted local state, like `target/`.
4. **Wiring:**
   - `runtime::systemize_with(root, previous, cache)`.
   - `atlas-systemizer systemize --cache DIR`.
   - `SystemizeReport.extraction_cache` reports hits, misses, bypassed and write failures (report schema v14).

## Measured result: the premise was falsified at this repository's scale

On this repository (debug build), cold and warm cached runs produce reports byte-identical to an uncached run (62 misses, then 62 hits). The warm run is **not** faster (about 177 s against about 190 s).

Stage timing showed why. Snapshot, inventory, extraction, census and normalization together take about 6 s. `summarize_system_graph_with_dependencies` takes about **166 s**, roughly 95% of the run.

Caching extraction is correct but addresses about 2% of edit-to-recensus cost here. It may matter on larger corpora, where extraction is proportionally larger. The dominant R5 cost is engineering-graph construction, and that is the next target.

## Consequences

- The memo mechanism is absorbed from salsa: reuse a derivation only when its inputs are verified unchanged. Salsa's dependency tracking, early cutoff, durability and cycle handling are not absorbed.
- The salsa and differential-dataflow blocking questions are re-aimed at the measured cost centre.
