---
id: ops-production-contracts-ops-manifest
type: reference
status: active
canonical: true
---
# Development Cell / Package Manifest Compatibility Contract

Status: **ACTIVE COMPATIBILITY CONTRACT — v7**

The historical `ops.manifest.yaml` filename may remain for existing repositories and automation. It now describes a **Development Cell/package distribution surface**, not an autonomous Ops system.

Every active Development Cell SHOULD also expose `chronica-package.json`, which owns stable package identity and the non-sovereignty contract.

The manifest is bounded discovery/governance metadata. It does **not** grant runtime authority, legal rights, semantic truth, source trust or production success by declaration alone.

Its purpose is to let humans, agents and CI answer from one controlled surface:

```text
which Development Cell/package this legacy repository represents
which external sources have been admitted and under which mode
which exact external revisions/versions are trusted for the declared scope
which real donor tree became its Git baseline when deep-fork mode applies
how much donor/legacy ownership remains
which Chronica SHA was actually consulted
how donor/package semantics converge with Chronica
which aliases are boundary-only
which richer semantics may need Chronica extension
what docs/provenance/IP/branding gates exist
what local-dev/projection/Chronica-composed surfaces exist
what logistics mechanisms are enabled
which Git/build/runtime artifacts are actually deployed
```

## Package contract precedence

`chronica-package.json` is authoritative for Cell/package identity and non-sovereignty. The legacy manifest cannot override it.

The following are never legitimized by manifest declaration:

```text
peer canonical World
peer canonical Identity
peer canonical Authority
peer canonical Execution
peer canonical Evidence
peer canonical Memory
peer canonical history/shared truth
```

Local storage, authorization stubs and execution harnesses are allowed only when explicitly classified as provider-local, cache/projection, simulation/test or development-only mechanics.

## Required location

```text
<ops-repo>/ops.manifest.yaml
```

Do not place secrets, private tenant data, confidential agreement text, signatures, payment details or credentials in the manifest.

## Minimum v7 shape

```yaml
manifest_version: 7

development_cell:
  id: helpdesk
  repository: llgtrn/HelpdeskOps

package:
  id: helpdesk
  kind: DOMAIN_PACKAGE
  name: Chronica Helpdesk
  version: 1.0.0
  maturity: PACKAGE_PROVEN
  contract: chronica-package.json
  canonical_runtime: CHRONICA_REQUIRED
  standalone_sovereignty: false

modes:
  independent_development_harness: true
  projection_deployable: true
  chronica_composed: true

fleet:
  mirror_contract: chronica-mirror.json
  allocation_source: llgtrn/Chronica:tools/system-atlas/fleet/ops-registry.yaml
  donor_corpus_source: llgtrn/Chronica:tools/refoundation/donor-corpus.yaml
  role_source_of_truth: chronica-mirror.json
  generated_fleet_progress_stored_here: false

surfaces:
  ui: true
  api: true
  cli: false
  mcp: true
  workers: true

contracts:
  source_admission: 1
  ops_protocol: 1.0.0
  chronica_integration: 1.0.0
  semantic_contract: 1.0.0
  event_envelope: 1.0.0
  mcp_contract: 1.0.0
  ui_host: 1.0.0

resources:
  - conversation
  - contact
  - inbox

commands:
  - conversation.reply
  - conversation.close

queries:
  - conversation.get
  - conversation.search

events:
  - conversation.replied
  - conversation.closed

semantics:
  aspects:
    - descriptive
    - active
  capability_resolution:
    model: derived_can
    canonical_registry: false
  normative:
    present: false
    source_families: []
    governed_commands: []
    resolution_contract: null

authority:
  canonical_policy: chronica
  local_harness_policy: noncanonical_test_or_dev_only

storage:
  local_store: postgres
  local_store_classification: provider_local_cache_projection_or_test_state
  canonical_truth_store: false
  outbox: true
  inbox_deduplication: true

source_admission:
  contract_version: 1
  sources:
    - id: upstream-helpdesk
      mode: DEEP_FORK
      source: https://github.com/example/upstream
      revision: <exact-source-sha-or-version>
      integrity_status: INTEGRITY_VERIFIED
      rights_status: RIGHTS_VERIFIED_FOR_MODE
      security_status: NO_KNOWN_BLOCKING_FINDING
      secret_data_status: CLEAN_OR_REMEDIATED
      sbom_ref: provenance/sbom/upstream-helpdesk.cdx.json
      upstream_policy: ABSORB_AND_RETIRE
      admission: ADMIT_DEEP_FORK
      evidence_refs:
        - provenance/source/upstream-helpdesk.yaml

donor:
  required: true
  complete_tree_accounted: true
  repositories:
    - url: https://github.com/example/upstream
      commit: <exact-donor-sha>
      license: <verified-spdx-or-bounded-reference>
  cell_git_baseline:
    imported: true
    baseline_commit: <cell-commit-containing-real-donor-tree>
    baseline_tag_or_branch: donor-baseline/<name>-<sha>
    remote_pushed: true
    baseline_build_status: passed_or_bounded_failure

legacy_burndown:
  status: IN_PROGRESS
  donor_legacy_files_or_modules_remaining: 0
  donor_public_callers_remaining: 0
  donor_db_or_migration_ownership_remaining: 0
  donor_ui_routes_remaining: 0
  donor_background_jobs_remaining: 0
  donor_policy_authority_paths_remaining: 0
  donor_docs_brand_surfaces_remaining: 0
  duplicate_semantics_remaining: 0
  compatibility_aliases_remaining: 0
  terminal_target: ZERO_DONOR_LEGACY_IMPLEMENTATION_AND_ZERO_UNEXPLAINED_DUPLICATE_SEMANTICS

chronica_reference:
  repository: llgtrn/Chronica
  ref: <branch-or-main>
  sha: <exact-40-character-sha>
  owners_read:
    - docs/architecture/foundation/capability-resolution.md
    - docs/architecture/governance/authority.md
    - docs/architecture/governance/execution.md
  code_paths_read: []
  test_paths_read: []
  conflict_audit_status: proven
  conflicts_found: []
  decisions: []

semantic_convergence:
  status: proven
  contract: docs/ops_production/contracts/SEMANTIC-CONVERGENCE.md
  chronica_sha: <same-exact-audited-sha>
  mappings:
    - semantic_id: conversation.close
      donor_terms:
        - resolve_ticket
      package_term_before: resolve_ticket
      fingerprint:
        resource: conversation
        transition: open_to_closed
        effect: close_conversation
        authority: required
        evidence: close_result
        idempotency: required
        unknown_outcome: reconcile
      match_status: EQUIVALENT_WITH_DIFFERENT_NAME
      disposition: ALIAS_AT_BOUNDARY
      canonical_term_after: conversation.close
      chronica_owner: docs/architecture/governance/execution.md
      chronica_code_refs:
        - <real-chronica-code-path-at-recorded-sha>
      chronica_test_refs:
        - <real-chronica-test-path-at-recorded-sha>
      search_evidence:
        owners:
          - docs/architecture/governance/execution.md
        code_paths:
          - <searched-code-path>
        test_paths:
          - <searched-test-path>
      temporary_aliases:
        - resolve_ticket
      extension_gap: null
      retirement_target: keep_resolve_ticket_only_at_provider_boundary
  unmapped_semantics: []
  unexplained_duplicate_semantics: []
  extension_candidates: []
  compatibility_aliases: []

rights:
  status: IP_RIGHTS_VERIFIED_FOR_SCOPE
  acquisition_reference: provenance/acquisition-ref.yaml
  source_code_right: verified
  copyright_scope: verified_or_not_applicable
  patent_scope: verified_or_not_applicable
  trademark_scope: verified_or_not_applicable
  white_label_right: verified
  third_party_obligations_remain: true

branding:
  target_identity: Chronica Helpdesk
  donor_identity_status: provenance_only
  census_complete: true
  compatibility_aliases: []

documentation:
  kernel_version: 1
  index: docs/INDEX.md
  template: docs/TEMPLATE.md
  architecture_router: docs/architecture/README.md
  blueprints_root: docs/blueprints
  decisions_root: docs/decisions
  guides_root: docs/guides
  references_root: docs/references
  governance_status: proven
  atlas_projection: enabled

logistics:
  contract_version: 1
  reality:
    mode: observed_working_tree
  docs_sync:
    mode: proposal_only
  deployment_evidence:
    path: .chronica/deployment-evidence.json
    required_environments:
      - staging
      - production

chronica:
  mapping_root: integration/chronica
  shared_resources:
    - conversation
    - contact
  local_only_classes:
    - provider_diagnostic
    - transport_runtime_state
```

## v7 source-admission + fleet rule

Manifest v7 retains the machine-readable source-admission contract introduced by v6 and adds Fleet/Mirror linkage; manifests remain evidence pointers, not self-proving truth.

Manifest v7 adds a pointer to Fleet allocation/Mirror configuration without copying generated Fleet progress into the manifest.

```text
ops.manifest.yaml        = stable Ops discovery/governance metadata
chronica-mirror.json     = stable Mirror pin/allocation/mapping contract
Fleet/Mirror reports     = generated disposable evidence
```

The source admission contract is:

```text
docs/ops_production/contracts/SOURCE-ADMISSION.md
```

Every material external production source should have a primary acquisition mode:

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

and one primary upstream lifecycle policy:

```text
PIN_AND_DIVERGE
TRACK_UPSTREAM
SECURITY_PATCH_ONLY
PROVIDER_MANAGED
ABSORB_AND_RETIRE
REFERENCE_ONLY
```

Source admission must preserve the distinction:

```text
SourceAdmission(code)
!= SemanticAdmission(meaning)
!= ExecutionAuthority(effect)
```

`ADMIT_DEEP_FORK` is the mode that activates the full donor Git-baseline law below. Ordinary package dependencies, provider SDKs, external services, reference implementations and specifications do not need to be artificially imported as donor monoliths merely to satisfy deep-fork rules.

Blocking source-admission states include:

```text
SOURCE_IDENTITY_UNRESOLVED
INTEGRITY_FAILED
RIGHTS_INCOMPATIBLE
UNRESOLVED_SECURITY_RISK
UNREMEDIATED_SECRET_EXPOSURE
UNAUTHORIZED_DATASET
```

Bounded unresolved states must remain explicit and must not be silently rewritten as proven.

## v6 logistics rule

Manifest v6 retains the executable logistics mechanisms introduced previously:

```text
Ops Reality Mirror
Semantic Convergence Checker
Absorption Planner
Docs Sync Proposal
CI gate
Deployment / runtime evidence
```

Their canonical contract is `docs/ops_production/contracts/LOGISTICS-KERNEL.md`.

`logistics.reality.mode: observed_working_tree` means implementation state is derived from the real Git/working tree, not typed manually into a progress percentage.

`logistics.docs_sync.mode: proposal_only` means code/absorption may trigger a docs-sync proposal but MUST NOT automatically rewrite canonical Chronica architecture docs.

`logistics.deployment_evidence` defines where rebuildable local/CI runtime identity observations are read and which environments are required before a production proof claim.

## Real donor Git baseline rule

A donor admitted as a deep fork must become actual Ops Git history before architectural refactor.

`donor.ops_git_baseline.imported: true` requires the complete tracked donor tree for the recorded donor revision to have been imported into the Ops repository.

`remote_pushed: true` is a factual claim that the baseline is present on an Ops remote. Local tooling may classify this as `UNVERIFIED_IN_LOCAL_CLONE` when remote refs are unavailable; it must not fabricate proof.

Do not rewrite the donor baseline away merely to make later history appear greenfield.

For source-admission modes other than `DEEP_FORK`, use the mode-specific provenance/pinning rules in `SOURCE-ADMISSION.md` rather than fabricating a donor baseline.

## Legacy burn-down rule

The long-run target for admitted donor/deep-fork work is:

```text
legacy_code_remaining = 0
legacy_public_callers_remaining = 0
legacy_architecture_ownership_remaining = 0
unexplained_duplicate_semantics_remaining = 0
```

The numeric `*_remaining` counters are machine-readable accounting. They must be supported by reproducible census rules in the Ops repository.

Git history, licenses, required attribution, provenance records, provider names at adapter boundaries and deliberately retained compatibility aliases are not donor implementation debt.

## Exact Chronica evidence rule

Every substantial semantic/refactor wave records an exact Chronica SHA.

For any Chronica-mapped semantic, the manifest records real owner/code/test paths. The executable checker verifies those paths **at the recorded SHA**, not merely at whatever Chronica HEAD happens to exist later.

A code path changing or disappearing in later Chronica history does not invalidate historical evidence; it means the next Ops refactor wave must refresh its Chronica comparison.

`NO_EQUIVALENT_FOUND` requires explicit `search_evidence` including owners and code paths searched. It is a search result, not permission to invent a new universal primitive casually.

## Semantic convergence rule

The semantic contract is `docs/ops_production/contracts/SEMANTIC-CONVERGENCE.md`.

For equivalent meaning:

```text
same resource/state transition/effect/authority/evidence meaning
=> reuse, map or deliberately extend the Chronica semantic
=> do not keep a second internal canonical name merely because donor vocabulary differs
```

Provider/donor terminology may remain at transport/adapter/compatibility boundaries.

When `ALIAS_AT_BOUNDARY` is claimed, the Reality Mirror may fail if the donor alias is still observed in non-boundary production source.

When Ops proves a reusable richer semantic than current Chronica, classify `CHRONICA_NARROWER` + `EXTEND_CHRONICA_SEMANTIC`, record the extension gap, generate a Docs Sync Proposal, and converge back after canonical admission.

## Documentation rule

`documentation.governance_status: proven` requires at least:

```text
docs/README.md
docs/INDEX.md
docs/TEMPLATE.md
docs/architecture/README.md
docs/blueprints/
docs/decisions/
docs/guides/
docs/references/
```

Ops docs may own product-local behavior, provider compatibility, deployment, migration and provenance. They must reference—not redefine—canonical Chronica World, Capability Resolution, Authority, Normativity, Execution, Evidence, Memory, Context, Sovereignty or Safety.

## Deployment/runtime evidence rule

Repository state and deployed state are distinct.

Deployment observations are generated evidence, not source truth. By default they live under `.chronica/` and remain untracked.

Each record contains:

```text
environment
exact gitSha
deploymentId
buildId when available
runtimeVersion when available
artifactDigest
healthStatus = PASS | FAIL | UNKNOWN
evidenceRefs
observedAt
```

A staging/production deployment on an older SHA must be reported as behind current HEAD instead of silently shown as current.

## Rights / white-label rule

Rights fields reference bounded evidence; the manifest does not itself prove ownership, patent scope, trademark rights, sublicensing rights or white-label permission.

Allowed top-level rights states include:

```text
IP_RIGHTS_VERIFIED_FOR_SCOPE
IP_RIGHTS_PARTIALLY_VERIFIED
THIRD_PARTY_OBLIGATIONS_REMAIN
IP_SCOPE_UNRESOLVED
```

White-labeling must not erase required third-party attribution/provenance.

## Release rule

A production Ops release is incomplete when any of the following is true:

```text
material external source lacks source-admission disposition
source identity is unresolved
integrity failed
rights are incompatible with the declared mode
security risk remains unresolved
secret/data contamination remains unremediated
real donor baseline missing for an admitted deep fork
complete donor tree not accounted for for an admitted deep fork
Chronica reference SHA stale/unverifiable for the active semantic wave
unmapped or unexplained duplicate semantics remain
provider aliases leak into internal canonical code contrary to their disposition
legacy terminal state is falsely claimed
required docs kernel is missing
required deployment evidence is absent
runtime health evidence says FAIL
rights/branding/provenance claims exceed evidence
```

## Tooling

Chronica's executable reference implementation currently includes:

```text
tools/ops-logistics/reality.mjs
tools/ops-logistics/semantic-check.mjs
tools/ops-logistics/absorption-plan.mjs
tools/ops-logistics/docs-sync-proposal.mjs
tools/ops-logistics/deployment-evidence.mjs
tools/ops-logistics/scorecard.mjs
tools/ops-logistics/ops-logistics.test.mjs
```

These tools are projections/checkers/planners. They do not become architecture authority or execution authority. Source-admission checks may initially be contract/manual evidence until executable tooling is added; the manifest must state evidence honestly rather than pretending automation exists.

## Final rule

**MANIFEST v6 BINDS SOURCE ADMISSION, REAL DONOR GIT HISTORY WHEN DEEP-FORK MODE APPLIES, LEGACY ACCOUNTING, EXACT-SHA CHRONICA SEMANTIC EVIDENCE, PROPOSAL-ONLY DOC SYNC, EXECUTABLE LOGISTICS CHECKS, AND DEPLOYMENT/RUNTIME IDENTITY INTO ONE CONTROLLED DISCOVERY SURFACE WITHOUT TURNING THAT MANIFEST INTO TRUST, TRUTH OR AUTHORITY.**
