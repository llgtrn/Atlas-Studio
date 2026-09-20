---
id: ops-production-donor-donor-census-provenance
type: reference
status: active
canonical: true
---
# Donor Census, Acquisition Evidence and Provenance

Every production Ops refactor starts with evidence about the donor that actually exists. For an acquired/deep-forked product, the census covers **the complete tracked donor repository, the rights relied upon for modification/white-labeling, the donor brand surface, and the documentation surface**.

## Required donor identity record

For each donor repository record:

```text
name
canonical repository URL
exact commit SHA/tag
clone/fetch date
repository root tree hash where available
submodules / LFS / generated-source boundaries
license identifier
license/notice paths
upstream copyright holders
build/runtime instructions
primary language/frameworks
DB/storage dependencies
external integrations
known enterprise/proprietary split
local modification summary
```

If code, model, dataset, plugin, theme, icon pack, font, bundled dependency, enterprise module, or generated artifact has separate licensing, record it separately.

## Acquisition / IP evidence record

When an Ops relies on purchased/acquired rights rather than only ordinary upstream OSS rights, record the evidence actually relied upon.

```text
transaction/acquisition reference
seller / licensor / assignor
buyer / licensee / assignee
assets/right categories covered
source-code ownership or license scope
copyright assignment/license scope
patent assignment/license identifiers + jurisdiction where applicable
trademark/brand assignment/license scope where applicable
white-label / modification / redistribution / sublicensing rights
territorial / field-of-use / duration limits if any
surviving third-party obligations
confidential/proprietary components with restricted handling
source escrow/delivery references if relevant
effective date
verification status
```

Use explicit status rather than prose optimism:

```text
IP_RIGHTS_VERIFIED_FOR_SCOPE
IP_RIGHTS_PARTIALLY_VERIFIED
THIRD_PARTY_OBLIGATIONS_REMAIN
IP_SCOPE_UNRESOLVED
```

A claim such as "the company/full license/patent was purchased" is an input requiring evidence classification. The repository must not treat it as a universal legal conclusion by itself.

## Required complete technical census

Inspect the complete tracked donor tree and enumerate:

```text
entrypoints
routes/endpoints
commands/jobs/workers
schema/migrations/seeds
core domain entities
state machines/statuses
permission/auth model
normative/policy models where real
external provider adapters
webhooks/events
background processing
search/index/cache layers
files/object storage
UI routes/workspaces/components
API/CLI/MCP surfaces
exports/imports
admin/diagnostic surfaces
observability
backup/recovery assumptions
configuration/secrets model
build/release/deployment files
CI/CD
scripts/generators
fixtures/demo data
tests
existing docs
```

Do not census only the files expected to survive. Full intake comes before KEEP/ADAPT/REPLACE/REMOVE/DEFER classification.

## Required product census

For each externally meaningful feature capture:

```text
feature/workflow
primary actor
input
state read
state written
effects/external calls
permissions
normative meaning where real
failure modes
evidence/receipts
UI surface
API/CLI surface
candidate MCP surface
Chronica mapping candidate
migration verdict: KEEP / ADAPT / REPLACE / REMOVE / DEFER
```

## Required brand / white-label census

For every public or operationally visible donor identity surface record:

```text
surface/path
current donor name/mark/logo/string
audience: public / operator / developer / provider / legal
replacement target
disposition:
  REBRAND_TO_CHRONICA
  RETAIN_FOR_REQUIRED_ATTRIBUTION
  RETAIN_AS_PROVIDER_REFERENCE
  RETAIN_AS_PROVENANCE_ONLY
  REMOVE
legal/provenance reason
verification evidence
```

Audit at minimum:

```text
repo/package/module/crate names
product/application name
logos/icons/favicons/assets
UI strings/page titles
email/notification templates
CLI/MCP descriptions
API/server banners and user-agent strings
OAuth app identity
bundle/service/container/image/chart names
domains/docs/support links
telemetry/analytics identifiers
seed/demo content/screenshots
copyright/license/patent/trademark notices
```

White-label completion is not measured by a global string replace. It is measured by explicit disposition of every material identity surface.

## Required documentation census

Donor documentation is part of product reality. Record:

```text
README / docs roots
architecture/design docs
API/MCP docs
operator/deployment guides
schema/data docs
security/privacy docs
license/legal notices
user help / screenshots
legacy/obsolete docs
```

Classify each as:

```text
MIGRATE_TO_OPS_DOCS_KERNEL
EXTRACT_FACTS_THEN_RETIRE
RETAIN_AS_REQUIRED_PROVENANCE
RETAIN_AS_PROVIDER_REFERENCE
RETIRE
```

The refounded Ops repository must then maintain its own Chronica-style documentation kernel rather than keeping donor docs as an uncontrolled second product identity.

## Why this matters

The census is the anti-hallucination and anti-loss control. It prevents an agent from replacing a mature donor with generic generated CRUD, accidentally dropping features, claiming rights that were not verified, or erasing legally required attribution during white-labeling.

## Baseline proof

Before major refactor, capture evidence that the donor baseline works where feasible:

```text
build result
test result
smoke workflow
schema boot/migration
UI boot
critical provider integration shape
documentation build if one exists
```

A broken/unbuildable donor may still be useful, but the limitation must be explicit.

## Provenance after assimilation

Even after donor files move or disappear, preserve enough lineage to explain:

```text
what behavior came from where
which upstream version was used
which acquisition/license/IP evidence authorized retained/modified/white-labeled use
which third-party obligations survived
which brand surfaces were changed and why
which local implementation replaced the donor path
which Chronica commit became canonical after terminal absorption
```

## No permanent census museum

The Ops repo should keep current provenance and migration evidence needed for active code. Massive obsolete audit dumps belong in Git history or CI artifacts, not as a second documentation universe.

## Gates

Any of these blocks a production assimilation claim in the relevant scope:

```text
UNKNOWN_SOURCE
NO_COMPLETE_DONOR_CENSUS
UNKNOWN_LICENSE
IP_SCOPE_UNRESOLVED where acquired rights are required
NO_BRAND_CENSUS for a white-label product
NO_DOCS_CENSUS
```

Required attribution/provenance must remain even after the donor product identity has been fully replaced by a Chronica-branded product identity.
