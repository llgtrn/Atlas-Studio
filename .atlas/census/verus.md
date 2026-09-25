---
id: donor-census-verus
type: reference
status: active
canonical: true
---
# Donor Census: Verus

## Source

- Remote: https://github.com/verus-lang/verus.git
- Commit: 5f0930a0b820425aa857269070362ddcf7c29672
- Branch: main
- Retrieved: 2026-09-20T12:08:55.4960343Z

## Census State

Status: SKELETON. Source is cloned and pinned; implementation inspection still needs to classify modules, algorithms, storage, execution, query, incremental behavior, tests, benchmarks, assumptions, accepted ideas, rejected ideas, and Atlas-native replacement gaps.

## Native Replacement

core constraint/proof model


## G105 — bounded census; terminal EXTERNAL_BOUNDARY (+ REFERENCE_ONLY); source extinct

Verus lowers rustc HIR through VIR, SST and AIR to SMT queries that Z3 discharges. Its main design points are spec/proof/exec modes, triggers, context pruning and termination checks, and it reports results through `--output-json`.

Its P0 role has the same missing consumer as Kani (G104): no VerificationEvidence code exists, and the contract has no proof class. Atlas's census obligations are extraction-closure obligations, a different notion. No verification result is claimed. The VIR design stays REFERENCE_ONLY. The checkout was physically deleted. Evidence: `../evidence/campaign/41-verus.json`.
