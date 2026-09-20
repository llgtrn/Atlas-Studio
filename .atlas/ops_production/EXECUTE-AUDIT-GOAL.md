# Development Cell / Package Execute + Audit Goal

This compatibility path is the execution prompt for coding agents operating an acquired/donor-derived legacy `*Ops` repository as a **Development Cell** under the Chronica Package model.

The job is to advance real code while preserving donor reality, package identity, local documentation governance, independent development/test capability and convergence with canonical Chronica semantics. It must not build a standalone canonical Ops system.

The default loop is:

```text
READ REAL DEVELOPMENT CELL / PACKAGE STATE
-> VERIFY FLEET ALLOCATION / PRIMARY ABSORBER
-> VERIFY REAL DONOR BASELINE EXISTS IN CELL GIT
-> READ CURRENT CHRONICA @ EXACT SHA
-> EXTRACT DONOR SEMANTIC FINGERPRINT
-> SEARCH CHRONICA BY MEANING + CODE + TESTS
-> AUDIT CONFLICTS / DUPLICATION
-> CONVERGE VOCABULARY / SEMANTICS
-> CHOOSE ONE REAL DONOR VERTICAL
-> REFACTOR / MIGRATE CALLERS
-> TEST SEMANTIC PARITY
-> DELETE OBSOLETE DONOR PATH
-> MEASURE LEGACY REDUCTION
-> UPDATE PACKAGE CONTRACT / MIRROR / MANIFEST / DOCS
-> COMMIT
-> REPEAT
```

A run that only adds a new layer beside donor legacy is not successful migration. A run that preserves the same meaning under a second internal canonical name is also not successful convergence.

# GOAL

Advance the current Development Cell/package repository toward a production state that is simultaneously:

```text
REAL-DONOR-GIT-BASELINED
FULL-DONOR-INTAKE-DERIVED
LEGACY-BURNDOWN-TRACKED
CHRONICA-CONFLICT-AUDITED
SEMANTIC-CONVERGENCE-PROVEN
IP/PROVENANCE-ACCOUNTED
CHRONICA-BRANDED
INDEPENDENTLY-DEVELOPABLE
CHRONICA-COMPOSED
STANDALONE-SOVEREIGNTY-FORBIDDEN
ONE-PACKAGE-LOGIC / MANY-PORTS
MCP-OPERABLE
CHRONICA-COMPOSABLE
FABRIC-COMPATIBLE
SEMANTICALLY EXPLICIT
DOCS-GOVERNED
VERSIONED
RECONCILABLE
AUTHORITY-AWARE
EVIDENCE-PRODUCING
```

The unit of progress is a **real donor-derived vertical whose target behavior is proven, whose semantics deliberately converge with Chronica, and whose obsolete donor ownership is reduced**.

# 1. RESOLVE THE CHRONICA CONTRACT YOURSELF

Before designing a major abstraction, read current Chronica from `llgtrn/Chronica` at an exact SHA/ref and record it.

At minimum understand the relevant current owners for:

```text
docs/architecture/constitution/NORTH-STAR.md
docs/architecture/foundation/world.md
docs/architecture/foundation/resource.md
docs/architecture/foundation/state.md
docs/architecture/foundation/relation-binding.md
docs/architecture/foundation/capability-resolution.md
docs/architecture/governance/authority.md
docs/architecture/governance/normative.md
docs/architecture/governance/execution.md
docs/architecture/governance/evidence.md
docs/architecture/operations/fabric.md
docs/architecture/operations/recovery.md
```

Load intelligence/memory/context, machine/safety, sovereignty, performance and other owners when relevant.

Also load:

```text
docs/ops_production/README.md
docs/ops_production/PRODUCTION-NORTH-STAR.md
docs/ops_production/contracts/OPS-MANIFEST.md
docs/ops_production/contracts/ABSORPTION-CONTRACT.md
docs/ops_production/contracts/SEMANTIC-CONVERGENCE.md
docs/ops_production/donor/OSS-DONOR-CLONE-REFACTOR.md
docs/ops_production/donor/DONOR-CENSUS-PROVENANCE.md
docs/ops_production/engineering/REPO-STRUCTURE.md
```

Do not ask the human to paste these files.

# 2. PROVE REAL DEVELOPMENT CELL REPOSITORY STATE

Before substantive mutation inspect:

```text
pwd
git remote -v
git branch --show-current
git rev-parse HEAD
git status --short
git log --oneline -20
```

Then inspect real code, migrations, routes, jobs, UI, MCP, adapters, providers, tests, CI, docs, provenance and branding around the active vertical.

Preserve concurrent work. Never reset/force-push/discard unrelated later work.

# 2A. FLEET ALLOCATION GATE — FAIL CLOSED

Before cloning/importing a deep donor, prove:

```text
DONOR_ID exists in tools/refoundation/donor-corpus.yaml
FLEET_ALLOCATION covers DONOR_ID
PRIMARY_ABSORBER = this Development Cell/package
MULTIPLE_PRIMARY_ABSORBERS = 0
```

If this package is only a `REFERENCE_CONSUMER`, do not create another deep donor baseline. Consume the converged Chronica semantic/provider boundary or wait for the allocated primary's proof.

If the donor is missing from the master corpus, add/account for it with provenance before continuing. Never treat local discovery as permission to skip the fleet census.

# 2B. PACKAGE NON-SOVEREIGNTY GATE — FAIL CLOSED

Before substantive package refactor, require `chronica-package.json` and verify:

```text
development_mode = INDEPENDENTLY_DEVELOPABLE
production_mode = CHRONICA_COMPOSED
canonical_runtime = CHRONICA_REQUIRED
standalone_sovereignty = false
```

The Cell/package must not claim peer canonical ownership of:

```text
World
Identity
Authority
Execution
Evidence
Memory
Canonical History
Shared Truth
```

Local persistence must be classified as provider-local, cache/projection, fixture/simulation or other noncanonical state. A local test/dev authorization stub may exist for harness purposes; it must not be represented as production canonical authority.

# 3. REAL DONOR BASELINE GATE — FAIL CLOSED

The donor must not exist only as a URL or temporary checkout.

Prove:

```text
DONOR_REPO_URL
DONOR_SHA_OR_TAG
complete donor tree available/accounted for
CELL_BASELINE_COMMIT
CELL_BASELINE_TAG_OR_BRANCH
CELL_REMOTE_PUSHED=yes
baseline build/test status
submodule/LFS/generated boundaries
license/notice paths
acquisition/license/IP evidence state where relied upon
```

If the Ops repo does not yet contain the real donor baseline:

```text
clone/fetch real donor
-> import complete tracked tree into Development Cell repo
-> commit donor baseline
-> push baseline to Ops remote
-> record provenance
-> only then continue refactor
```

Do not replace this step with generated code based on donor documentation or memory.

If there is no acceptable donor and no explicit greenfield waiver:

```text
STOP: NO_ACCEPTABLE_DONOR
```

# 4. CHRONICA SEMANTIC CONVERGENCE + CONFLICT AUDIT — REQUIRED

Record:

```text
CHRONICA_REFERENCE_SHA
CHRONICA_OWNERS_READ
CHRONICA_CODE_PATHS_READ
CHRONICA_TEST_PATHS_READ where relevant
```

Do not search only for identical names. Before creating, retaining or renaming a material local abstraction, extract the donor/package semantic fingerprint:

```text
current names / aliases
resource or entity acted upon
inputs / outputs
preconditions / postconditions
state transition
side effects
authority semantics
normative semantics where real
idempotency / correlation
evidence produced
failure / UNKNOWN behavior
risk class where relevant
persistence ownership
provider dependency
real callers
tests
```

Then search Chronica for the **same meaning**, including synonyms, state transitions, effects, invariants, code symbols and tests.

At minimum compare against Chronica for:

```text
identity/resource/state model
relations/bindings
Capability Resolution / Can(...)
normativity / May-Must-Forbidden
authority / Authorized(...)
ExecutionAdmission / WorkRun
event/evidence/canonicalization
memory/context/sovereignty
machine/safety when relevant
recovery/reconciliation/idempotency
Fabric boundary semantics
persistent store ownership
core/runtime/adapter placement
crate/package/service naming
domain command/query/event vocabulary when already present
```

For every material semantic classify equivalence before coding:

```text
EXACT_EQUIVALENT
EQUIVALENT_WITH_DIFFERENT_NAME
CHRONICA_NARROWER
CHRONICA_BROADER
PROVIDER_MECHANIC_ONLY
PRODUCT_LOCAL
NO_EQUIVALENT_FOUND
UNRESOLVED
```

Then choose exactly one primary disposition:

```text
REUSE_CHRONICA_SEMANTIC
ALIAS_AT_BOUNDARY
MAP_TO_CHRONICA_SEMANTIC
EXTEND_CHRONICA_SEMANTIC
KEEP_PRODUCT_LOCAL
KEEP_PROVIDER_MECHANIC
TEMPORARY_COMPATIBILITY_ALIAS
RETIRE_DUPLICATE_SEMANTIC
DEFER_UNRESOLVED_WITH_EVIDENCE
```

A donor/provider name may survive at an adapter or compatibility boundary, but internal Ops semantics should use the same canonical Chronica vocabulary when meaning is equivalent.

Example:

```text
provider: resolve_ticket
        ↓ adapter translation
canonical Ops/Chronica semantic: conversation.close
```

Do not preserve a donor registry/store/policy engine/workflow engine merely because it already exists. If Chronica already owns the responsibility or meaning, converge rather than create a second universe.

If donor/Ops behavior is **richer than Chronica**, do not flatten it merely for naming conformity. Preserve and prove the richer distinction, determine whether it is universally reusable, and when it is generalizable route it toward the relevant Chronica canonical owner. Ops may prove a stronger candidate; Chronica decides canonical adoption. After adoption, converge Ops onto the canonical result.

Every substantial vertical should record enough semantic parity evidence to answer:

```text
DONOR_TERMS
OPS_TERM_BEFORE
SEMANTIC_FINGERPRINT
CHRONICA_REFERENCE_SHA
CHRONICA_OWNER
CHRONICA_CODE_REFS
CHRONICA_TEST_REFS
MATCH_STATUS
DISPOSITION
CANONICAL_TERM_AFTER
TEMPORARY_ALIASES
EXTENSION_GAP_IF_ANY
RETIREMENT_TARGET_IF_ANY
```

A conflict/semantic audit is not one-time intake paperwork. Refresh it whenever Chronica has materially advanced or before introducing a new major abstraction.

# 5. WHITE-LABEL / IP GATE

Where verified rights permit white-labeling, target an approved `Chronica <Product>` identity.

Audit donor identity surfaces:

```text
repo/package/module/crate names
product/app names
logos/icons/favicons
UI strings/page titles
email/notification templates
CLI/MCP descriptions
API/server banners
OAuth/bundle/service/container/image names
domains/docs/support links
telemetry identifiers
screenshots/seeds/demo content
legal notices
```

Classify each as:

```text
REBRAND_TO_CHRONICA
RETAIN_FOR_REQUIRED_ATTRIBUTION
RETAIN_AS_PROVIDER_REFERENCE
RETAIN_AS_PROVENANCE_ONLY
REMOVE
```

Never delete required legal/provenance notices merely to make branding visually clean.

# 6. DOCS KERNEL GATE

Every active Ops repo maintains:

```text
docs/README.md
docs/INDEX.md
docs/TEMPLATE.md
docs/architecture/
docs/blueprints/
docs/decisions/
docs/guides/
docs/references/
```

The Ops docs may own product-local workflows, providers, deployments, compatibility and migration/provenance facts. They MUST NOT redefine canonical Chronica World, Capability Resolution, Authority, Execution, Evidence, Memory, Context, Sovereignty or Safety.

If docs are stale because a real code responsibility moved, update the corresponding owner/decision/reference in the same coherent run.

# 7. BUILD THE LEGACY BURN-DOWN BASELINE

Before selecting work, measure the donor legacy surface at the current Ops HEAD.

Track at minimum:

```text
legacy_files_or_modules_remaining
legacy_public_callers_remaining
legacy_db_or_migration_ownership_remaining
legacy_ui_routes_remaining
legacy_background_jobs_remaining
legacy_policy_authority_paths_remaining
legacy_docs_brand_surfaces_remaining
legacy_duplicate_semantics_remaining
legacy_compatibility_aliases_remaining
```

The exact counting method may differ by repository, but it must be reproducible enough to show direction.

The terminal target is:

```text
legacy_code_remaining = 0
legacy_public_callers_remaining = 0
legacy_architecture_ownership_remaining = 0
unexplained_duplicate_semantics_remaining = 0
```

Required provenance/history/license/reference material is excluded from legacy-code counts.

# 8. AUDIT REAL PRODUCT + FABRIC PATH

For the active public workflow identify:

```text
resource/state
DESCRIPTIVE / NORMATIVE / COGNITIVE / ACTIVE aspects
current donor owner
intended Ops/Chronica owner
canonical semantic name
UI/API/CLI/MCP/worker callers
provider/persistence adapters
authority path
normative path where real
event/evidence path
idempotency/correlation
unknown-outcome behavior
reconciliation
context/disclosure boundary
standalone path
connected path
current tests
```

Keep:

```text
Transport != Semantics
Binding != Authority
Can(...) != Authorized(...) != May(...) != Must(...)
Provider Success != Reconciled Effect
ExecutionAdmission != Canonicalization
UNKNOWN != SUCCESS
```

# 9. CHOOSE ONE HIGH-VALUE VERTICAL

Prefer a vertical that removes real donor ownership while preserving behavior and reducing semantic duplication.

Priority:

```text
1. donor effectful path missing authority/idempotency/evidence/recovery
2. same meaning exists in Chronica under another name / duplicate semantic vocabulary
3. donor abstraction conflicting with a canonical Chronica owner
4. duplicated donor + new Ops business logic
5. donor policy/normative model that should converge with Chronica semantics
6. donor UI/API/MCP/worker callers that still bypass one application owner or use divergent verbs for the same effect
7. donor provider mechanics that should become an adapter
8. donor persistence/schema ownership needing controlled migration
9. donor branding/docs paths still defining product identity
10. tests required before deleting an obsolete donor path or compatibility alias
```

Do not choose a purely cosmetic rename when a substantive legacy owner can be retired safely.

# 10. REFACTOR, CONVERGE, MIGRATE, DELETE

For the selected vertical:

```text
READ REAL DONOR CODE
-> IDENTIFY REAL INVARIANTS
-> EXTRACT SEMANTIC FINGERPRINT
-> SEARCH CHRONICA BY MEANING + CODE + TESTS
-> CLASSIFY EQUIVALENCE
-> SELECT CONVERGENCE DISPOSITION
-> ADOPT CANONICAL VOCABULARY INTERNALLY
-> IMPLEMENT / EXTEND TARGET OWNER
-> MAP DONOR/PROVIDER ALIASES AT BOUNDARY
-> MIGRATE REAL CALLERS
-> MIGRATE UI/API/MCP/EVENT VOCABULARY WHERE SAFE
-> MIGRATE TESTS / DATA / MIGRATIONS AS REQUIRED
-> PROVE SEMANTIC PARITY
-> PROVE STANDALONE
-> PROVE MCP / CONNECTED MODE AS RELEVANT
-> DELETE OBSOLETE DONOR IMPLEMENTATION / DUPLICATE SEMANTIC
-> RE-RUN TESTS
```

Do not stop at:

```text
old donor implementation
+
new implementation
+
permanent adapter between them
```

or:

```text
same behavior
+
two permanent internal names
```

Temporary compatibility shims/aliases need explicit retirement criteria and should shrink over subsequent loops.

# 11. UPDATE MANIFEST WITH REAL EVIDENCE

Update `ops.manifest.yaml` when relevant:

```text
donor.ops_git_baseline
legacy_burndown
chronica_reference
semantic_convergence
rights
branding
documentation
contract versions
public surfaces
```

Do not claim:

```text
remote_pushed: true
ZERO_DONOR_LEGACY_IMPLEMENTATION
conflict_audit_status: proven
semantic_convergence.status: proven
branding.census_complete: true
documentation.governance_status: proven
```

without matching source/Git/test evidence.

# 12. TEST THE REAL PATH

Run narrow tests first, then broader affordable checks.

For semantic convergence/differential migration test as applicable:

```text
same valid input -> same accepted semantic outcome
same invalid input -> equivalent rejection class
same precondition -> same state transition
same effect -> same evidence requirement
same authority state -> same admission result
same idempotency key -> equivalent replay behavior
same event meaning -> same canonical classification
provider timeout -> equivalent UNKNOWN/reconciliation behavior
```

For effectful/recovery work additionally test as applicable:

```text
unauthorized
normatively forbidden
missing approval/procedure
stale precondition
idempotency replay
provider timeout
provider accepted but response lost
provider success contradicted by reconciliation
duplicate/out-of-order event
Chronica unavailable
process restart
partial sync
unsupported contract version
```

Never use live money, irreversible production effects or physical control merely to prove a test unless explicitly authorized and safely bounded.

# 13. SELF-AUDIT THE DIFF

Before commit answer internally:

```text
Did this run use the real pushed donor baseline?
Which donor legacy owner did it remove or reduce?
Did any old caller still bypass the new owner?
Did I read current Chronica at an exact SHA?
Did I search Chronica by meaning, not only by identical name?
Did I inspect relevant Chronica code/tests before inventing a new semantic?
Could any new Ops term be an alias for an existing Chronica meaning?
If Ops is richer than Chronica, did I preserve the richer distinction and record an extension/adoption decision?
Are provider/donor aliases confined to boundaries?
Do UI/API/MCP/workers use one internal semantic for the same effect?
Did I accidentally create a second World/Capability/Authority/Execution/Event/Memory/Safety universe?
Does standalone still work where promised?
Does MCP call the same application owner?
Are provider success and reconciled effect still distinct?
Does UNKNOWN remain durable?
Did required provenance/license/IP evidence survive?
Did white-label work avoid erasing required attribution?
Are docs and manifest truthful?
Did I preserve concurrent/later work?
```

If any answer is wrong, fix it before committing.

# 14. COMMIT DISCIPLINE

Commit only a coherent verified slice.

Do not force-push. Do not rewrite the donor baseline away. Do not create architecture-only placeholder modules for symmetry.

A successful coding run normally ends with:

```text
real implementation diff
real semantic convergence decision
real caller migration
real tests / parity evidence
legacy reduction
updated manifest/docs if semantics moved
one coherent forward commit
```

# 15. REPEAT UNTIL LEGACY + DUPLICATE SEMANTIC EXTINCTION

After one slice is green, remeasure the legacy surface and semantic duplicates, then continue to the next highest-value gap while the session can safely progress.

The long-run direction must be monotonic:

```text
DONOR LEGACY(t+1) <= DONOR LEGACY(t)
UNEXPLAINED DUPLICATE SEMANTICS(t+1) <= UNEXPLAINED DUPLICATE SEMANTICS(t)
```

If legacy temporarily increases because of a migration shim/alias, record why, bound it, and give it an explicit retirement gate.

Stop only when:

```text
selected work is verified
OR a genuine external/authority/license/blocker prevents safe progress
OR remaining work is too broad for the current run
```

# 16. FINAL REPORT — EVIDENCE ONLY

```text
OPS_REPO:
OPS_HEAD_BEFORE:
OPS_HEAD_AFTER:
DONOR_REPO_AND_SHA:
CELL_BASELINE_COMMIT:
OPS_BASELINE_PUSHED: yes/no
CHRONICA_REFERENCE_SHA:
CHRONICA_OWNERS_READ:
CHRONICA_CODE_PATHS_READ:
CHRONICA_TEST_PATHS_READ:

SEMANTIC_FINGERPRINT:
- ...
SEMANTIC_MATCH_STATUS:
- ...
SEMANTIC_DISPOSITION:
- ...
CANONICAL_TERM_AFTER:
- ...
TEMPORARY_ALIASES_REMAINING:
- ...
CHRONICA_EXTENSION_GAP:
- ...

CONFLICTS_FOUND:
- ...
CONFLICT_DECISIONS:
- ...

LEGACY_BEFORE:
- files/modules:
- callers:
- db/migrations:
- ui routes:
- jobs:
- policy/authority paths:
- docs/brand surfaces:
- duplicate semantics:
- compatibility aliases:

SLICE_EXECUTED:
- ...

CODE_CHANGED:
- ...

TESTS:
- PASS/FAIL ...

LEGACY_AFTER:
- files/modules:
- callers:
- db/migrations:
- ui routes:
- jobs:
- policy/authority paths:
- docs/brand surfaces:
- duplicate semantics:
- compatibility aliases:

LEGACY_DELTA:
- ...
SEMANTIC_CONVERGENCE_DELTA:
- ...

MANIFEST_UPDATED: yes/no
DOCS_UPDATED: yes/no
COMMIT:
- <sha> <message>

REMAINING_HIGHEST_VALUE_LEGACY_OR_SEMANTIC_DUPLICATE:
- ...
```

# FINAL RULE

**CLONE REAL DONOR INTO OPS -> PUSH BASELINE -> READ CURRENT CHRONICA -> EXTRACT DONOR SEMANTIC FINGERPRINT -> SEARCH CHRONICA BY MEANING + CODE + TESTS -> REUSE/MAP/EXTEND THE CANONICAL SEMANTIC -> REFACTOR ONE REAL VERTICAL -> MIGRATE CALLERS -> TEST SEMANTIC PARITY -> DELETE OBSOLETE DONOR PATH / DUPLICATE SEMANTIC -> MEASURE LEGACY + SEMANTIC CONVERGENCE -> COMMIT -> REPEAT UNTIL ZERO DONOR-OWNED LEGACY IMPLEMENTATION AND ZERO UNEXPLAINED DUPLICATE SEMANTICS.**
