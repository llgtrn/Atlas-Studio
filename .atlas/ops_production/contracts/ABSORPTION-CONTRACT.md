# Ops Absorption Contract

This contract defines the terminal lifecycle of a mature acquired/donor-derived Ops repository.

An Ops source repository is a **temporary proving, refoundation and assimilation environment**. It may be fully deployable, commercially useful and independently operated during incubation, but terminal source ownership converges into Chronica.

The default lifecycle is:

```text
REAL DONOR / ACQUIRED PRODUCT
-> COMPLETE REPOSITORY INTAKE
-> LICENSE / ACQUISITION / IP / PROVENANCE ACCOUNTING
-> CHRONICA-BRANDED OPS REFOUNDATION
-> STANDALONE + MCP + CONNECTED PROOF
-> COMPLETE SOURCE + BRAND + DOCS INTAKE
-> CHRONICA-NATIVE ABSORPTION
-> CHRONICA-BRANDED STANDALONE ARTIFACTS REBUILT FROM CHRONICA
-> DUPLICATE SOURCE PATHS RETIRED
-> INDEPENDENT OPS SOURCE UNIVERSE RETIRED
```

## Critical distinctions

```text
OPS SOURCE REPOSITORY LIFECYCLE
!=
OPS PRODUCT / MCP DISTRIBUTION LIFECYCLE
```

and:

```text
WHITE-LABEL PRODUCT IDENTITY
!=
ERASURE OF REQUIRED LICENSE / COPYRIGHT / PATENT / TRADEMARK / PROVENANCE EVIDENCE
```

The source repository may be retired while standalone Chronica-branded product/UI/API/CLI/workers/MCP outputs continue indefinitely as build/deployment artifacts of the Chronica monorepo.

## Rights and provenance gate

When absorption relies on purchased/acquired rights, the source record must identify the evidence actually relied upon:

```text
acquisition / asset purchase reference
software/source-code ownership or license scope
copyright assignment/license scope
patent assignment/license scope + identifiers/jurisdiction where relevant
trademark/brand assignment/license scope where relevant
white-label/modification/redistribution/sublicensing rights
surviving third-party OSS/proprietary obligations
territorial/time/field-of-use limits if any
```

Use explicit status:

```text
IP_RIGHTS_VERIFIED_FOR_SCOPE
IP_RIGHTS_PARTIALLY_VERIFIED
THIRD_PARTY_OBLIGATIONS_REMAIN
IP_SCOPE_UNRESOLVED
```

No absorption report may state that a specific right authorizes a specific operation when that scope is unresolved.

## Canonical terminal architecture

The final Chronica tree remains responsibility-based:

```text
core             shared semantics/invariants
runtime          durability/authority/execution/evidence/recovery
adapter          provider/protocol/database/network/machine mechanics
organism         persistent cognition/world-model/learning/adaptation
apps             thin deployable/UI/API/CLI/MCP compositions
graph            static semantic definitions
bindings         static semantic-to-implementation declarations
deploy           deployment/runtime-environment material
tools            build/release/migration/developer tooling
docs             current controlled documentation
```

Absorption MUST NOT create permanent canonical source universes such as:

```text
BnbOps/
HelpdeskOps/
FinOps/
LegalOps/
ERP/
cap/
legal/
normative/
policy-engine/
organs/
donor-monolith/
```

merely to preserve the old product boundary.

A Chronica-branded product/distribution label may survive in thin app packaging and releases; duplicated business/normative/authority semantics may not.

## Fleet allocation gate

Before terminal absorption or deep donor ownership, verify the donor is present in the master corpus and that Fleet allocation names exactly one primary absorber.

```text
DONOR_IN_MASTER_CORPUS
PRIMARY_ABSORBER_EXACTLY_ONE
MULTIPLE_PRIMARY_ABSORBERS = 0
SKIPPED_DONORS = 0
UNALLOCATED_DONORS = 0
```

A donor discovered in an Ops but absent from the master corpus is an accounting failure until added with provenance. Do not keep it as an invisible local exception.

## Mirror-to-absorption handoff

Terminal absorption does not begin from an unscoped moving target.

Freeze:

```text
SOURCE_OPS_REPO
SOURCE_OPS_SHA
SOURCE_OPS_MIRROR_REPORT / reproducible inputs
TARGET_CHRONICA_REPO
TARGET_CHRONICA_SHA
DONOR BASELINES / PROVENANCE relied upon
```

The last Ops Mirror comparison is evidence of what the proving repo believed at its pinned Chronica reference. Terminal absorption must still re-audit against the **current target Chronica SHA** because Chronica may have advanced since the Ops slice was proven.

The Ops worker itself does not directly promote local semantics to canonical Chronica truth. Absorption is a separate target-Chronica integration operation.

## Complete intake before decomposition

Absorption begins from the exact Ops source SHA and must account for the **whole tracked source tree**.

Inventory:

```text
code/packages/crates/modules
schema/migrations/seeds
routes/controllers
commands/queries
workers/jobs
provider adapters
webhooks/events/outbox/inbox
UI/API/CLI/MCP surfaces
auth/policy
normative source/rule/procedure paths where real
storage/search/cache
configuration/secrets
build/release/deployment/CI
tests/fixtures
documentation
brand/product identity surfaces
license/acquisition/IP/provenance evidence
```

Do not selectively copy only files that look important before the full tree has been classified.

Temporary worktrees/intake areas are allowed for mechanical analysis, but no temporary donor universe survives terminally.

## Documentation parity is part of absorption

The source Ops must have a controlled Chronica-style docs kernel before terminal absorption:

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

During absorption, product-local semantics/decisions/guides/provenance are either:

```text
MIGRATED TO RELEVANT CHRONICA DOC OWNER/REFERENCE
RETAINED IN THIN PRODUCT/DISTRIBUTION DOCS
RETAINED AS REQUIRED PROVENANCE
RETIRED AFTER CONTENT IS ACCOUNTED FOR
```

The donor or Ops docs tree must not survive as an uncontrolled parallel architecture.

## Semantic fidelity law

Source absorption preserves meaning, not filenames or endpoint shape.

Use Chronica's one-world aspects where applicable:

```text
DESCRIPTIVE  what is / was
NORMATIVE    what may / must / must not / is empowered to happen
COGNITIVE    what is inferred / interpreted / predicted / recommended
ACTIVE       what is intended / execution-admitted / executed / reconciled
```

Keep:

```text
Fact != Norm
Interpretation != Source Text
Can(...) != Authorized(...) != May(...) != Must(...)
ExecutionAdmission != Canonicalization
Provider Success != Reconciled Effect
UNKNOWN != SUCCESS
```

Capability remains derived semantics; absorption MUST NOT recreate a permanent Capability registry/store/layer.

## White-label parity law

Before terminal absorption, every material donor identity surface must have one disposition:

```text
REBRAND_TO_CHRONICA
RETAIN_FOR_REQUIRED_ATTRIBUTION
RETAIN_AS_PROVIDER_REFERENCE
RETAIN_AS_PROVENANCE_ONLY
REMOVE
```

Audit at minimum:

```text
repo/package/crate/module names
product/app names
logos/icons/favicons/assets
UI strings/page titles
emails/notifications
CLI/MCP descriptions
API/server banners
OAuth/bundle/service/container/image names
domains/docs/support links
telemetry/analytics identifiers
screenshots/seeds/demo content
legal notices
```

A donor name remaining because it identifies an external provider or required attribution is acceptable. A donor name remaining because the white-label/refactor missed it is not.

## Native placement law

Every retained responsibility converges by meaning:

```text
pure universal semantics/invariants
-> core

durable coordination/admission/authority/execution/evidence/recovery
-> runtime

vendor/provider/protocol/storage/network/browser/machine mechanics
-> adapter

thin API/CLI/UI/MCP/server packaging
-> apps

static semantics
-> graph

static implementation declarations
-> bindings

deployment/bootstrap
-> ops

repo/build/release/migration tooling
-> tools
```

External donor/provider mechanics intentionally remaining outside Chronica are classified as external providers/adapters, not a second world.

## Strategic/foundational technology conservation gate

For donor technology classified `STRATEGIC` or `FOUNDATIONAL`, this Ops/source absorption contract is subordinate to the canonical Native Technology Strategy.

```text
CURRENT SLICE COMPLETE
!=
TECHNOLOGY ABSORBED
!=
SOURCE EXTINCTION AUTHORITY
```

A source path MUST NOT be retired merely because a bounded Chronica slice exists, compiles, has migrated one caller, or passes parity for that slice. The technology census, lineage, dependency graph, native reconstruction ladder, proof ladder, and extinction eligibility must remain explicit.

For promoted foundational technology, the normal terminal path is:

```text
DONOR_TECHNOLOGY
-> CENSUSED
-> PRESERVED_LINEAGE
-> NATIVE_CANDIDATE
-> NATIVE_PROTOTYPE
-> NATIVE_PARITY
-> NATIVE_PRIMARY
-> FULLY_NATIVE
-> DONOR_EXTINCT
```

If source was deleted before the applicable gate was justified and the pinned upstream remains recoverable, treat that deletion as historical evidence rather than completion authority and rehydrate the exact pinned source before continuing a census that would otherwise require rediscovery.

See `docs/architecture/constitution/NATIVE-TECHNOLOGY-STRATEGY.md` and `tools/system-atlas/native-technology-promotion.yaml`.

## Source absorption is not a blind copy

Absorption has two operations:

```text
1. COMPLETE INTAKE
   account for all source/behavior/docs/brand/provenance

2. SEMANTIC DISSOLUTION
   move retained responsibilities into canonical owners without losing meaning
```

A bulk copied donor tree under `products/<ops>`, `organs/<ops>`, `cap/<ops>`, `legal/`, `normative/`, or a giant untouched compatibility module is not terminal absorption.

Deleting source and rewriting from memory is also not valid absorption unless every behavior is intentionally replaced and parity is proven.

## Standalone distribution after absorption

Chronica must be able to produce standalone artifacts for any Ops whose contract promised independent operation.

Illustrative artifact names:

```text
chronica-helpdesk-server
chronica-helpdesk-mcp
chronica-hospitality-server
chronica-finance-cli
```

Exact product naming is a human decision. The invariant is that terminal public identity belongs to Chronica rather than accidentally reverting to the donor product.

Standalone artifacts may embed required Chronica libraries/runtime components in-process and may use local auth/persistence/provider adapters where promised by contract.

## MCP after absorption

The MCP server is a surface, not a temporary bridge.

Preserve unless deliberately versioned otherwise:

```text
tool/resource names or approved renamed aliases
schemas
risk classification
error/status vocabulary
idempotency semantics
evidence behavior
local authority behavior
normative status behavior where exposed
transport behavior
```

If public MCP naming changes due to white-labeling, document/version aliases or breaking changes deliberately rather than silently losing compatibility.

## 100% compatibility definition

For every inventoried public workflow, terminal state is exactly one of:

```text
PARITY
INTENTIONALLY VERSIONED CHANGE
EXPLICIT HUMAN-APPROVED RETIREMENT
```

For every material donor identity surface:

```text
CHRONICA-BRANDED
REQUIRED ATTRIBUTION
PROVIDER REFERENCE
PROVENANCE ONLY
REMOVED
```

For every maintained source documentation responsibility:

```text
MIGRATED
THIN PRODUCT DOC RETAINED
PROVENANCE RETAINED
RETIRED AFTER ACCOUNTING
```

No unexplained gaps.

## Conflict prevention

Before moving code, compare source Ops and target Chronica for:

```text
duplicate domain/state types
duplicate authority/policy layers
duplicate normative models
duplicate workflow/execution engines
duplicate event/evidence formats
duplicate memory/context models
duplicate Capability registry/layer concepts
duplicate persistence/migrations
duplicate provider adapters
name/package/crate collisions
route/API/MCP collisions
contract-version conflicts
license/IP conflicts
brand/trademark conflicts
documentation-owner conflicts
```

Prefer reuse/merge/adapt/temporary shim/retire. Never preserve two permanent implementations merely because both work.

## Provenance

Absorption preserves lineage sufficient to answer:

```text
where behavior/code originated
which donor commit was used
which Ops source commit was absorbed
which acquisition/license/IP evidence was relied upon
which third-party obligations remain
which brand surfaces were changed/retained and why
which Chronica commit became canonical
which source/docs paths were replaced or retired
which behavior was intentionally versioned
```

## Retirement gate

An independent Ops source repo is retirement-ready only when:

```text
complete donor/source census exists
rights/obligations are sufficiently verified for the retained scope
public-surface parity is 100% accounted for
brand/white-label census is 100% dispositioned
docs responsibilities are 100% migrated/retained/retired intentionally
native Chronica owners hold retained behavior
all real callers have migrated
Chronica-branded standalone artifacts build
standalone MCP parity is proven
connected parity is proven where required
migrations/recovery are proven
provider adapters are native or intentionally external
provenance is retained
no unexplained duplicate source path remains
```

After retirement, canonical feature development happens in Chronica rather than both source universes.

## Evidence states

Use these honestly:

```text
ABSORPTION_READY
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
OPS_SOURCE_UNIVERSE_RETIRED
```

## Final invariant

> **Clone and account for the complete acquired/donor product, including code, docs, brand and legal/IP provenance. Refound it as a Chronica-branded Ops, prove real behavior, then dissolve mature responsibilities into canonical Chronica owners. Keep independently deployable Chronica-branded distributions, preserve required third-party obligations and provenance, and retire duplicate source universes.**
