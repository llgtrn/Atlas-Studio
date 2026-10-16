# Schema compatibility producer currency

The retired `atlas::` selector stopped matching when Core's census container moved at
`6e2ee1d3c5de427a2aed1fb9be7d1c02bbc93782`. Cargo exit zero alone could then produce
SATISFIED without running a schema/history test. This repair addresses
`OBL-SELF-COMPATIBILITY-SCHEMAS` (G127 / ADR 0048), supporting the source-evidence obligations
of native contract N11 and journey J8; it does not qualify the full native product.

The producer now selects exactly these three library tests, offline, with explicit pretty,
non-colored libtest output:

- `codara::artifact::census_container::tests::the_current_schema_is_the_latest_recorded_generation`
- `codara::artifact::census_container::tests::every_recorded_schema_conforms_to_the_current_one`
- `codara::artifact::census_container::tests::a_container_an_older_conforming_writer_produced_is_read_with_its_history`

SATISFIED requires exit zero, the exact three distinct named passing outcomes and one complete
successful summary with no failures, ignored tests or measured benchmarks. Zero tests, wrong
subsets, duplicate/conflicting outcomes, incomplete output and unknown formats are UNRESOLVED;
a failed or signalled process is VIOLATED. A missing executable returns its I/O error. The
original stdout-plus-stderr transcript hash, supplied candidate, obligation and ambient-process
debt remain bound to the observation. Generic command producers retain their existing exit-status
contract. No evidence wire, policy, obligation ID or authority changes. `verify_self` calls the
same producer as `schema_compatibility_evidence`; the helper supplies one obligation, not full
self-scope admission. Process execution remains ambient cloud execution, not sandbox proof.

`historical-claims.json` pins the 63 preserved G127–G189 self-scope reports and their original
candidates/results at `706a1c431b8e7abbcd899ebb97893a681572b217`. Their reuse as evidence for the
changed/current source is UNSUPPORTED / STALE. This does not assert their original runs failed:
the old namespace existed then, and those artifacts do not record named outcomes. No historical
report, proof record, encoded artifact or saved ArtifactId is rewritten. Existing BLAKE3 proof
records have independent producer scopes; BLAKE2 is not BLAKE3 byte parity. Unknown rights remain
unqualified and do not authorize adoption.

The 2026-10-16 18:34 UTC owner direction supersedes the earlier implementation pause. The saved
STOP checkpoint and held source patch remain intact; the reviewed repair is resumed in the
current-main checkout. The committed-source observation will be appended separately after the
source commit exists. Until that observation is present, this source contribution claims no
new compatibility SATISFIED result. Full SEMANTIC/UNIT/COMPATIBILITY admission, native execution,
Chronica adoption, target receipts, rights and UI cutover remain separate obligations.
