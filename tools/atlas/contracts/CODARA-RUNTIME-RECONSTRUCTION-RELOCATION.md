# Runtime reconstruction residence and target caller boundary

This bounded source increment starts at Atlas
`492406ea795bd6fe73018e15c2c545353b0e33b4`. It physically relocates
`adapter/runtime/src/self_reconstruction.rs` into
`adapter/runtime/src/codara/reconstruction.rs`, inside the unchanged `runtime`
package and canonical `compiler.runtime` owner. Root retains the historical logical
`self_reconstruction` module through `#[path]`; `codara::reconstruction` is a public
alias of those same definitions. `Candidate`, `Attempt`, constants, function
signatures and production function bodies remain identical. There is one implementation.

Exactly six production references in the existing CLI `self-reconstruct
candidates|roots|attempt` dispatch switch to the alias. Arguments, sorting, output
JSON/text, errors, parent-root check, shadow paths and offline commands remain unchanged.
The only edit inside the moved implementation is one test's relative `include_str!`:
`../../../core/src/donor/mod.rs` becomes `../../../../core/src/donor/mod.rs`.
The included donor bytes and target identity remain unchanged. All eight historical
`self_reconstruction::tests::*` selectors and the existing reconstruction ledger test
remain valid. No historical schema, native knowledge, root, attempt, report, witness,
shadow or proof identity is rewritten.

## Existing inputs, outputs and effects

`candidates(WorldModel)` returns the same ordered `Candidate` values, excluding observed
effects, state, persistence, concurrency, resource use, leaving/unresolved calls and
unsafe/async functions from constructibility. The CLI retains its schema, revision,
full counts and optional output limit. This is discovery, not execution authority.

`target_roots(records, path, type)` returns the existing ordered SYMBOL and
FUNCTION_IDENTITY record references. The CLI first reads the complete existing container
through `design::read_container`; the root is recomputed from bytes, not trusted input.

`Attempt` carries `root`, complete `CensusAtlas`, decoded `container_root`, target
`path`/`type_name`, `design_id`/`design_state` and optional `comparison_id`. The CLI
reads the design/comparison and requires `design.parent_root == container_root` before
attempting work. `lift` consumes typed records and these references, never original
source text. Missing declarations/shapes/bodies stay explicit typed gaps. It returns the
same content-identified `ConstructionModule`; the existing backend produces `RustEmission`.

If nothing emits, the result remains unsupported or a construction gap without creating
a shadow. Otherwise the existing hosted workflow writes `target/atlas/shadow`, Cargo
manifest/lock, source and differential tests; executes offline Cargo against compiled
original Core; and checks the shadow census. Original source/oracle execution remains
verification input, never construction input. It returns the existing content-identified
`SelfReconstructionReport`, with toolchain, oracle uses, checks, gaps, variations and verdict.
The CLI validates module/report, writes module.json/report.json/shadow.rs.txt, and preserves
its existing refusal behavior. Local source validation is not Chronica work admission.

`oracle_path` remains the existing syntactic `core/src/*.rs` mapping, including lib/mod
handling. It is not a public-type whitelist or authority gate; actual compiler/differential
checks qualify an oracle. No runtime path was added to that mapping because this file moved.
`SHADOW_ROOT`, all schemas, encodings, lineage strings, module/report identity algorithms
and package/dependency identities remain unchanged.

## Measured source identity and regression

A fresh isolated full-index native witness starts from the prior verified 2559-row
edited-source registry. Actual baseline sync observes 2566 artifacts and retains that
entire seed and original 2502 saved IDs. Actual git movement is observed before any edit:
zero births, one move, zero changes, retirements or ambiguities. All 2566 IDs remain.
The moved native artifact stays `art1:5840ab4120952af87788c8ac9c138fe1`, owner
`compiler.runtime`, with original SHA256
`4766575ab49868be918c1cfeef3149ece3ac5743fee27da1b227dfacfd0598dd`.
Afterward the fixture include changes full-file SHA256 to
`03215bb4f8302dfa33d3fa9a778b21e0fc3bc1f84fa7d5e524835ad9032ffdae`;
all production bodies remain exact. Prior witnesses and active checkout state are untouched.

The actual existing `adapter::inventory_source` producer over scoped fixtures observes
path-derived coordinate `artifact:758c0ca9abc83983` at the old path and
`artifact:08a77c9d8de25769` at the new path. At the byte move both have BLAKE3
`57a8d162b8d413e451fc197a9d71c2b211c36d3f094795f83a65771c5e5d36fe`;
after the include adjustment the new digest is
`e3138448eca5048b7b0738b9f96b6b6538c561063a42bbed5ebe1240c5386e4f`.
These path coordinates legitimately differ; native art1 stability does not equate them.
Current function observations also carry new physical spans/revision and require explicit
recensus correspondence. Old observations retain their qualified historical meaning.

Runtime regression compares both public routes over actual immutable Core observations:
full module/identity/lineage, roots, historical type names, emission/semantic checks and
a body falsifier, Core-only oracle mapping, missing target and honest unsupported bodies.
CLI regression derives an actual model/container from one owned source fixture and runs
all three real verbs, including unsafe exclusion, candidate limit/counts, exact roots,
full gap module/report/output, validation and parent mismatch before outputs. Its design
is explicitly untrusted CANDIDATE data, never selection or a grant. Its no-emission attempt
avoids nested Cargo; successful hosted shadow verification is a separate local workflow.
Failed development checks remain recorded; the inherited CLI collapsible_if lint101 is
not fixed, suppressed or included in an all-checks-pass claim.

The actual candidate requires fresh before/after/replay census, exact intended entity and
signed-total deltas, and full source library admission. Existing semantic/unit/compatibility
obligations remain intact. Final publication requires a fresh complete source container at
the exact clean published HEAD, verify with the same absolute root and pinned environment,
then complete byte/evidence/identity readback. Its manifest stores HEAD; later commits make
that container historical. Final artifacts stay outside the repository with no later commit.
The previous 492406ea container qualification is preserved as historical evidence, not rerun
or rebound to this changed source. UNSEALED and all current blockers remain explicit.

## Concrete Chronica caller dependencies

The read-only target audit is pinned to Chronica
`59f4e409a61ef1044515d80f7dca66e7decead4d`, observed 2026-10-17T00:01:46Z.
Its complete file/blob/line evidence and source fixture measurements are appended in
`tools/atlas/evidence/census/runtime-reconstruction-relocation/target-contract-audit.json`.
Moving main can advance; this is an immutable inspection, not an executed target census.

That target has real immutable source-assertion capture/read: Lineara SourceObservation
binds exact bytes to complete TargetBinding; Host import/observe uses current Holding,
purpose, lease, request, provider/version and durable known-outcome work. Allocation also
has a private receipt-codec seam. Evaluate those existing mechanisms first. The dedicated
`chronica.pump-source-import` provider and its known one-unit validation cost do not define
reconstruction semantics, CPU demand, source truth, authority or a generic output budget.

Both existing Codara FUNCTIONS decoder and ordinary Lineara work receipt still cap results
at 1024 bytes. Historical reconstruction module/report fixtures already reach 5094/6491
bytes; candidate JSON is 1908 bytes. ArtifactBook resolves metadata, not body bytes. A digest,
source path or printed prefix cannot substitute for complete immutable evidence. The new
source-observation seam changes which existing mechanism can be reused, not these contracts.

The target-owned integration must bind exact verified container bytes/root, path/type and
design/comparison to the admitted request/IR/effect and a pinned implementation; retain
lossless module/emission/report/gap/check evidence with authorized readback; validate capacity
before publishing success references; and preserve refusal, unknown and no-rerun behavior.
Authentication, grants, lease/work, durable pre-perform binding, known settlement and current
resource.observe/disclosure remain required. Do not borrow the BLAKE3 caller's one-unit demand
or widen its capsule/shared wire/budgets. The hosted shadow/compiler path separately needs
admitted filesystem/subprocess effects, toolchain/dependency closure, bounds and isolation.
No such target changes or authority decisions are made in this source increment.

This closes the third of three specifically inventoried runtime source responsibilities.
It does not retire supported historical APIs, prove native target parity, extinguish any OSS
dependency, resolve Atlas UNLICENSED/OD-15 rights or complete native Chronica OS V1.
Those obligations remain mandatory and separate. UI remains exclusively Chronica/Phenora.
