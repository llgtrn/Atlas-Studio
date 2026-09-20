---
id: ops-production-donor-oss-donor-clone-refactor
type: reference
status: active
canonical: true
---
# Donor Full-Repository Acquisition and Refactor Contract

Production Ops are donor-first and **full-repository-intake first**. The default path is not to recreate a product from memory, cherry-pick a few donor files, wrap a remote API forever, or keep donor architecture alive beside a new Chronica-shaped layer.

The target is:

```text
real OSS / acquired donor repository at exact SHA/tag
-> complete clone/fetch of tracked source tree
-> donor baseline pushed into the Ops Git repository
-> legal/IP/provenance verification
-> reproducible donor baseline
-> complete technical + product + documentation + brand census
-> Chronica conflict audit at a recorded Chronica SHA
-> incremental vertical refactor
-> caller/test migration
-> donor legacy burn-down
-> zero donor-owned legacy implementation
-> standalone + MCP + connected proof
-> eventual Chronica-native source absorption
```

## 1. Mandatory first step: clone the whole real repository into the Ops repo

Before substantial product implementation, the agent must:

```text
1. identify the real donor repository or repositories;
2. clone/fetch the complete repository, not a hand-picked subset;
3. pin exact commit SHA/tag;
4. import that real donor tree into the Ops repository/worktree;
5. push a baseline commit/tag/branch to the Ops Git remote before architectural refactor;
6. record repository history boundaries and submodules/LFS/generated assets where relevant;
7. verify the applicable license/IP/acquisition evidence;
8. build/run/test the donor baseline when feasible;
9. inspect the complete tracked tree: schema/routes/services/jobs/UI/integrations/config/build/deploy/tests/docs;
10. produce a complete donor capability/code/product/docs/brand census;
11. record the current Chronica reference SHA and run a conflict audit;
12. establish the target Chronica-branded product identity;
13. only then begin refactor and white-label convergence.
```

A temporary checkout outside the Ops repository is useful for comparison, but it does **not** satisfy this contract by itself. The donor baseline must become auditable Ops Git history.

The expected evidence is at least:

```text
DONOR_REPO_URL
DONOR_SHA_OR_TAG
OPS_BASELINE_COMMIT
OPS_BASELINE_TAG_OR_BRANCH
OPS_REMOTE_PUSHED=yes
BASELINE_BUILD_TEST_RESULT
CHRONICA_REFERENCE_SHA
```

No "I know how Chatwoot/ERPNext/X works, so I will recreate it" workflow is acceptable.

No "copy only the interesting modules" workflow is acceptable for a donor selected for deep assimilation.

## 2. Acquisition / license / patent / trademark gate

A human may state that the donor company, software rights, full license, patents, trademarks, source code, or other IP have been acquired. Chronica documentation must not convert that statement into an unsupported legal fact.

Record the evidence that actually grants the rights being relied upon, for example:

```text
acquisition / asset-purchase agreement reference
license agreement reference + scope
source-code ownership / assignment reference
patent assignment or license reference + jurisdiction / identifier where applicable
trademark / brand assignment or license reference where applicable
copyright assignment / license reference where applicable
redistribution / modification / sublicensing / white-label rights
surviving third-party OSS/proprietary obligations
territorial / temporal / field-of-use limitations if any
confidentiality / source-availability restrictions if any
```

Use evidence states such as:

```text
IP_RIGHTS_VERIFIED_FOR_SCOPE
IP_RIGHTS_PARTIALLY_VERIFIED
THIRD_PARTY_OBLIGATIONS_REMAIN
IP_SCOPE_UNRESOLVED
```

`IP_SCOPE_UNRESOLVED` blocks claims that a specific patent/trademark/license right authorizes a specific action. It does not block preservation of ordinary OSS rights already independently verified.

## 3. Real Git baseline is immutable migration evidence

The baseline import is not a throwaway staging copy. It is migration evidence.

Preferred pattern:

```text
upstream donor clone @ exact SHA
        |
        v
Ops Git baseline commit/tag
        |
        +--> baseline build/tests
        +--> complete source/schema/UI/docs census
        +--> IP/license/provenance record
        +--> brand/asset/string/domain/package census
        +--> Chronica conflict audit @ exact Chronica SHA
        |
        v
incremental refactor commits
```

After the baseline is pushed, do not rewrite history merely to make the donor origin disappear. Use ordinary forward commits so the repository shows exactly how donor reality was transformed.

## 4. Legacy burn-down is the default refactor strategy

The Ops repo starts with real donor legacy because the real donor was cloned. That is expected.

The migration strategy is **not** to create a permanent new architecture beside the donor and leave both alive. It is to shrink the donor-owned implementation continuously.

For each vertical:

```text
read donor implementation
-> compare with current Chronica semantics/runtime
-> identify conflict/duplication
-> select target Ops responsibility
-> refactor/move implementation
-> migrate real callers
-> migrate tests/data/migrations as needed
-> verify standalone/MCP/connected behavior
-> delete obsolete donor path
-> record legacy surface reduction
```

Each coding loop should leave the repository in one of two states:

```text
LEGACY_REDUCED
or
LEGACY_NOT_REDUCED_WITH_EVIDENCED_BLOCKER
```

A loop that adds a replacement path but leaves the old path as an indefinite second implementation is architectural debt, not progress.

Track at minimum:

```text
legacy_files_or_modules_remaining
legacy_public_callers_remaining
legacy_db_or_migration_ownership_remaining
legacy_ui_routes_remaining
legacy_background_jobs_remaining
legacy_policy_authority_paths_remaining
legacy_docs_brand_surfaces_remaining
```

The terminal target is:

```text
legacy_code_remaining = 0
legacy_public_callers_remaining = 0
legacy_architecture_ownership_remaining = 0
```

This does **not** mean deleting required licenses, provenance, acquisition references, compatibility aliases intentionally retained, historical Git commits, or external-provider names.

## 5. Chronica conflict audit is mandatory during coding

Before introducing or preserving any major semantic/runtime abstraction, inspect Chronica at a recorded SHA.

At minimum compare against relevant canonical owners for:

```text
identity/resource/state representation
Capability Resolution / Can(...)
relations/bindings
normativity / May-Must-Forbidden
authority / Authorized(...)
ExecutionAdmission / WorkRun
events/evidence/canonicalization
memory/context/sovereignty
safety/machine semantics where relevant
recovery/reconciliation/idempotency
Fabric boundary semantics
runtime/store/adapter responsibility
crate/package naming and existing implementation owners
```

Conflict outcomes must be explicit:

```text
REUSE_CHRONICA_SEMANTICS
MAP_TO_CHRONICA_OWNER
MERGE_WITH_EXISTING_CHRONICA_OWNER
KEEP_PRODUCT_LOCAL
KEEP_AS_PROVIDER_ADAPTER
TEMPORARY_COMPATIBILITY_SHIM
RETIRE_DONOR_PATH
DEFER_WITH_EVIDENCE
```

Do not create a new local registry/store/policy engine/workflow engine merely because the donor already had one. First prove it is not duplicating an existing Chronica responsibility.

Every meaningful coding report should include:

```text
CHRONICA_REFERENCE_SHA
CHRONICA_OWNERS_READ
CONFLICTS_FOUND
CONFLICT_DECISIONS
```

## 6. Full-tree refactor, not permanent wrapping

The target is not:

```text
new Ops shell
-> giant untouched donor subsystem forever
```

Nor:

```text
Chronica adapter
-> donor REST API forever
```

unless the donor is explicitly classified as an intentionally external provider rather than an acquired/deep-forked product.

For an acquired/deep-forked Ops product, refactor the complete retained product into native ownership:

```text
donor route/controller/service/model/UI/job
-> identify real semantic responsibility
-> isolate provider/vendor mechanics
-> converge shared business semantics into one Ops application/domain path
-> map Chronica World / Capability Resolution / Authority / Execution / Evidence semantics
-> migrate UI/API/CLI/MCP/workers
-> preserve migrations/recovery/evidence
-> delete obsolete duplicate path after proof
```

The goal is **behavioral continuity with architectural ownership changed**, not line-for-line rewriting for appearance.

## 7. Mandatory white-label / brand refoundation

For an Ops product whose applicable acquisition/license evidence permits white-labeling, donor branding must not remain the public product identity by default.

The target identity is a **Chronica product projection**, using a product name approved under Chronica naming, for example:

```text
Chronica <Product>
Chronica Helpdesk
Chronica Commerce
Chronica Hospitality
Chronica Finance
```

Actual product names remain a human naming decision; the invariant is that the donor brand is not accidentally preserved as the canonical product identity.

Audit and deliberately classify every donor-visible brand surface:

```text
repository/package/crate/module names
application/product name
logos/icons/favicons/images
UI strings / page titles / emails / notifications
API user-agent / server banners
CLI/MCP command descriptions
OAuth app names / callback labels
mobile/desktop bundle identifiers where present
domains / URLs / docs links / support links
Docker image / chart / service names
telemetry/analytics product identifiers
sample data / seed data / screenshots
copyright / legal notices
README / docs / comments that assert product identity
```

Disposition:

```text
REBRAND_TO_CHRONICA
RETAIN_FOR_REQUIRED_ATTRIBUTION
RETAIN_AS_PROVIDER_REFERENCE
RETAIN_AS_PROVENANCE_ONLY
REMOVE
```

Do **not** remove legally required attribution, notices, copyright statements, patent notices, or third-party license text merely because the public product is white-labeled.

White-labeling means product identity converges; it does not mean provenance is erased.

## 8. Preserve behavior deliberately

For each donor behavior decide:

```text
KEEP
ADAPT
REPLACE
REMOVE
DEFER
```

For each donor brand/legal artifact decide separately using the white-label dispositions above.

Do not silently lose features during architectural cleanup or silently erase obligations during rebranding.

## 9. Donor UI

Donor UI is implementation material, not permanent product identity.

A deep-forked acquired Ops UI should converge on:

```text
Chronica product identity
the Ops product's own design system/docs
shared application services
Chronica frontend projection rules where connected
no donor-only business logic hidden in components
no donor branding except required attribution/provider references
```

Rename-only rebranding is insufficient if routes, UX, state ownership, business logic, docs, packages, deployment names, and public metadata still encode the donor product as the architectural owner.

## 10. Donor database

Do not blindly preserve donor schema as permanent domain ontology. First understand transactional invariants and migrations. Refactor only with migration/recovery evidence.

Full-repository intake includes all migrations, seeds, fixtures and operational DB tooling even if only a subset survives terminal convergence.

## 11. External integrations

Existing donor integrations are valuable implementation evidence. Preserve protocol/provider mechanics behind adapters instead of rebuilding them from memory.

Provider brand names may remain where they identify the external provider rather than the Ops product itself.

## 12. Documentation must be refounded too

The donor's documentation is part of the complete intake and must be censused before deletion.

The refounded Ops repository must create and maintain a **Chronica-style documentation kernel** described by `docs/ops_production/engineering/REPO-STRUCTURE.md`:

```text
docs/INDEX.md
docs/TEMPLATE.md
docs/architecture/
docs/blueprints/
docs/decisions/
docs/guides/
docs/references/
```

The Ops docs explain product-local semantics, composition, decisions, guides and provenance. They MUST reference canonical Chronica owners for shared World/Authority/Execution/Memory/Safety semantics rather than copying those semantics into a competing Ops architecture.

Donor docs may be mined for factual behavior and provenance, then rewritten/retired under the new documentation taxonomy.

## 13. License and provenance

Open source does not mean obligation-free; acquisition does not automatically extinguish third-party obligations.

Preserve, as actually required:

```text
licenses
notices
copyright
patent notices
source-offer obligations
attribution
third-party proprietary notices
redistribution terms
acquisition/assignment references needed to explain white-label rights
```

## 14. Agent prohibitions

Unless explicitly waived, coding agents working on an Ops must not:

```text
invent a full product architecture before cloning the donor
clone only a convenient subset and call it full assimilation
keep the donor only in an external temporary checkout without pushing a baseline into Ops Git
replace donor investigation with generic generated CRUD
claim feature parity without a complete donor census
claim white-label rights without recorded legal/IP evidence
create a permanent new implementation beside donor legacy instead of burning legacy down
remove donor code before callers/tests are migrated
skip the Chronica conflict audit before adding a major semantic/runtime abstraction
erase legally required notices/provenance because code was reformatted/refactored
preserve donor branding as the product identity merely because renaming is inconvenient
maintain donor docs as a second uncontrolled documentation universe
```

## 15. Completion signal

A donor is assimilated when all of the following are true:

```text
complete original tracked tree was cloned and accounted for
real donor baseline was pushed into Ops Git history
retained behaviors have explicit owners
current coding/reference decisions have been conflict-audited against a recorded Chronica SHA
public product identity is Chronica-branded where rights permit
required donor/third-party attribution remains correctly preserved
product-local docs follow the Chronica-style docs kernel
UI/API/CLI/MCP/workers converge on one application/domain ownership path
standalone behavior is proven
connected Chronica semantics are proven where required
source/provenance/IP lineage remains auditable
legacy donor implementation/callers/architecture ownership have burned down to zero
```

The measure is not "every original line was rewritten". The measure is **real baseline preservation, monotonic legacy extinction, conflict-aware refactor, white-label convergence, preserved obligations and behavioral proof**.
