---
id: ops-production-readme
type: reference
status: active
canonical: true
---
# Atlas Ops Production

`ops_production` supports building, validating and operating Atlas-Systemizer itself.

It is not a product domain, not an organism layer, and not a second architecture universe. Its job is to make Atlas production work auditable:

```text
Atlas source
-> repository systemization
-> donor provenance and license checks
-> production-readiness evidence
-> bounded work preparation
-> verification reports
```

Canonical runtime ownership remains:

```text
core/             product-neutral semantics
runtime/          orchestration and work preparation
adapter/          external mechanics translated into Atlas facts
ops_production/   Atlas support/readiness kernels
apps/ui/          TypeScript UI projection
graph/            declarative graph definitions
bindings/         declarative binding definitions
```

The legacy `organism` lane is intentionally not part of Atlas-Systemizer. AI-assisted analysis belongs behind explicit provider/adaptation boundaries and remains proposal/inference until runtime validation converts it into evidence.

## OSS Donors

Donor source belongs only under:

```text
.atlas/temporary/donors/
```

License copies belong under:

```text
.atlas/licenses/
```

Provenance belongs under:

```text
.atlas/provenance/
```

Donor code is reference material and evidence. It is not a permanent runtime owner. Native promotion requires census, license review, target mapping, Atlas-owned implementation, tests and evidence.

## Evidence

Ops production work should emit or update:

```text
.atlas/evidence/
.atlas/reports/
.atlas/provenance/
.atlas/references/donor-corpus.toml
```

Claims without evidence stay pending. Legacy files with no remaining study value should be extincted rather than kept as working-tree noise.
-> EXTRACT DONOR SEMANTIC FINGERPRINT
-> SEARCH CHRONICA BY MEANING + CODE + TESTS
-> SEMANTIC CONVERGENCE + CONFLICT AUDIT
-> CHRONICA PACKAGE REFOUNDATION
-> VERTICAL REFACTOR + REAL CALLER MIGRATION
-> SEMANTIC PARITY PROOF
-> LEGACY + DUPLICATE-SEMANTIC BURN-DOWN
-> LOCAL HARNESS / MCP / CHRONICA-COMPOSED PROOF
-> ABSORPTION PLAN
-> DOCS-SYNC PROPOSAL WHEN CANONICAL MEANING CHANGES
-> DEPLOYMENT / RUNTIME EVIDENCE
-> NATIVE CHRONICA ABSORPTION
```

A package projection and MCP/API/UI surface may remain separately deployable where useful. That deployment must not become a peer canonical World/Authority/Execution/truth system. The independent Cell source universe should not remain a competing native implementation source after terminal convergence.

## Primary entrypoints

For any new external code/source intake:

```text
docs/ops_production/contracts/SOURCE-ADMISSION.md
```

For normal admitted donor/Ops refactor/coding:

```text
docs/ops_production/EXECUTE-AUDIT-GOAL.md
```

For terminal source absorption:

```text
docs/ops_production/ABSORB-INTO-CHRONICA-GOAL.md
```

Four contracts should be treated as the shared operational core:

```text
docs/ops_production/contracts/SOURCE-ADMISSION.md
docs/ops_production/contracts/OPS-MANIFEST.md
docs/ops_production/contracts/SEMANTIC-CONVERGENCE.md
docs/ops_production/contracts/LOGISTICS-KERNEL.md
```

`SOURCE-ADMISSION.md` classifies external code before trust/ownership. `OPS-MANIFEST.md` is the machine-readable discovery surface. `SEMANTIC-CONVERGENCE.md` prevents same-meaning/different-name semantic forks. `LOGISTICS-KERNEL.md` defines executable Reality Mirror, semantic checking, absorption planning, docs-sync proposals, CI and deployment/runtime evidence.

## Non-negotiable rules

1. **FLEET ALLOCATION BEFORE DEEP DONOR OWNERSHIP.** Every master donor remains counted; one donor has exactly one primary absorber and zero or more reference consumers. A local Ops may not silently claim an unallocated or differently allocated donor.
2. **SOURCE ADMISSION BEFORE TRUST.** Classify external code as `DEEP_FORK`, `VENDORED_SOURCE`, `PACKAGE_DEPENDENCY`, `SUBMODULE`, `EXTERNAL_PROVIDER`, `REFERENCE_IMPLEMENTATION`, `SPEC_ONLY` or reject it; pin exact identity; verify integrity/provenance, rights/license compatibility, supply chain, security, secrets/data contamination and upstream lifecycle before treating it as trusted production source.
3. **DO NOT DEEP-FORK EVERYTHING.** Full-repository donor intake applies to admitted deep forks/acquired products, not automatically to ordinary package dependencies, provider SDKs, external services or reference implementations.
4. **REAL CLONE, REAL GIT BASELINE FOR DEEP FORKS.** Clone/fetch the complete donor at an exact revision, import the real tracked tree, commit it, and push that baseline to the Ops remote before architectural refactor.
5. **FULL DONOR INTAKE.** Census the complete tracked source/schema/UI/jobs/integrations/tests/docs/build/deploy/brand surface before decomposition.
6. **LEGACY MUST BURN DOWN.** Migrate callers/tests/data, then delete obsolete donor ownership. Do not leave old and new implementations alive indefinitely.
7. **SEARCH CHRONICA BY MEANING, NOT NAME.** Compare resource/state transition/effect/authority/evidence/failure semantics plus real code/tests.
8. **ONE MEANING, ONE INTENTIONAL INTERNAL SEMANTIC.** Donor/provider terms may survive at adapters/compatibility boundaries, not as duplicate internal primitives.
9. **PACKAGES MAY IMPROVE CHRONICA.** A richer reusable package semantic may be proven first in a Development Cell, but it must be proposed/admitted through the relevant Chronica owner, after which affected packages reconverge to the canonical result.
10. **CHRONICA REFERENCE MUST BE EXACT-SHA AND FRESH FOR ACTIVE WORK.** Major refactor/absorption waves verify owner/code/test references at the recorded SHA and refresh when current Chronica has materially moved.
11. **RIGHTS/OBLIGATIONS ARE EVIDENCE-BASED.** Acquisition/full-license/patent/trademark/source rights are bounded by actual agreements/assignments/licenses and surviving third-party obligations.
12. **WHITE-LABEL WHERE VERIFIED RIGHTS PERMIT.** Product identity converges on an approved `Chronica <Product>` name while required attribution/provenance survives.
13. **ONE PACKAGE LOGIC / MANY PORTS.** UI/API/CLI/MCP/workers call the same package/application semantics; ports do not fork business logic or authority. Shared canonical authority/execution still belongs to Chronica.
14. **EVERY DEVELOPMENT CELL HAS A PACKAGE CONTRACT + DOCS KERNEL.** Local docs may own package/provider/deployment/migration concerns but reference rather than redefine universal Chronica semantics.
15. **LOCAL DEV/TEST INDEPENDENCE IS REAL; CANONICAL SOVEREIGNTY IS NOT.** A Cell/package may boot local harnesses, fixtures, simulators and projection services for development/testing. Production canonical World/Identity/Authority/Execution/Evidence/Memory/history remains Chronica-composed.
16. **MCP IS A PORT, NOT A SECOND CORE.** It calls the same semantic/application owner and does not gain authority merely by tool exposure.
17. **FABRIC COMPATIBILITY IS SEMANTIC.** Transport reachability alone does not prove identity, authority, evidence, idempotency, uncertainty, reconciliation, disclosure or version compatibility.
18. **REPOSITORY REALITY != DEPLOYED REALITY.** Git HEAD, build artifact and staging/production runtime identity are separately evidenced.
19. **DOCS DO NOT AUTO-REWRITE FROM CODE.** Reality Mirror may trigger a Docs Sync Proposal; canonical docs change only through reviewed/admitted commits.
20. **NO PERMANENT DUAL IMPLEMENTATION OR DUAL SEMANTIC.** Temporary shims/aliases carry retirement criteria.
21. **LICENSE/IP/PROVENANCE SURVIVE REBRANDING AND ABSORPTION.** Clean branding never justifies erasing required evidence.

## Source admission law

Every external source records a mode and an admission disposition before production use.

Primary acquisition modes:

```text
DEEP_FORK
VENDORED_SOURCE
PACKAGE_DEPENDENCY
SUBMODULE
EXTERNAL_PROVIDER
REFERENCE_IMPLEMENTATION
SPEC_ONLY
REJECTED_SOURCE
```

Primary upstream lifecycle policies:

```text
PIN_AND_DIVERGE
TRACK_UPSTREAM
SECURITY_PATCH_ONLY
PROVIDER_MANAGED
ABSORB_AND_RETIRE
REFERENCE_ONLY
```

Admission does not grant canonical semantics or execution authority:

```text
SourceAdmission(code)
!= SemanticAdmission(meaning)
!= ExecutionAuthority(effect)
```

See `contracts/SOURCE-ADMISSION.md`.

## Real donor baseline law

For an admitted deep fork/acquired donor, the Ops repository records:

```text
DONOR_REPO_URL
DONOR_SHA_OR_TAG
CELL_BASELINE_COMMIT
CELL_BASELINE_TAG_OR_BRANCH
CELL_REMOTE_PUSHED=yes
BASELINE_BUILD_TEST_RESULT
```

A donor used only from a temporary checkout or reconstructed from memory/docs does not satisfy this gate.

This full baseline law is mode-specific: an ordinary package dependency, external provider, spec-only source or reference implementation is governed by source-admission evidence instead of being artificially imported as a donor monolith.

## Legacy + semantic burn-down law

Track at minimum:

```text
legacy_files_or_modules_remaining
legacy_public_callers_remaining
legacy_db_or_migration_ownership_remaining
legacy_ui_routes_remaining
legacy_background_jobs_remaining
legacy_policy_authority_paths_remaining
legacy_docs_brand_surfaces_remaining
duplicate_semantics_remaining
compatibility_aliases_remaining
```

Terminal target:

```text
legacy_code_remaining = 0
legacy_public_callers_remaining = 0
legacy_architecture_ownership_remaining = 0
unexplained_duplicate_semantics_remaining = 0
```

Git history, licenses, provenance, required attribution and approved boundary aliases are not implementation debt.

## Continuous semantic + conflict audit

Each substantial wave records:

```text
CHRONICA_REFERENCE_SHA
CHRONICA_OWNERS_READ
CHRONICA_CODE_PATHS_READ
CHRONICA_TEST_PATHS_READ
SEMANTIC_FINGERPRINTS
SEMANTIC_MATCHES
SEMANTIC_DISPOSITIONS
CONFLICTS_FOUND
CONFLICT_DECISIONS
```

Compare meaning using:

```text
resource/entity
inputs/outputs
preconditions/postconditions
state transition
effect
authority/normativity
evidence
idempotency/correlation
failure/UNKNOWN
risk
persistence/provider boundary
real callers/tests
```

Equivalent meanings converge by reuse/mapping/boundary aliasing. `NO_EQUIVALENT_FOUND` requires recorded owner/code search evidence.

## Semantic convergence law

```text
same meaning -> same internal canonical semantic
provider/donor name -> boundary alias only
richer reusable Ops meaning -> Chronica extension/adoption candidate
true product-specific meaning -> explicit product-local semantic
```

Ops can be more complete than Chronica during proving, but reusable improvements flow upward through Chronica's canonical owner rather than becoming a permanent parallel ontology.

See `contracts/SEMANTIC-CONVERGENCE.md`.

## Executable logistics law

Ops logistics is not a prose-only discipline. The reference implementation under `tools/ops-logistics/` provides:

```text
Reality Mirror
Semantic Convergence Checker
Absorption Planner
Docs Sync Proposal
Deployment/runtime evidence recorder + checker
Logistics scorecard
Fixture-based tests
```

Manifest binds these through its declared logistics contract version and evidence paths.

`node tools/ops-logistics/watch.mjs --ops /path/to/Ops` provides a live Reality Mirror projection under `.chronica/ops-reality.json`.

The independent CI lane is `.github/workflows/ops-logistics.yml`. A YAML file existing in Git is not proof; the workflow must actually start jobs and complete successfully before hosted-CI evidence is claimed.

See `contracts/LOGISTICS-KERNEL.md` and `contracts/OPS-MANIFEST.md`.

## Deployment / runtime evidence law

Repository code, built artifact and deployed runtime are distinct observations.

Each deployment evidence record includes:

```text
environment
exact gitSha
deploymentId
buildId
runtimeVersion
artifactDigest
healthStatus = PASS | FAIL | UNKNOWN
evidenceRefs
observedAt
```

A deployment on an older commit is reported as `OLDER_REPO_COMMIT`, never silently promoted to current reality.

Generated deployment evidence belongs under `.chronica/` (or another explicitly configured rebuildable runtime-evidence path), not canonical source truth.

## Legal/IP and white-label discipline

Use bounded states such as:

```text
IP_RIGHTS_VERIFIED_FOR_SCOPE
IP_RIGHTS_PARTIALLY_VERIFIED
THIRD_PARTY_OBLIGATIONS_REMAIN
IP_SCOPE_UNRESOLVED
```

Material donor identity surfaces are dispositioned as:

```text
REBRAND_TO_CHRONICA
RETAIN_FOR_REQUIRED_ATTRIBUTION
RETAIN_AS_PROVIDER_REFERENCE
RETAIN_AS_PROVENANCE_ONLY
REMOVE
```

## Mandatory Ops documentation kernel

```text
<ops-repo>/docs/
├── README.md
├── INDEX.md
├── TEMPLATE.md
├── architecture/
│   ├── README.md
│   ├── constitution/
│   ├── foundation/
│   ├── governance/
│   ├── integration/
│   └── operations/
├── blueprints/
├── decisions/
├── guides/
└── references/
```

The structure mirrors Chronica's discipline, but local Ops docs are subordinate to Chronica for shared universal semantics.

## One-world distinctions

Keep:

```text
ExternalCode != TrustedCode != CanonicalSemantics
SourceAdmission != SemanticAdmission != ExecutionAuthority
Can(...) != Authorized(...) != May(...) != Must(...)
ExecutionAdmission != Canonicalization
Provider Success != Reconciled Effect
Projection != Truth
UNKNOWN != SUCCESS
same semantic != same provider name
Architecture Intent != Repository Reality != Deployment Reality
```

## Lifecycle distinction

```text
OPS SOURCE REPO
  acquisition/refoundation/incubation source universe

OPS PRODUCT / DISTRIBUTION
  Chronica-branded product; may remain independently deployable

OPS MCP SERVER
  may remain independently operable

CHRONICA MONOREPO
  terminal native source owner after absorption
```

## Terminal placement

```text
pure/shared semantics           -> crates/core
durable coordination/runtime   -> crates/runtime
provider/protocol/I/O mechanics -> crates/adapter
thin UI/API/CLI/MCP packaging   -> apps
static semantics                -> graph
static implementation mapping   -> bindings
deployment/bootstrap            -> deploy
repo/build/release tooling      -> tools
```

Fabric is not a terminal placement category. Capability remains derived semantics, not a permanent `cap/` universe.

## Final principle

> **Classify and admit external code before trust. For admitted deep forks, clone and push the real donor baseline. Observe the real Ops repository continuously. Compare every material semantic against exact current Chronica code/tests by meaning, not name. Reuse/map/extend deliberately, migrate real callers, burn legacy and duplicate semantics down, generate deterministic absorption and docs-sync proposals, and link deployed runtime back to exact Git/build artifact evidence. Ops may improve Chronica, but external code never becomes trusted code, canonical semantics or execution authority merely by being imported.**
