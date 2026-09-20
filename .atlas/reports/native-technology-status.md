---
id: native-technology-status
type: generated-evidence
status: active
canonical: true
---
# Native Technology Status

## Donor Corpus

The canonical machine-readable donor state is `.atlas/references/donor-corpus.toml`.

Current corpus state:

- Donors cloned: 24
- Clone root: `.atlas/temporary/donors/`
- Provenance root: `.atlas/provenance/donors/`
- License root: `.atlas/licenses/`
- Census root: `.atlas/census/`

## Native Implementation

Atlas has an active Rust workspace shaped as:

- `core/`: product-neutral facts, nodes, edges, bindings, evidence, manifest policy, graph bootstrap.
- `adapter/`: filesystem, repository manifest, docs and source observation adapters.
- `runtime/`: repository compilation and CLI orchestration.

## Status

Donors are cloned and pinned, but not absorbed. Their census files are skeletons until source inspection records major modules, algorithms, storage model, execution model, query model, tests, benchmarks, assumptions, accepted ideas, rejected ideas and native replacement gaps.

Runtime dependency status remains `REFERENCE_ONLY` for every donor. No donor is marked extinct.
