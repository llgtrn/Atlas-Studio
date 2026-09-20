# Development Cell / Package Production North Star

This path retains the historical "Ops" name for compatibility. The semantic unit is now a **Development Cell** that incubates a **Chronica Package** from real donor/acquired source and domain-focused engineering.

A Development Cell may be independently developed and tested. A package projection may be separately deployable where useful. Neither is a permanent peer architecture above or beside Chronica, and neither may own standalone canonical sovereignty.

Fleet allocation precedes per-Cell/package lifecycle. Every donor in the master corpus is assigned to exactly one primary absorber (or to direct Chronica-native work where explicitly mapped); other packages are reference consumers rather than duplicate primary refactors.

The lifecycle is:

```text
FLEET CLOSED-WORLD DONOR ALLOCATION
        ↓
PIN CHRONICA_REFERENCE_SHA + CELL_SHA + PACKAGE_ID
        ↓
GENERATE / REVIEW CELL MIRROR + PACKAGE CONTRACT
        ↓
REAL OSS / ACQUIRED DONOR PRODUCT
        ↓
COMPLETE REPOSITORY CLONE @ EXACT SHA
        ↓
PUSH REAL DONOR BASELINE INTO CELL GIT
        ↓
LEGAL/IP/PROVENANCE + CODE/UI/DOCS/BRAND CENSUS
        ↓
CHRONICA CONFLICT AUDIT @ EXACT CHRONICA SHA
        ↓
CHRONICA PACKAGE REFOUNDATION
        ↓
VERTICAL-BY-VERTICAL LEGACY BURN-DOWN
        ↓
ZERO DONOR-OWNED LEGACY IMPLEMENTATION
        ↓
LOCAL HARNESS / MCP / CHRONICA-COMPOSED PROOF
        ↓
ABSORPTION READY
        ↓
CHRONICA-NATIVE SOURCE OWNERSHIP
        ↓
CHRONICA PACKAGE PROJECTIONS / DISTRIBUTIONS
```

## 1. Full-repository reality comes first

For a donor chosen for deep assimilation, the entire tracked repository is the intake unit.

```text
clone complete donor
-> pin exact SHA/tag
-> import into Ops repository
-> push baseline commit/tag/branch to Ops remote
-> preserve history/provenance boundaries
-> baseline build/test
-> census code/schema/UI/docs/brand/integrations/tests/deploy
-> verify license/acquisition/IP rights relied upon
-> only then refactor
```

A donor that exists only as an external temporary checkout is not enough. The real donor baseline must be present in auditable Development Cell Git history before architectural refactor.

Full-repository intake does **not** mean every original file must survive terminally. It means every meaningful source responsibility is accounted for before KEEP / ADAPT / REPLACE / REMOVE / DEFER decisions.

## 2. The baseline is preserved; legacy implementation is not

The baseline commit is historical evidence. The baseline architecture is not sacred.

After the baseline is pushed, refactor proceeds as a one-way burn-down:

```text
DONOR LEGACY
   ↓
understand one vertical
   ↓
compare with current Chronica
   ↓
refactor into target responsibility
   ↓
migrate callers/tests/data
   ↓
delete obsolete donor path
   ↓
LESS LEGACY
```

Every substantial coding loop should reduce the legacy surface or report a specific blocker with evidence.

The terminal goal is:

```text
legacy_code_remaining = 0
legacy_public_callers_remaining = 0
legacy_architecture_ownership_remaining = 0
```

This does not mean deleting required licenses, attribution, provenance, acquisition records, Git history, provider names, or intentional compatibility aliases.

## 2A. Development Cell/package is a mirror target, not a peer canonical system

During incubation:

```text
Chronica        = read-only canonical semantic/architecture reference for the slice
Development Cell = mutable engineering/proving target
Chronica Package = logical domain/capability/adapter/projection output
Donor            = source/provenance input
```

The worker freezes exact `CHRONICA_REFERENCE_SHA` and `CELL_SHA`. Cross-repository access never means cross-repository write authority.

A reusable general improvement discovered by the package is packaged as an explicit `CHRONICA_CANDIDATE`; Chronica evaluates it through its own code/test/architecture/integration path. Affected packages then reconverge on the canonical result.

The Cell Mirror Atlas report + chronica-package.json is required evidence for substantial refoundation work. Missing configuration, stale/unverified Chronica pin, language-law violations, donor coupling or missing docs kernel must be visible rather than silently assumed away.

The Fleet Atlas additionally verifies that the Development Cell/package is registered, every master donor is accounted for, this Cell/package is primary only for donors allocated to it, and the same donor is not claimed as primary by another Cell/package. Missing donors/Reports remain visible hard gaps; fleet coordination never converts absence into completion.

## 2B. Package non-sovereignty gate

Each Cell maintains `chronica-package.json` with:

```text
canonical_runtime = CHRONICA_REQUIRED
standalone_sovereignty = false
```

Independent local development/test harnesses are allowed. Separately deployed projections are allowed. Canonical World, Identity, Authority, Execution, Evidence, Memory, canonical history and shared truth remain Chronica-only.

Local databases may exist only when their role is explicit, such as provider state, local cache, test fixture state or rebuildable projection. A package-local database must not quietly become a peer canonical truth store.

## 3. Chronica conflict audit is continuous, not only an absorption-time check

Every substantial Ops refactor records an exact `CHRONICA_REFERENCE_SHA` and consults the relevant Chronica architecture/runtime before introducing or preserving a major abstraction.

Always check for conflict/duplication around:

```text
identity/resource/state
relations/bindings
Capability Resolution / Can(...)
normativity / May-Must-Forbidden
authority / Authorized(...)
ExecutionAdmission / WorkRun
events/evidence/canonicalization
memory/context/sovereignty
machine/safety where relevant
recovery/reconciliation/idempotency
Fabric boundary semantics
runtime/store/adapter responsibility
crate/package/service ownership
```

The question is not merely "can this donor code run?" but:

```text
Does Chronica already own this semantic/runtime responsibility?
Would preserving this donor abstraction create a competing universe?
Should we reuse/map/merge/adapterize/retire it instead?
```

Allowed outcomes:

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

## 4. Legal/IP evidence controls what white-labeling may do

A human may state that the donor company, full license, patents, trademarks, software rights, or source code have been purchased/acquired. Treat that as an instruction to inspect and record the corresponding evidence, not as a self-proving legal conclusion.

Where the verified rights permit modification and white-labeling, the product identity converges to an approved **Chronica <Product>** identity.

Where third-party obligations survive, retain them exactly as required.

Therefore:

```text
WHITE_LABEL_PRODUCT_IDENTITY != ERASE_PROVENANCE
```

## 5. What survives and what does not

May survive indefinitely:

```text
Chronica-branded product name
standalone deployment profile
standalone server/UI/CLI/MCP
provider-specific adapters
compatibility contract
required donor/third-party attribution
historical donor baseline in Git
```

Does not survive as permanent canonical ownership:

```text
donor product identity as Chronica's owner
separate Ops business universe
separate capability registry
separate authority model
separate normative/policy universe
separate execution/workflow engine
separate event/evidence model
separate shared identity model
permanent donor monolith
permanent cap/ or organs/<ops> universe
uncontrolled donor documentation tree
```

## 6. Incubation properties

Before absorption an Ops must be:

```text
REAL-DONOR-GIT-BASELINED
FULL-DONOR-INTAKE-DERIVED
LEGACY-BURNDOWN-TRACKED
CHRONICA-CONFLICT-AUDITED
IP/PROVENANCE-ACCOUNTED
CHRONICA-BRANDED
SELF-SUFFICIENT
ONE-CORE / MANY-PORTS
MCP-OPERABLE
CHRONICA-MAPPABLE
SEMANTICALLY EXPLICIT
DOCUMENTED
VERSIONED
RECONCILABLE
AUTHORITY-AWARE
EVIDENCE-PRODUCING
```

## 7. One-world semantic law

Ops integration converges on the same Chronica world semantics rather than preserving a separate donor meaning system.

Keep these distinctions explicit:

```text
Fact != Norm
Interpretation != Source Text
Can(...) != Authorized(...) != May(...) != Must(...)
ExecutionAdmission != Canonicalization
Provider Success != Reconciled Effect
UNKNOWN != SUCCESS
```

## 8. One core during incubation

```text
UI / API / CLI / MCP / Workers / Chronica Bridge
                    ↓
        one application command/query layer
                    ↓
             domain + policy
                    ↓
        persistence/provider ports
```

The application/domain owner keeps business semantics reusable across surfaces. No port owns a second product truth.

## 9. Chronica-style documentation kernel is mandatory

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

Local Ops docs may own product-local workflows, deployments, provider details, compatibility, migration and provenance. They must reference rather than redefine canonical Chronica owners for World, Capability Resolution, Authority, Normativity, Execution, Evidence, Memory, Context, Sovereignty and Safety.

A local Docs Atlas may be generated as a projection of the Ops docs. A System Atlas Ops Mirror report measures local repository/runtime convergence against the pinned Chronica reference. Neither projection is canonical truth.

## 10. White-label refoundation is architectural, not cosmetic

A successful white-label pass accounts for product/app name, repo/package/crate/module names, logos/icons, UI strings, email/notification templates, CLI/MCP descriptions, API/server banners, OAuth/bundle/service/container/image names, domains/docs/support links, telemetry identifiers, seed/demo/screenshots and legal notices.

Each material donor identity is explicitly rebranded, retained for attribution/provider reference/provenance, or removed.

## 11. Standalone and connected proof

Standalone proves that the Chronica-branded product is real and not dependent on hidden Chronica server behavior.

Connected mode proves that shared semantics converge to Chronica identity/resource/state/Capability Resolution/normative/authority/execution/event/evidence contracts.

## 12. Absorption target

Once mature, retained source responsibilities are decomposed by canonical responsibility:

```text
semantics/invariants        -> core
durable runtime behavior    -> runtime
provider/protocol mechanics -> adapter
persistent cognition / learning / adaptation -> organism
thin deployable surfaces    -> apps
static semantics            -> graph
static implementation maps  -> bindings
deployment/bootstrap        -> deploy
repo/build tooling          -> tools
```

Do not create permanent `legal`, `normative`, `policy-engine`, `cap`, `organs/`, product-domain or Ops-named canonical universes merely to preserve donor structure.

## 13. Terminal donor-refactor gate

Before an Ops may claim donor refoundation complete:

```text
real donor baseline exists in pushed Development Cell Git history
complete donor tree was accounted for
white-label/provenance obligations are resolved or explicitly bounded
Chronica conflict audit is current for major abstractions
all public workflows have migrated to intended owners
legacy donor implementation = 0
legacy donor callers = 0
legacy donor architecture ownership = 0
standalone/MCP/connected tests are green or explicitly bounded
```

## Final invariant

> **Clone the real donor into the Ops repository, push the donor baseline, then refactor forward through ordinary Git history while continuously comparing against current Chronica. Every proven vertical must shrink donor legacy until no donor-owned implementation remains. Preserve provenance and legal obligations, not obsolete architecture.**
