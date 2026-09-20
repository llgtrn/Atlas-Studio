---
id: ops-production-absorb-into-chronica-goal
type: reference
status: active
canonical: true
---
# Converge Development Cell / Package Into Chronica Goal

This compatibility path is the terminal convergence prompt for a mature donor-derived Development Cell/package.

Use it only after the Cell/package has complete donor/source accounting, real implementation/tests, package contract + Mirror evidence, a substantially complete compatibility matrix, and controlled local docs/provenance.

The purpose is not to preserve a standalone Ops system. The purpose is to **preserve proven behavior/provenance, dissolve duplicate canonical ownership, place mature implementation by Chronica responsibility, and keep only justified package-local adapters/projections/provider mechanics**.

# GOAL

Given:

```text
SOURCE_CELL_REPO + exact source SHA
TARGET_CHRONICA_REPO + exact target branch/SHA
```

produce a verified absorption in which:

```text
100% of tracked source responsibilities are accounted for
100% of inventoried public behavior is accounted for
100% of material donor identity surfaces are dispositioned
100% of maintained Cell/package docs responsibilities are dispositioned
verified license/acquisition/IP provenance survives
reusable implementation is preserved rather than reimagined
stable semantics converge into existing Chronica owners
Capability remains derived rather than a permanent registry/layer
provider/vendor mechanics converge into adapters
UI/API/CLI/MCP remain thin surfaces over the same native logic
Chronica-branded standalone artifacts remain buildable
connected mode uses canonical Chronica authority/execution/evidence
no permanent Cell/package/domain/cap/fabric/legal/normative/policy-engine canonical universe survives
legacy Cell repository may remain as a thin package/distribution boundary or be retired after proof; it must not retain duplicate canonical ownership
```

# 1. LOAD THE CONTRACT YOURSELF

Read:

```text
docs/architecture/constitution/NORTH-STAR.md
docs/architecture/foundation/system-model.md
docs/architecture/foundation/capability-resolution.md
docs/architecture/governance/authority.md
docs/architecture/governance/execution.md
docs/architecture/governance/evidence.md
docs/architecture/operations/fabric.md
AGENTS.md
docs/ops_production/README.md
docs/ops_production/PRODUCTION-NORTH-STAR.md
docs/ops_production/contracts/ABSORPTION-CONTRACT.md
docs/ops_production/donor/DONOR-CENSUS-PROVENANCE.md
docs/ops_production/engineering/REPO-STRUCTURE.md
```

Load `docs/architecture/governance/normative.md` when the source contains real normative behavior. Load other canonical owners only as needed.

# 2. FREEZE SOURCE AND TARGET REALITY

Record:

```text
SOURCE_REPO
SOURCE_BRANCH
SOURCE_HEAD
SOURCE_STATUS
SOURCE_REMOTE

TARGET_REPO
TARGET_BRANCH
TARGET_HEAD
TARGET_STATUS
TARGET_REMOTE
```

Also record:

```text
last reproducible Cell Mirror/package report / inputs when available
Chronica reference SHA used by the final package proving slice
donor repository + exact SHA/tag
complete donor/source census status
license/notice paths
acquisition/license/IP evidence status
white-label census status
Cell/package docs-kernel status
manifest/protocol/semantic/MCP/event/integration/UI versions
```

Never absorb against stale Chronica state. Never reset or force-push away concurrent target work.

# 2A. PACKAGE CONTRACT AT SOURCE HEAD

Freeze and verify:

```text
DEVELOPMENT_CELL_ID
PACKAGE_ID
PACKAGE_KIND
canonical_runtime = CHRONICA_REQUIRED
standalone_sovereignty = false
```

Terminal convergence does not require deleting all package-local source. It requires eliminating duplicate **canonical** ownership. Domain-specific projections, adapters, UI/API surfaces, provider mechanics and deployment wrappers may remain when they still have a real package responsibility.

# 3. COMPLETE SOURCE INTAKE — DO NOT SAMPLE

Inspect the **entire tracked source tree at SOURCE_HEAD**.

Account for:

```text
source files / generated boundaries
packages/crates/modules
schema/migrations/seeds
routes/controllers/commands/queries
workers/jobs/schedulers
state machines/invariants
persistence/repositories
providers/adapters
webhooks/events/outbox/inbox
UI/API/CLI/MCP surfaces
identity/auth/policy
normative sources/procedures where present
Capability-derived behavior / Can(...)
idempotency/correlation/evidence
unknown-outcome/reconciliation
context/disclosure/provider boundaries
configuration/secrets
build/release/deployment/CI
tests/fixtures
documentation
branding/product identity surfaces
licenses/notices/acquisition/IP/provenance
```

Do not selectively copy only "important" files first and call it full absorption.

# 4. VERIFY RIGHTS / OBLIGATIONS USED BY THE TARGET STATE

For any retained proprietary/acquired/white-labeled surface, record the evidence actually relied upon:

```text
software/source-code right
copyright right
patent right where applicable
trademark/brand right where applicable
modification/redistribution/white-label right
territorial/time/field-of-use limits
surviving third-party obligations
```

Use:

```text
IP_RIGHTS_VERIFIED_FOR_SCOPE
IP_RIGHTS_PARTIALLY_VERIFIED
THIRD_PARTY_OBLIGATIONS_REMAIN
IP_SCOPE_UNRESOLVED
```

`IP_SCOPE_UNRESOLVED` blocks an unsupported legal-completion claim.

# 5. BUILD THE ABSORPTION MAP

Every retained source responsibility receives exactly one terminal class:

```text
CORE
RUNTIME
ADAPTER
APP_SURFACE
GRAPH
BINDING
OPS_DEPLOYMENT
TOOLING
PROVENANCE
EXTERNAL_PROVIDER
RETIRE
```

Placement:

```text
pure/shared semantics           -> core
durable authority/execution/evidence/recovery -> runtime
provider/protocol/storage/network mechanics   -> adapter
persistent cognition/world-model/learning/adaptation -> organism
thin UI/API/CLI/MCP/server compositions       -> apps
static semantics                              -> graph
static semantic-to-implementation declarations -> bindings
deployment/bootstrap                         -> deploy
build/release/migration/developer tooling     -> tools
```

Forbidden terminal classes:

```text
PERMANENT_OPS_SUBSYSTEM
PERMANENT_CAPABILITY_REGISTRY
PERMANENT_FABRIC_SUBSYSTEM
PERMANENT_LEGAL/NORMATIVE/POLICY UNIVERSE
PERMANENT_DONOR_MONOLITH
```

# 6. BUILD THE WHITE-LABEL MAP

Every material donor identity surface receives one disposition:

```text
REBRAND_TO_CHRONICA
RETAIN_FOR_REQUIRED_ATTRIBUTION
RETAIN_AS_PROVIDER_REFERENCE
RETAIN_AS_PROVENANCE_ONLY
REMOVE
```

Audit:

```text
repository/package/module/crate names
product/app names
logos/icons/favicons
UI strings/page titles
emails/notifications
CLI/MCP descriptions
API/server banners
OAuth/bundle/service/container/image names
domains/docs/support links
telemetry identifiers
screenshots/seeds/demo content
legal notices
```

A missed donor brand surface is a white-label gap. A required attribution retained deliberately is not.

# 7. BUILD THE DOCUMENTATION ABSORPTION MAP

The source Ops docs kernel must be complete enough to understand the product before absorption:

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

Every maintained documentation responsibility receives one disposition:

```text
MIGRATE_TO_CHRONICA_CANONICAL_DOC
RETAIN_AS_THIN_PRODUCT/DISTRIBUTION_DOC
RETAIN_AS_PROVENANCE/REFERENCE
RETIRE_AFTER_ACCOUNTING
```

Do not preserve the donor/Ops docs tree as a second uncontrolled Chronica architecture.

# 8. CONFLICT AUDIT BEFORE MOVEMENT

Detect collisions with current Chronica:

```text
duplicate entity/state types
duplicate stable identity/correlation systems
duplicate Capability registries/layers
duplicate authority/policy/normative systems
duplicate execution/workflow engines
duplicate evidence/event models
duplicate memory/context models
duplicate persistence/migrations
duplicate provider adapters
name/package/crate collisions
route/API/MCP collisions
contract-version conflicts
license/IP conflicts
brand/trademark conflicts
documentation-owner conflicts
```

For each choose deliberately:

```text
REUSE CHRONICA OWNER
MERGE INTO CHRONICA OWNER
ADAPT SOURCE CALLERS
KEEP SOURCE MECHANICS AS ADAPTER
TEMPORARY COMPATIBILITY SHIM
RETIRE SOURCE PATH
```

No permanent dual implementation.

# 9. ABSORB VERTICAL BY VERTICAL

For each real workflow:

```text
donor-derived behavior
-> identify native semantic owner
-> preserve identity/correlation/idempotency
-> preserve source/validity/procedure where normative
-> preserve binding/provider boundary
-> preserve derived Can(...) semantics
-> move/refactor reusable implementation
-> migrate UI/API/CLI/MCP/worker callers
-> preserve evidence/unknown-outcome/reconciliation
-> differential-test behavior + semantics + Fabric compatibility
-> delete duplicate source path after caller convergence
```

A file move without changed ownership is not absorption. A rewrite from memory is not absorption.

# 10. PRESERVE CHRONICA-BRANDED STANDALONE PRODUCTS

After absorption, Chronica must build the promised standalone distributions, illustratively:

```text
chronica-helpdesk-server
chronica-helpdesk-mcp
chronica-hospitality-server
chronica-finance-cli
```

Exact names are human product decisions. Public identity must not silently revert to donor branding.

Standalone products may compose native Chronica libraries/runtime in-process and use local auth/persistence/provider adapters where the compatibility contract requires independent operation.

# 11. 100% COMPATIBILITY GATE

For each public workflow prove exactly one:

```text
PARITY
INTENTIONALLY VERSIONED CHANGE
EXPLICIT HUMAN-APPROVED RETIREMENT
```

For each material brand surface prove exactly one:

```text
CHRONICA-BRANDED
REQUIRED ATTRIBUTION
PROVIDER REFERENCE
PROVENANCE ONLY
REMOVED
```

For each maintained documentation responsibility prove exactly one:

```text
MIGRATED
THIN PRODUCT DOC RETAINED
PROVENANCE RETAINED
RETIRED AFTER ACCOUNTING
```

Compatibility includes where relevant:

```text
UI behavior
API/CLI/MCP contract
schema/migrations
provider behavior
state transitions
Can(...) semantics
authority/normativity
idempotency/retries
evidence/audit
unknown outcome/reconciliation
offline/recovery
backup/restore
upgrade path
```

# 12. RETIRE SOURCE UNIVERSE ONLY AFTER PROOF

Retirement requires:

```text
complete source census
verified rights/obligations sufficient for retained scope
100% public-workflow accounting
100% brand-surface disposition
100% docs-responsibility disposition
native Chronica ownership of retained behavior
all real callers migrated
Chronica-branded standalone artifacts build
standalone MCP parity proven
connected parity proven where required
recovery/migrations proven
provenance retained
no unexplained duplicate source path
```

Then archive/mirror/read-only/retire the independent Ops source according to repository policy. Do not continue canonical feature development in both repositories.

# 13. REQUIRED FINAL EVIDENCE STATES

```text
ABSORPTION_SOURCE_FROZEN
ABSORPTION_CENSUS_COMPLETE
IP_RIGHTS_VERIFIED_FOR_SCOPE
WHITE_LABEL_CENSUS_COMPLETE
OPS_DOCS_KERNEL_COMPLETE
ABSORPTION_MAP_COMPLETE
CHRONICA_NATIVE_ABSORPTION_PROVEN
STANDALONE_ARTIFACT_PARITY_PROVEN
MCP_STANDALONE_PARITY_PROVEN
CONNECTED_PARITY_PROVEN
SEMANTIC_PARITY_PROVEN
PROVENANCE_PRESERVED
DUPLICATE_SOURCE_PATHS_RETIRED
OPS_SOURCE_UNIVERSE_RETIREMENT_READY
```

# 14. FINAL REPORT

```text
SOURCE_CELL_REPO:
SOURCE_HEAD:
TARGET_CHRONICA_HEAD_BEFORE:
TARGET_CHRONICA_HEAD_AFTER:
DONOR_REPO_AND_SHA:

IP_RIGHTS_STATUS:
FULL_SOURCE_CENSUS:
WHITE_LABEL_CENSUS:
OPS_DOCS_KERNEL:

PUBLIC_WORKFLOWS_TOTAL:
PUBLIC_WORKFLOWS_PARITY:
INTENTIONAL_VERSIONED_CHANGES:
UNEXPLAINED_GAPS:

BRAND_SURFACES_TOTAL:
BRAND_SURFACES_CHRONICA:
REQUIRED_ATTRIBUTION_RETAINED:
BRAND_GAPS:

DOCS_RESPONSIBILITIES_MIGRATED:
DOCS_RETAINED_THIN:
DOCS_PROVENANCE_RETAINED:
DOCS_GAPS:

PLACEMENT_SUMMARY:
CORE:
RUNTIME:
ADAPTER:
APP_SURFACE:
GRAPH_BINDING:
OPS_TOOLING_PROVENANCE:
RETIRED:

STANDALONE_ARTIFACTS:
MCP_PARITY:
CONNECTED_PARITY:
TESTS:

REMAINING_BLOCKERS:
SOURCE_REPO_RETIREMENT_READY: yes/no

COMMITS:
- ...
```

# FINAL RULE

**INGEST THE COMPLETE REAL OPS -> VERIFY RIGHTS/OBLIGATIONS -> ACCOUNT FOR SOURCE + BEHAVIOR + BRAND + DOCS -> ABSORB INTO EXISTING CHRONICA OWNERS -> PRESERVE CHRONICA-BRANDED STANDALONE/MCP OUTPUTS -> DIFFERENTIAL-TEST COMPATIBILITY -> RETAIN REQUIRED PROVENANCE -> DELETE DUPLICATES -> RETIRE THE TEMPORARY OPS SOURCE UNIVERSE.**
