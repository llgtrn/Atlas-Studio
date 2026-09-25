---
id: donor-census-kani
type: reference
status: active
canonical: true
---
# Donor Census: Kani

## Source

- Remote: https://github.com/model-checking/kani.git
- Commit: 80f3cc98af976a54b9dd0614e877e4d2f1f5a58d
- Branch: main
- Retrieved: 2026-09-20T12:08:55.4960343Z

## Census State

Status: SKELETON. Source is cloned and pinned; implementation inspection still needs to classify modules, algorithms, storage, execution, query, incremental behavior, tests, benchmarks, assumptions, accepted ideas, rejected ideas, and Atlas-native replacement gaps.

## Native Replacement

runtime verification/proof engine


## G104 — bounded census; terminal EXTERNAL_BOUNDARY (+ REFERENCE_ONLY); source extinct

Surfaces inspected:
- harnesses over `kani::any`/`kani::assume`;
- function contracts (`requires`/`ensures`, `proof_for_contract`, verified stubs);
- autoharness;
- the CBMC output parser;
- SARIF 2.1.0 output of harness results;
- concrete playback of counterexamples as unit tests.

Kani runs on CBMC, and its results already leave it as SARIF. The P0 role, proofs as constraint evidence, has no consumer yet. VerificationEvidence exists only as contract text, and the contract's class vocabulary has no proof/model-checking class, which is a recorded gap. Neither Kani nor CBMC is installed, so no proof result is claimed. The harness and contract designs stay REFERENCE_ONLY. The checkout was physically deleted. Evidence: `../evidence/campaign/40-kani.json`.
