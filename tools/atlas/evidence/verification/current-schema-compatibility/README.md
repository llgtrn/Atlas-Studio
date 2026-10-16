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
current-main checkout.

The [committed-source observation](source-ef99a8b-run.json) records the actual existing CLI
`verification self` caller on source `ef99a8b80c6e916d91018289fc480602371e9c3a`. Its
[schema/history evidence](source-ef99a8b-self-scope.json) is SATISFIED for the exact three-test
compatibility run. After ten minutes of caller validation, its long-running workspace UNIT
Runtime test process was deliberately interrupted with SIGTERM. The actual UNIT evidence is
VIOLATED and the complete caller verdict is BLOCKED; the full workspace suite is
**NOT COMPLETED**, not a reported intrinsic regression failure. The receipt retains process
ancestry, timing, interruption and the original counterexample. This demonstrates fail-closed
caller behavior when UNIT is interrupted. The [semantic input](source-ef99a8b-self-replay.json) is actual same-current-source
deterministic self-replay, not a cross-revision semantic change proof. Its candidate differs from
the caller's snapshot candidate, so the caller correctly records SEMANTIC as UNVERIFIED. The
snapshot/prove commands used relative root `.` while the caller used an absolute root; both
digests remain intact and are not rebound or treated as interchangeable. Preserve the separate
results and their source/candidate/transcript bindings. This source observation does not qualify
Chronica adoption, native execution, full V1, hardware, rights or UI cutover.
