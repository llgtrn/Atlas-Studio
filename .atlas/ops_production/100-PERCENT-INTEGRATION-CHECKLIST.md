---
id: ops-production-100-percent-integration-checklist
type: reference
status: active
canonical: true
---
# 100% Integration, Refoundation, Absorption, and Production Release Gate

An Ops is not ready for Chronica merely because it has one webhook, one bridge, one MCP tool, a renamed logo, or a new architecture beside untouched donor code. Production maturity is evidence-based across **real donor Git baseline, complete donor intake, Chronica conflict audit, semantic convergence, monotonic legacy burn-down, IP/provenance, Chronica-branded product refoundation, docs governance, standalone operation, semantic/Fabric compatibility, authority, evidence, recovery and terminal absorption**.

## Incubation proof states

```text
REAL_DONOR_GIT_BASELINE_PUSHED
FULL_DONOR_CENSUS_COMPLETE
CHRONICA_CONFLICT_AUDIT_CURRENT
SEMANTIC_CONVERGENCE_PROVEN
LEGACY_BURNDOWN_TRACKED
IP_RIGHTS_VERIFIED_FOR_SCOPE
WHITE_LABEL_CENSUS_COMPLETE
OPS_DOCS_KERNEL_COMPLETE
DONOR_PROVEN
STANDALONE_PROVEN
PORT_CONVERGENCE_PROVEN
MCP_PROVEN
CHRONICA_MAPPING_PROVEN
FABRIC_COMPATIBILITY_PROVEN
AUTHORITY_PROVEN
EVIDENCE_PROVEN
OFFLINE_RECONNECT_PROVEN
VERSION_COMPATIBILITY_PROVEN
ZERO_DONOR_LEGACY_IMPLEMENTATION
ZERO_UNEXPLAINED_DUPLICATE_SEMANTICS
ABSORPTION_READY
```

## Terminal absorption proof states

```text
ABSORPTION_CENSUS_COMPLETE
ABSORPTION_MAP_COMPLETE
CHRONICA_NATIVE_ABSORPTION_PROVEN
SEMANTIC_PARITY_PROVEN
FABRIC_PARITY_PROVEN
STANDALONE_ARTIFACT_PARITY_PROVEN
MCP_STANDALONE_PARITY_PROVEN
CONNECTED_PARITY_PROVEN
PROVENANCE_PRESERVED
DUPLICATE_SOURCE_PATHS_RETIRED
DUPLICATE_SEMANTIC_PATHS_RETIRED
OPS_SOURCE_UNIVERSE_RETIRED
```

A label without source/Git/tests/runtime evidence is not proof.

## Gate 1 — real donor Git baseline

```text
[ ] real upstream OSS/acquired donor repository cloned/fetched
[ ] exact donor commit/tag pinned
[ ] complete tracked donor tree imported into the Ops repository
[ ] Ops donor-baseline commit exists
[ ] donor-baseline tag or branch exists
[ ] donor baseline was pushed to the Ops Git remote
[ ] baseline commit/tag recorded in ops.manifest.yaml
[ ] submodule/LFS/generated-source boundaries accounted for
[ ] baseline build/run/test result recorded
```

A donor that exists only in a temporary sibling checkout or was merely used as inspiration does not satisfy this gate.

Do not rewrite the baseline out of history after refactor.

## Gate 2 — complete donor reality

```text
[ ] complete source/schema/routes/services/jobs/UI/integrations/tests/docs census exists
[ ] migrations/seeds/fixtures/build/deploy/tooling inventoried
[ ] brand/assets/public metadata inventoried
[ ] no source area omitted merely because it looked unimportant
[ ] KEEP / ADAPT / REPLACE / REMOVE / DEFER decisions are evidence-backed
```

Selective copying does not satisfy full donor intake.

## Gate 3 — Chronica semantic convergence + conflict audit

```text
[ ] exact CHRONICA_REFERENCE_SHA recorded
[ ] relevant canonical architecture owners read
[ ] relevant Chronica runtime/code owners searched where material
[ ] relevant Chronica tests searched where material
[ ] donor/Ops semantic fingerprint extracted for each active material vertical
[ ] search performed by meaning/state transition/effect, not only identical names
[ ] identity/resource/state conflicts checked
[ ] Capability registry/resolution conflicts checked
[ ] binding/provider conflicts checked
[ ] authority/normative conflicts checked
[ ] execution/workflow conflicts checked
[ ] event/evidence/canonicalization conflicts checked
[ ] memory/context/sovereignty conflicts checked where relevant
[ ] machine/safety conflicts checked where relevant
[ ] recovery/reconciliation/Fabric conflicts checked
[ ] core/runtime/adapter placement conflicts checked
[ ] crate/package/service naming conflicts checked
[ ] domain command/query/event vocabulary checked against Chronica
[ ] every material semantic has explicit match status
[ ] every material semantic has explicit convergence disposition
[ ] same-meaning/different-name internal duplicates identified
[ ] provider/donor aliases are boundary-local or have explicit retirement criteria
[ ] richer reusable Ops semantics have a Chronica extension/adoption decision
[ ] every material conflict has an explicit decision
```

Semantic fingerprints cover as applicable:

```text
resource/entity
inputs/outputs
preconditions/postconditions
state transition
effect
authority
normativity
evidence
idempotency/correlation
failure/UNKNOWN
risk
persistence/provider boundary
real callers/tests
```

Allowed semantic match statuses include:

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

Required semantic dispositions include one of:

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

Other architecture conflict decisions may include:

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

Fail this gate when two internal canonical names represent the same resource/state transition/effect/authority/evidence meaning without an explicit compatibility/provider-boundary reason.

`NO_EQUIVALENT_FOUND` is not enough by itself; the searched Chronica owners/code/tests must be recorded.

Refresh this audit before major new abstractions, when Chronica has materially advanced, or when Ops discovers a richer semantic that may belong in Chronica.

## Gate 4 — legacy + semantic-duplicate burn-down

Measure donor ownership and semantic duplication at the current Ops HEAD:

```text
[ ] legacy files/modules counted
[ ] legacy public callers counted
[ ] legacy DB/migration ownership counted
[ ] legacy UI routes/business components counted
[ ] legacy background jobs counted
[ ] legacy policy/authority paths counted
[ ] legacy docs/brand surfaces counted
[ ] duplicate internal semantics counted
[ ] temporary compatibility aliases counted
```

For every refactor vertical:

```text
[ ] target owner implemented
[ ] canonical semantic term selected
[ ] donor/provider terminology translated at a boundary when retained
[ ] real callers migrated
[ ] UI/API/MCP/workers converge on one internal semantic for the same effect
[ ] tests/data/migrations migrated as needed
[ ] semantic parity/differential tests added where migration risk warrants them
[ ] standalone/MCP/connected behavior tested as relevant
[ ] obsolete donor path deleted after proof
[ ] duplicate semantic/alias retired after proof where possible
[ ] legacy + semantic counters remeasured
```

A migration that adds a new path but leaves the donor path indefinitely active fails this gate. A rename that leaves two permanent internal meanings for the same behavior also fails this gate.

Terminal donor-refactor state:

```text
[ ] legacy_code_remaining = 0
[ ] legacy_public_callers_remaining = 0
[ ] legacy_architecture_ownership_remaining = 0
[ ] unexplained_duplicate_semantics_remaining = 0
```

Required licenses, attribution, provenance, Git history, provider names and approved boundary compatibility aliases are excluded from legacy-code counts.

## Gate 5 — acquisition / license / IP provenance

```text
[ ] ordinary OSS license/notice obligations verified
[ ] acquisition/asset-purchase reference recorded where relied upon
[ ] source-code ownership/license scope recorded where relied upon
[ ] copyright assignment/license scope recorded where relied upon
[ ] patent assignment/license scope + identifiers/jurisdiction recorded where applicable
[ ] trademark/brand assignment/license scope recorded where applicable
[ ] modification/redistribution/white-label/sublicensing rights recorded where relied upon
[ ] territorial/time/field-of-use limits recorded where applicable
[ ] surviving third-party obligations recorded
[ ] IP status explicit: VERIFIED / PARTIAL / UNRESOLVED
```

A user statement that rights were purchased is not by itself the evidence record.

## Gate 6 — white-label / product identity

Every material donor identity surface is classified:

```text
REBRAND_TO_CHRONICA
RETAIN_FOR_REQUIRED_ATTRIBUTION
RETAIN_AS_PROVIDER_REFERENCE
RETAIN_AS_PROVENANCE_ONLY
REMOVE
```

Check product/app names, repo/package/module/crate names, logos/assets, UI strings, email/notifications, CLI/MCP descriptions, API/server banners, OAuth/bundle/service/container/image names, domains/docs/support links, telemetry IDs, seeds/demo/screenshots and legal notices.

A global string replace is not proof of complete white-labeling.

## Gate 7 — Ops documentation kernel

```text
[ ] docs/README.md
[ ] docs/INDEX.md
[ ] docs/TEMPLATE.md
[ ] docs/architecture/README.md
[ ] docs/architecture/constitution/
[ ] docs/architecture/foundation/
[ ] docs/architecture/governance/
[ ] docs/architecture/integration/
[ ] docs/architecture/operations/
[ ] docs/blueprints/
[ ] docs/decisions/
[ ] docs/guides/
[ ] docs/references/
```

And:

```text
[ ] every maintained local doc is registered/routed
[ ] one local owner per stable local responsibility
[ ] blueprints compose instead of creating subsystems
[ ] donor docs were censused before retirement
[ ] provenance/IP/license facts live in references/provenance surfaces
[ ] Ops docs reference canonical Chronica World/Capability Resolution/Authority/Execution/etc instead of redefining them
[ ] Ops docs use the same canonical semantic vocabulary for equivalent meanings
[ ] semantic aliases/extensions are documented with provenance and retirement/adoption status
[ ] no local Capability registry/layer is introduced by docs
[ ] broken/stale doc links are CI-detectable
[ ] Docs Atlas or equivalent graph projection is rebuildable and never truth
```

## Gate 8 — manifest/discovery

```text
[ ] root ops.manifest.yaml exists
[ ] manifest_version supports semantic convergence contract
[ ] Ops ID/version/modes accurate
[ ] real donor baseline commit/tag/remote status accurate
[ ] legacy_burndown counters accurate
[ ] Chronica reference SHA / conflict audit accurate
[ ] semantic_convergence mappings accurate
[ ] canonical terms after refactor accurate
[ ] unexplained_duplicate_semantics list empty for proven scope
[ ] extension_candidates accurately reflect richer Ops semantics awaiting Chronica decision
[ ] Chronica-branded product identity accurate
[ ] UI/API/CLI/MCP/worker surfaces accurate
[ ] public resources/queries/commands/events inventoried
[ ] donor lineage recorded
[ ] acquisition/IP evidence status referenced
[ ] contract versions declared independently
[ ] no secrets/confidential agreements in manifest
```

## Gate 9 — one-core port convergence

```text
[ ] UI uses canonical application command/query owner
[ ] API/CLI use the same owner
[ ] MCP uses the same owner
[ ] workers invoke the same domain/application semantics
[ ] equivalent effects use the same internal canonical command/event vocabulary
[ ] provider aliases are translated before application/core semantics
[ ] Chronica bridge contains mapping/integration logic, not duplicate business rules
[ ] no surface-specific authorization/normative fork
[ ] no port directly mutates DB/provider around application policy
```

## Gate 10 — standalone reality

```text
[ ] install/run without separate Chronica server where promised
[ ] local durable state boots
[ ] local auth/policy works
[ ] critical workflows work
[ ] Chronica-branded UI works independently where promised
[ ] MCP works independently where promised
[ ] effects produce local audit/evidence
[ ] backup/restore works
[ ] migration/upgrade works
[ ] restart/recovery works
```

## Gate 11 — MCP contract

```text
[ ] every MCP tool classified READ / ANALYZE / EFFECTFUL / HIGH_RISK_EFFECTFUL
[ ] effectful tools use same application owner as UI/API/CLI
[ ] MCP semantic names map to the same canonical internal meaning as other ports
[ ] standalone effectful tools use local policy/authority
[ ] connected shared effects use Chronica authority where required
[ ] no direct-provider bypass tool exists
[ ] idempotency/unknown-outcome semantics explicit
```

## Gate 12 — connected semantic coverage

For every externally meaningful workflow:

```text
[ ] identity/resource mapping
[ ] durable state/lifecycle mapping
[ ] semantic aspect classification
[ ] derived Can(...) / Capability Resolution mapping
[ ] canonical semantic name
[ ] donor/provider aliases and boundary translation
[ ] query/command mapping
[ ] authority/delegation rule
[ ] normative mapping where real
[ ] idempotency/correlation
[ ] event/evidence mapping
[ ] Holding/scope/sensitivity/disclosure mapping
[ ] UI projection path where human-facing
[ ] MCP path where agent-facing
[ ] semantic parity evidence where donor and Chronica/Ops paths coexist
```

Keep:

```text
Can != Authorized != May != Must
ExecutionAdmission != Canonicalization
Projection != Truth
same semantic != same provider name
```

## Gate 13 — Fabric / reconciliation compatibility

```text
[ ] stable identities survive boundary crossing
[ ] semantic action meaning survives transport/provider changes
[ ] Binding remains distinct from Authority
[ ] endpoint/MCP reachability does not imply permission
[ ] correlation/idempotency survives retry
[ ] provider observations remain evidence/observation until reconciled/canonicalized
[ ] Provider Success != Reconciled Effect
[ ] UNKNOWN is durable
[ ] outbox/inbox/reconnect behavior tested where relevant
[ ] disclosure remains purpose/scope/sensitivity gated
[ ] version compatibility explicit
```

## Gate 14 — version compatibility

```text
[ ] Ops release version
[ ] protocol/API version
[ ] Chronica integration version
[ ] semantic contract version where used
[ ] event envelope version
[ ] MCP contract version
[ ] UI/product compatibility version
[ ] unsupported major versions fail closed
[ ] migration + rollback plan exists for breaking rollout
```

## Gate 15 — absorption readiness

Before invoking `ABSORB-INTO-CHRONICA-GOAL.md`:

```text
[ ] real pushed donor baseline exists
[ ] complete tracked-source census exists
[ ] current Chronica semantic/conflict audit exists
[ ] zero unexplained duplicate semantics for absorption scope
[ ] richer reusable Ops semantics have explicit Chronica adoption decisions
[ ] legacy donor implementation has burned down to terminal allowed state for absorption scope
[ ] complete public-workflow inventory exists
[ ] IP/license/provenance status current
[ ] white-label census complete
[ ] Ops docs kernel complete/current
[ ] standalone baseline reproducible
[ ] MCP baseline reproducible where exposed
[ ] connected/Fabric baseline exists where required
[ ] public contract versions frozen for differential tests
```

## Gate 16 — absorption map / Chronica-native ownership

Every retained responsibility is one of:

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

And:

```text
[ ] no permanent Ops/domain/organ/cap/fabric/legal/normative/policy-engine universe proposed for convenience
[ ] duplicate Chronica/source semantics identified
[ ] same-meaning/different-name semantics converged or explicitly boundary-scoped
[ ] Capability remains derived from World/state/bindings/constraints
[ ] provider/vendor mechanics use adapters
[ ] real callers migrated before duplicate deletion
[ ] no permanent duplicate business/Fabric/normative implementation remains
```

## Gate 17 — post-absorption Chronica-branded standalone parity

```text
[ ] standalone server/UI/CLI/MCP builds from Chronica where applicable
[ ] product identity remains Chronica-branded
[ ] canonical semantic vocabulary remains consistent with Chronica
[ ] required attribution/provenance still ships where required
[ ] local auth/persistence/provider config works
[ ] evidence/recovery behavior preserved
```

## Gate 18 — full public-surface differential parity

Every public workflow is exactly one of:

```text
PARITY
INTENTIONALLY_VERSIONED_CHANGE
EXPLICIT_HUMAN_APPROVED_RETIREMENT
```

Every material donor identity surface is exactly one of:

```text
CHRONICA_BRANDED
REQUIRED_ATTRIBUTION
PROVIDER_REFERENCE
PROVENANCE_ONLY
REMOVED
```

Every material semantic is exactly one of:

```text
CANONICAL_CHRONICA_SEMANTIC
BOUNDARY_ALIAS_TO_CHRONICA
APPROVED_CHRONICA_EXTENSION
TRUE_PRODUCT_LOCAL_SEMANTIC
PROVIDER_MECHANIC
EXPLICIT_VERSIONED_CHANGE
APPROVED_RETIREMENT
```

No unexplained behavioral, semantic, Fabric, source, brand, docs or legal/provenance gap.

## Gate 19 — source + semantic-universe extinction

```text
[ ] old Ops/donor source paths no longer own canonical behavior
[ ] zero donor legacy implementation remains
[ ] zero unexplained duplicate internal semantics remain
[ ] no canonical feature development required in both repos
[ ] Chronica-branded distributions build from Chronica
[ ] donor/Ops/IP provenance retained
[ ] old source repo can be archived/mirrored/read-only
[ ] Chronica is sole native implementation source after retirement
```

## Definitions

During incubation:

```text
100% integration
= real donor baseline pushed
+ complete donor tree accounted for
+ current Chronica conflicts audited
+ semantic convergence proven for active scope
+ public workflows mapped/proven
+ legacy burn-down tracked
+ rights/obligations recorded
+ material donor identity surfaces dispositioned
+ Ops docs kernel current
```

Terminal donor refoundation:

```text
100% donor refoundation
= ZERO_DONOR_LEGACY_IMPLEMENTATION
+ zero donor public callers
+ zero donor architecture ownership
+ ZERO_UNEXPLAINED_DUPLICATE_SEMANTICS
+ preserved provenance/history/obligations
```

At terminal absorption:

```text
100% absorption accounting
= every public workflow + retained source responsibility + semantic responsibility + brand surface + maintained docs responsibility
  has a proven native owner, intentional external/reference classification,
  approved canonical extension, true product-local classification,
  intentionally versioned change, required legal/provenance retention,
  or explicit approved retirement
```

100% does **not** mean every original byte remains forever. It means nothing material is silently lost, duplicated, semantically forked, misbranded, legally overclaimed, or left without an owner.
