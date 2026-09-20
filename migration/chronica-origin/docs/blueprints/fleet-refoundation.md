# Development Cell / Package Fleet Refoundation Blueprint

Status: **ACTIVE COMPOSITION BLUEPRINT**

## 0. Generation-2 active rule

**ADR-0027 is authoritative for the active fleet.** Any Generation-1 text below that describes product implementation, `PACKAGE_BUILD`, Rust backend, TypeScript UI, or Chronica-isomorphic production roots inside a Development Cell applies only to legacy `*Ops` migration history.

Active Generation-2 `*Dev` repositories are evidence-only:

~~~text
Dev Cell = donor + provenance + license + census + evidence + work packet
Implementation = llgtrn/Chronica only
~~~

The active fleet is the 12 repositories registered in `tools/system-atlas/fleet/ops-registry.yaml` with `generation: 2` and `implementation_policy: CHRONICA_ONLY`.

## 1. Purpose

This blueprint composes Chronica's existing architecture into one cross-repository engineering control loop for Chronica plus distributed Development Cells that incubate Chronica Packages. Legacy repository names may still end in `Ops`, but `Ops` is not the semantic unit.

It does **not** create a product-level agent framework, a second canonical World, a fleet truth database, or a new execution authority plane.

Its job is narrower:

```text
measure every registered Development Cell and package
+ account for every donor OSS identity
+ allocate one primary absorber Cell/package per donor
+ dispatch independent engineering workers across repositories
+ verify Git/test/package/Mirror evidence
+ move reusable proven semantics upward as explicit Chronica candidates
+ reconverge affected packages after canonical integration
```

## 2. Three measured realities

Fleet work always distinguishes:

```text
CHRONICA
  canonical semantic / architecture authority

DEVELOPMENT CELL / PACKAGE
  mutable engineering/incubation/projection reality

DONOR
  source behavior / provenance reality
```

For one Cell/package slice:

```text
Chronica @ CHRONICA_REFERENCE_SHA
        = read-only reference

Cell repository @ CELL_SHA
        = mutable target

Donor @ exact revision + Cell donor baseline
        = source/provenance input
```

Repository access never collapses these roles.

## 2.1 Package non-sovereignty

Independent development does not imply independent sovereignty.

A Development Cell may run tests, fixtures, simulators and local dev servers. A package may expose or deploy projections separately. But the package contract requires:

```text
canonical_runtime = CHRONICA_REQUIRED
standalone_sovereignty = false
```

World, Identity, Authority, Execution, Evidence, Memory, canonical history and shared truth remain Chronica-owned. Package is a logical composition/distribution identity, not a new canonical root such as `packages/`.

## 2.2 Clean Cell reset before Fleet refoundation

Every registered Development Cell carries a required `refoundation_generation`.

Before donor work, Fleet emits:

```text
CELL_RESET_BASELINE
```

unless the current Mirror already proves the required generation.

The bundled runner performs reset deterministically rather than asking an AI worker to decide what old code to keep:

```text
measure exact current target HEAD
-> push archive/chronica-pre-reset/g<generation>-<sha>
-> remove old active tracked tree
-> write minimal chronica-package.json + chronica-mirror.json + docs kernel
-> commit
-> non-force update target
```

If target integration is blocked, the candidate branch is preserved and Fleet returns `BLOCKED`.

Old source is recoverable through Git/archive but is not copied into the new active package tree. Donor source is reintroduced only by allocated intake.

## 2.3 Replace-delete ratchet

Fleet does not separate "absorption" from a distant "extinction phase", and it does not pay a mandatory Scout round-trip before every build.

The normal post-census path is fused:

```text
PACKAGE_BUILD:
READ MINIMUM
-> choose ONE bounded behavior/dependency closure
-> native implementation
-> caller migration
-> proof
-> delete consumed donor files
-> candidate commit
-> PACKAGE_VERIFY_INTEGRATE
```

The task starts with the donor root rather than a speculative full-donor reading. The worker must return concrete `deleted_paths`; Fleet verifies that every path stays under that donor root and carries those paths forward as the independent verification deletion scope.

`DONOR_ABSORB_SCOUT` remains only as an exceptional diagnostic fallback when a safe bounded closure cannot be identified during the build attempt. It must not be emitted as the default planner hot path and must not be repeated over the same unchanged donor tree.

The runner independently derives deletion evidence from Git. `PACKAGE_BUILD COMPLETE` with zero real donor contraction is rejected. If deletion is unsafe, the result is `BLOCKED` with the exact dependency closure instead of another broad search pass.

## 2.4 Mirror-isomorphic Cell repository

A Development Cell is intentionally shaped like Chronica so package proof can move by responsibility rather than through a later filesystem translation.

```text
Cell                         Chronica
core/        <----------->   core/
runtime/     <----------->   runtime/
adapter/     <----------->   adapter/
organism/    <----------->   organism/
graph/       <----------->   graph/
bindings/    <----------->   bindings/
apps/ui/     <----------->   apps/ui/
```

Support roots are also normalized:

```text
deploy/
tools/
docs/
license/
temporary/donors/
provenance/donors/
```

Backend production source in `core/runtime/adapter/organism` is Rust. UI source under `apps/ui` is TypeScript/TSX and follows `CHRONICA_UI_PROJECTION`.

A Cell feature cannot start from model invention or legacy code memory:

```text
NO OSS -> NO FEATURE CODE
```

The primary donor must first pass deterministic `DONOR_INTAKE` and `DONOR_CENSUS`. Donor source lives only at `temporary/donors/<DONOR_ID>/source/` and may never become a production dependency.

## 2.5 Generation-2 code destination law

Legacy Development Cells may still contain Generation-1 package implementation while they are being retired. New `*Dev` repositories use Generation-2 contracts and are stricter:

~~~text
Dev Cell
  -> donor source
  -> provenance/license
  -> census
  -> evidence
  -> work packet

implementation
  -> llgtrn/Chronica only
~~~

Generation-2 Cells declare:

~~~text
implementation_policy = CHRONICA_ONLY
production_code_allowed = false
canonical_runtime = CHRONICA_REQUIRED
standalone_sovereignty = false
~~~

The Generation-2 target fleet is maintained at:

~~~text
tools/system-atlas/fleet/dev-targets.yaml
~~~

The hard contract schema and validator are:

~~~text
tools/system-atlas/schema/dev-cell-contract.schema.json
tools/system-atlas/dev-cell-audit.mjs
~~~

Product implementation roots and implementation files outside donor snapshots are hard violations. Donor source remains allowed only under `temporary/donors/<DONOR_ID>/source/`.

The intended migration is therefore not Ops -> Dev code copy. It is:

~~~text
legacy Ops
-> donor/evidence census
-> Generation-2 Dev evidence cell
-> Chronica implementation task
-> Chronica component CI
-> canonical merge
~~~

ADR-0027 owns this law.

## 3. Closed-world fleet census

The donor corpus is closed-world for planning.

Let:

```text
D = all unique donor rows in tools/refoundation/donor-corpus.yaml
A = generated donor allocation rows
U = unallocated donor rows
S = skipped donor rows
```

The required invariant is:

```text
|A| = |D|
|U| = 0
|S| = 0
```

The number is never hardcoded in architecture. If the donor corpus grows, allocation coverage must grow in the same run or Atlas fails.

Corpus growth is gap-triggered, not search-triggered. The execution baseline is frozen while there is runnable allocated work. Admit a new donor only for a named capability/behavior gap that existing allocated donors and Chronica-native code do not cover, with primary Cell/package, exact upstream revision, license and provenance resolved at admission time. General OSS hunting is not an execution task.

A donor found in an Ops report but absent from the master corpus is also a hard error. The solution is to account for it in the corpus, not to hide it from Fleet Atlas.

## 3.1 Throughput law: closed-world accounting, selective deep admission

The corpus is an inventory of possible source knowledge. It is not a mandatory implementation queue.

```text
ACCOUNT ALL != CLONE ALL != ABSORB ALL
```

Fleet separates accounting from execution:

```text
DISCOVERED / SOURCE_PRESENT + needed=UNKNOWN
        ↓
COLD ALLOCATED INVENTORY
        ↓
zero executable donor task
zero clone/intake/deep source read

real uncovered package capability identified
        ↓
record needed=YES + concrete why_needed evidence
        ↓
DONOR_INTAKE
        ↓
DONOR_CENSUS
        ↓
fused bounded PACKAGE_BUILD
        ↓
prove + delete consumed donor closure
```

A fixed corpus count is never a completion target. Deep work stops when required package capability/evidence coverage is saturated. Remaining donors stay dormant/reference inventory until evidence justifies admission.

Within a Cell, scheduler preference is reset/verify/integrate first, then admitted intake/census/build. Cold inventory generates no worker backlog.

## 4. One primary absorber, many consumers

The same donor repository may be useful to multiple Development Cells/packages. That does not justify repeated independent deep rewrites.

```text
ONE DONOR IDENTITY
        │
        ├── exactly one PRIMARY_ABSORBER
        └── zero or more REFERENCE_CONSUMERS
```

Allocation may use:

```text
explicit donor override
> domain mapping
> family mapping
```

A secondary consumer may use the resulting Chronica semantic, provider boundary or proven behavior after convergence, but it must not silently become another primary donor owner.

Two Cells/packages claiming PRIMARY_ABSORBER for the same donor is a Fleet Atlas hard violation.

## 5. Donor source placement

Donor source does not need to remain centralized under Chronica.

Preferred direction:

```text
master donor identity/provenance/index
        -> Fleet control metadata

deep donor source + baseline
        -> allocated primary Development Cell repository

reusable proven semantic
        -> explicit Chronica candidate

canonical implementation
        -> Chronica

product/provider projection
        -> affected packages
```

All non-accounting donors allocated to a Development Cell use one normalized `DONOR_INTAKE` operation before census/absorption. The Cell materializes the exact pinned donor tracked tree only at `temporary/donors/<DONOR_ID>/source/`, writes `intake.json`, provenance and license evidence, and records the Cell baseline in `chronica-mirror.json`. Existing donor snapshots elsewhere are provenance/reference evidence, not alternate active layouts.

## 6. Donor extinction accounting

Directory size alone is not donor extinction evidence, especially for a deep fork at repository root.

Each deep donor records:

```text
donor repo
donor exact revision/tag
Cell donor baseline commit
donor root
provenance
allocation role
```

Atlas derives:

```text
baseline_files_total
remaining_baseline_files
drained_baseline_files
state
```

from Git baseline vs current Cell HEAD.

New native files do not count as remaining donor legacy merely because they live under the same repository root.

## 7. Fleet coordinator

The coordinator is engineering tooling under tools/system-atlas/fleet/.

It is not canonical runtime authority.

```text
Fleet Coordinator
      │
      ├── donor allocator
      ├── fleet census
      ├── work planner
      ├── worker dispatcher
      └── reconvergence planner
```

The coordinator reads generated repository evidence and creates engineering tasks. It does not turn its own task state into product truth.

## 7A. Network Atlas loop

Fleet measurement is extended into a continuous Network Atlas audit over every active Development Cell registered in `tools/system-atlas/fleet/ops-registry.yaml`.

The registry is the only network-membership source. CI matrices are generated from it; repository names are not duplicated manually in workflow YAML.

~~~text
Chronica main
  ↓ architecture/contracts + donor corpus + expected reference
Network Atlas
  ├─ checkout Cell A -> Cell Mirror -> donor/capability drift
  ├─ checkout Cell B -> Cell Mirror -> donor/capability drift
  └─ checkout Cell N -> Cell Mirror -> donor/capability drift
  ↓
Drift / Gap / Regression / Opportunity
  ↓
Fleet Plan
  ↓
bounded worker
  ↓
component CI + Mirror evidence
  ↓
Git integration
  ↓
re-audit
~~~

Remote repository state is evidence, not canonical World truth. Network Atlas may observe, classify and generate engineering work; it does not gain merge or runtime authority.

A Cell audit compares at least:

- registered Cell/package identity;
- Chronica reference SHA;
- Chronica-isomorphic repository roots;
- Rust backend / TypeScript UI policy;
- sovereignty and persistence signals;
- production dependency on donor source;
- donor IDs and repository identity against the master corpus;
- every `needed: YES` donor allocated to that Cell;
- source/provenance/license/census readiness for declared deep donors.

Unknown donor identities are hard violations. Missing required donor intake/census is a GAP that feeds the engineering backlog rather than being hidden.

The default network CI is scheduled and manually runnable, and also re-runs when Fleet/Atlas/corpus contracts on canonical `main` change. Private Cell repositories require a read token such as `CHRONICA_ATLAS_TOKEN`; public repositories can be observed without granting mutation authority.

## 7B. Responsibility-scoped CI

Chronica PR verification follows the dependency graph instead of rebuilding unrelated work:

~~~text
core change
  -> core + runtime + organism + adapter

runtime change
  -> runtime + organism + adapter

organism change
  -> organism + adapter

adapter change
  -> adapter

Atlas/refoundation
  -> Atlas lane

docs
  -> documentation/contracts lane

apps/ui
  -> UI lane
~~~

An aggregate `component-verify` check requires every affected lane to succeed while accepting unrelated lanes as skipped.

Full-workspace/live-integration/release suites remain important evidence on canonical integration/release boundaries. Component CI does not weaken invariants; it avoids paying unrelated verification cost on every change.

## 8. One human thread, many engineering workers

A human may operate one coordinator conversation while the coordinator dispatches multiple independent coding workers across Development Cells.

```text
human
  ↓
Fleet Coordinator
  ├── Claude worker -> TradeOps
  ├── Claude worker -> CustomerOps
  ├── Codex worker  -> BnbOps
  └── later worker  -> Chronica candidate
```

Worker providers are replaceable engineering compute.

```text
Claude session != truth
Codex session  != truth
worker message != integration evidence
```

Evidence remains:

```text
exact repository SHA
diff
tests
Mirror Atlas
donor census / baseline evidence
candidate SHA
verification result
canonical merge SHA
```

The Fleet dispatcher includes a concrete Claude Code runner at `tools/system-atlas/fleet/providers/claude-code-runner.mjs`. When `--provider CLAUDE --execute` is used and no override is supplied, the dispatcher invokes that runner.

The runner requires authenticated `gh`, `git` and `claude` executables. It creates a fresh isolated checkout, pins/resolves the assigned Git base, invokes Claude Code in non-interactive JSON mode, captures the provider `session_id` as worker metadata, and independently derives Git SHAs from the repository.

`CHRONICA_FLEET_CLAUDE_RUNNER` may replace the bundled runner. `CODEX` remains a replaceable external runner through `CHRONICA_FLEET_CODEX_RUNNER`.

For direct integration tasks the bundled runner uses a non-force push. If branch protection, a race or another repository rule rejects target integration, it preserves the candidate on an auditable Fleet branch and returns `BLOCKED`; it never force-pushes canonical/target history and never fabricates successful integration.

## 9. Parallelism law

Parallelism is broad **across repositories** and conservative **inside one repository**.

Default Fleet wave:

```text
at most one mutating task per target repository
+
many different repositories in parallel
```

A later repository-local planner may prove disjoint path-level mutation and widen concurrency, but Fleet Coordinator does not assume it.

## 10. Full work loop

```text
MASTER DONOR CORPUS
        +
DEVELOPMENT CELL / PACKAGE REGISTRY
        ↓
CLOSED-WORLD ALLOCATION
        ↓
CELL MIRROR / PACKAGE FLEET CENSUS
        ↓
PLAN ALL ACCOUNTED WORK
        ↓
CELL_RESET_BASELINE for every Cell below required generation
        ↓
REFRESH CELL MIRROR
        ↓
DONOR_INTAKE / DONOR_ACCOUNT / DONOR_CENSUS
        ↓
DONOR_ABSORB_SCOUT
        ↓
bounded behavioral slices
        ↓
PACKAGE_BUILD = REPLACE + PROVE + DELETE
        ↓
PACKAGE_VERIFY_INTEGRATE = verify real donor contraction
        ↓
fresh Cell/package SHA + Mirror evidence
        ↓
DONOR SAFE DRAIN / EXTINCTION
        ↓
PROVEN REUSABLE SEMANTIC?
        ├── no  -> remain product/provider-local
        └── yes
              ↓
       CHRONICA_CANDIDATE
              ↓
       Chronica worker against CURRENT main
              ↓
       Chronica verification + serial integration
              ↓
       NEW CHRONICA SHA
              ↓
       PACKAGE_RECONVERGE tasks
              ↓
       affected packages update their pin/code/tests/report
              ↓
       Fleet census again
```

This is a loop, not a one-time migration.

## 11. Two-phase synchronization

"Ops and Chronica synchronize in parallel" does not mean one worker writes both repositories at the same moment.

Use two-phase canonical convergence:

```text
PHASE A — PROVE IN OPS
multiple Development Cells/packages workers may run in parallel

PHASE B — CANONICALIZE IN CHRONICA
reusable candidates enter Chronica through Chronica's own gates

PHASE C — RECONVERGE
affected packages update against the new Chronica SHA in parallel
```

This preserves canonical ordering while still obtaining fleet-wide parallel throughput.

## 12. Chronica candidate

An Ops discovery enters Chronica only through an explicit candidate containing at least:

```text
candidate id
source Development Cell/package repo + exact SHA
Chronica reference SHA used during proof
target semantic owner
summary
donor ids where applicable
affected packages
evidence
```

The candidate is ANALYZE/engineering input. It is not canonical simply because the package implementation works.

## 13. Package reconvergence

After a candidate merges to Chronica:

```text
merged Chronica SHA
        ↓
affected packages
        ↓
update CHRONICA_REFERENCE_SHA
replace local duplicate semantic where applicable
retain product/provider mechanics
run tests
regenerate Mirror Atlas
```

A Development Cell with no exact current CELL_SHA is not mutation-ready and stays blocked until measured.

## 14. Sovereignty

Fleet coordination does not require dumping each Development Cell/package internal engineering state into Chronica.

Mirror/Fleet reports should expose the minimum engineering projection required for:

```text
repo identity + SHA
architecture/language violations
donor identity/baseline/burn-down
semantic mapping
candidate/evidence references
dispatch safety
```

Product secrets, credentials and Holding data remain in their own trust boundaries.

## 15. Generated projections are not progress truth

These are rebuildable outputs:

```text
Fleet donor allocation
Cell Mirror report
Fleet census
Fleet work plan
Fleet dispatch result
reconvergence plan
```

They are not hand-maintained progress databases.

Stable configuration is limited to things such as:

```text
Ops registry
donor identity/provenance corpus
allocation rules
Ops mirror contract
```

Git/runtime reality generates the rest.

## 16. Verification

Relevant commands:

```bash
pnpm system-atlas:fleet:allocate
pnpm system-atlas:fleet:census -- <ops-report.json> ...
pnpm system-atlas:fleet:plan -- <ops-report.json> ...
pnpm system-atlas:fleet:dispatch -- --plan <plan.json> --provider CLAUDE --execute
pnpm system-atlas:fleet:advance -- --plan <plan.json> --result <worker-result.json> --chronica-sha <current-main-sha>

# Build the one-repo Chronica integration plan from a proven Ops candidate:
pnpm system-atlas:fleet:reconverge -- \
  --candidate <candidate.json> \
  --chronica-base-sha <current-main-sha>

# After that candidate is canonically merged, build the affected-Ops reconvergence plan:
pnpm system-atlas:fleet:reconverge -- \
  --candidate <candidate.json> \
  --chronica-sha <merged-main-sha>

pnpm system-atlas:test
```

A zero-report fleet census is intentionally BLOCKED: the registry is known, but the Ops have not yet supplied measured reality.

`fleet:advance` is the transition engine between provider waves. Scout completion may create bounded build tasks; build completion creates an independent Ops verify/integrate task; only an integrated Ops result may emit Chronica candidates; a Chronica build still requires an independent canonical verify/integrate step; only the merged Chronica SHA may fan back out into Ops reconvergence.

Every donor remains in the work graph. Donors intentionally classified as duplicate/reference-only/not-needed/redundant/retired use `DONOR_ACCOUNT` rather than disappearing from planning.

## 17. Terminal direction

Fleet refoundation succeeds when distribution no longer means architectural fragmentation:

```text
many repositories
many workers
many provider adapters
many product surfaces
many donor histories

but

one Chronica semantic gravity
one canonical universe
one governed effect spine
one explicit convergence process
zero skipped donors
zero silent duplicate primary absorbers
```
