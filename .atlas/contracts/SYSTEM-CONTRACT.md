---
id: atlas.contract.system
type: contract
status: active
canonical: true
---
# Atlas Studio System Contract

Hard invariants: ATLAS_ROOT_IS_CANONICAL_KNOWLEDGE; BACKEND_IS_RUST; FRONTEND_IS_TYPESCRIPT; UNTRUSTED_INPUT_DEFAULT_DENY; INGESTION_IS_NOT_EXECUTION; DONOR_IS_EVIDENCE_NOT_AUTHORITY; PROVENANCE_REQUIRED; EPISTEMIC_STATUS_REQUIRED; REBUILDABLE_PROJECTION_IS_NOT_TRUTH; ATLAS_PRECEDES_ATLASX; ATLASX_PRECEDES_MATERIALIZATION; AI_OUTPUT_IS_CANDIDATE_UNTIL_VERIFIED; NATIVE_TECHNOLOGY_CANNOT_REQUIRE_DONOR_RUNTIME.

ATLAS binary readers validate format, version, bounds and integrity before trusting file-provided counts or offsets. Corpus publication is transactional. Secrets and unsafe donor content are not silently persisted. The atlas.systemizer.cli.v1 surface is compatibility only.
