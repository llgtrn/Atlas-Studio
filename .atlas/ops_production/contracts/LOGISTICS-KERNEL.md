# Ops Logistics Kernel

Status: **ACTIVE OPS PRODUCTION CONTRACT**

The Ops Logistics Kernel is the executable control surface that keeps a donor-derived Ops repository converging toward Chronica while refactor, semantic cleanup, absorption and deployment continue for months or years.

It does not create another canonical universe. The kernel observes Git/code/tests/manifests/deployment evidence, compares them with declared Ops/Chronica intent, produces plans/proposals, and fails closed on contradictions that can be checked mechanically.

The local Logistics Kernel is subordinate to the Fleet Control Plane. Fleet allocates donor ownership and dispatches cross-repository work; the local kernel proves one Ops repository. Local tooling may not override Fleet primary-absorber allocation.

## Core law

```text
INTENDED OPS / CHRONICA SEMANTICS
            !=
OBSERVED OPS IMPLEMENTATION
            !=
OBSERVED DEPLOYMENT
```

The Logistics Kernel keeps all three visible without promoting an observation into canonical truth.

```text
Ops manifest + docs        = declared intent / bounded metadata
Ops Reality Mirror         = observed repository implementation
Semantic Checker           = convergence evidence
Absorption Planner         = deterministic migration proposal
Docs Sync Proposal         = reviewed documentation change proposal
Deployment Evidence        = observed build/deploy/runtime identity
CI                         = executable regression gate
```

## Required executable mechanisms

Every production Ops must expose equivalent executable behavior for:

```text
1. Ops Reality Mirror
2. Semantic Convergence Checker
3. Absorption Planner
4. Docs Sync Proposal
5. CI gate
6. Deployment / runtime evidence
```

Chronica's reference implementation lives under `tools/ops-logistics/`. An Ops may vendor/synchronize these tools or implement equivalent commands in its own stack, but the machine-readable semantics and fail-closed behavior must remain compatible.

## 0. Fleet relationship

```text
Fleet Control
  -> allocates donor primary/reference roles
  -> expects one Mirror report per active Ops
  -> plans/distributes cross-repository work

Ops Logistics Kernel
  -> measures one Ops
  -> checks local semantic convergence
  -> produces local evidence/proposals
```

The same donor cannot become a deep local source merely because the local planner sees it. Deep intake requires this Ops to be the Fleet-assigned `PRIMARY_ABSORBER`.

## 1. Ops Reality Mirror

The mirror scans the real Ops Git/working tree and reports at least:

```text
current HEAD SHA
working-tree dirty state
real donor baseline existence
Ops docs-kernel completeness
legacy counters
semantic mappings
canonical-term references
donor/provider alias references
non-boundary alias leakage
required deployment environments
current deployment evidence
drift findings
```

It must never infer `production caller`, `production proven`, legal ownership or runtime success from simple text search.

The reference command is:

```bash
node tools/ops-logistics/reality.mjs --ops /path/to/Ops
```

Use `--strict` when error-class drift should fail the command.

## 2. Semantic Convergence Checker

The checker validates `docs/ops_production/contracts/SEMANTIC-CONVERGENCE.md` against real repository evidence.

It verifies:

```text
exact Chronica SHA
Chronica owner paths at that SHA
Chronica code refs at that SHA
Chronica test refs at that SHA
semantic fingerprints
match statuses
dispositions
canonical term selection
NO_EQUIVALENT_FOUND search evidence
extension gaps
alias retirement criteria
zero unexplained duplicate semantics
no donor/provider semantic leakage into application/core paths when alias-at-boundary is claimed
```

The reference command is:

```bash
node tools/ops-logistics/semantic-check.mjs \
  --ops /path/to/Ops \
  --chronica /path/to/Chronica
```

A same-name search is never sufficient. Exact-SHA implementation/test evidence is required for Chronica-mapped semantics.

## 3. Absorption Planner

The planner is generated from the manifest, Reality Mirror and Semantic Checker. It is a proposal, not execution authority.

It classifies each semantic into deterministic actions such as:

```text
REUSE_EXISTING_CHRONICA_IMPLEMENTATION
TRANSLATE_ALIAS_AND_RETIRE_INTERNAL_DUPLICATE
MIGRATE_CALLERS_TO_EXISTING_CHRONICA_SEMANTIC
EXTEND_CANONICAL_CHRONICA_OWNER_THEN_CONVERGE_OPS
KEEP_PRODUCT_LOCAL_WITH_EXPLICIT_CANONICAL_BOUNDARY
KEEP_PROVIDER_MECHANIC_BEHIND_ADAPTER
MIGRATE_CALLERS_THEN_DELETE_DUPLICATE_SEMANTIC
BLOCKED_PENDING_SEMANTIC_DECISION
```

It also carries blockers from donor baseline, docs governance, legacy debt and semantic drift.

Reference command:

```bash
node tools/ops-logistics/absorption-plan.mjs \
  --ops /path/to/Ops \
  --chronica /path/to/Chronica \
  --output /tmp/absorption-plan.json
```

`--strict` fails while the generated plan is blocked.

## 4. Docs Sync Proposal

Canonical docs must not automatically rewrite themselves from observed code.

```text
code changed
-> Reality Mirror observes
-> semantic diff evaluated
-> Docs Sync Proposal generated when canonical meaning may need change
-> review/admission
-> human/agent-authored canonical docs commit
```

The proposal generator emits, among other things:

```text
CANONICAL_SEMANTIC_EXTENSION
NEW_SEMANTIC_REVIEW
ORPHAN_IMPLEMENTATION
```

and always declares:

```text
PROPOSAL_ONLY_NEVER_AUTO_REWRITE_CANONICAL_DOCS
```

Reference command:

```bash
node tools/ops-logistics/docs-sync-proposal.mjs \
  --ops /path/to/Ops \
  --chronica /path/to/Chronica
```

`--require-clean` can be used by a release/admission gate when any pending docs-sync proposal must block.

## 5. CI gate

The repository must execute deterministic fixture tests for the logistics kernel and, where an Ops checkout is available, run the real semantic/reality checks against that repository.

The Chronica reference workflow is `.github/workflows/ops-logistics.yml`.

CI must prove at minimum:

```text
Reality Mirror sees a real Git baseline
semantic alias leakage is rejected
exact Chronica SHA refs resolve
Absorption Planner consumes the same observed state
Docs Sync Proposal never mutates docs
runtime evidence distinguishes current vs older Git SHA
scorecard reaches 100% only when all required mechanisms have evidence
```

A workflow file existing in Git is not CI proof. GitHub/runner evidence must show jobs actually started and completed.

## 6. Deployment / runtime evidence

Repository implementation and deployed runtime are separate realities.

The evidence recorder stores a rebuildable local/CI projection, by default:

```text
.chronica/deployment-evidence.json
```

This path must stay outside canonical source truth and must not contain secrets.

Each deployment observation includes:

```text
environment
gitSha
deploymentId
buildId when available
runtimeVersion when available
artifactDigest
healthStatus = PASS | FAIL | UNKNOWN
evidenceRefs
observedAt
```

Record:

```bash
node tools/ops-logistics/deployment-evidence.mjs record \
  --ops /path/to/Ops \
  --environment staging \
  --sha <exact-git-sha> \
  --deployment-id <id> \
  --build-id <id> \
  --artifact-digest <digest> \
  --health PASS \
  --evidence <bounded-reference>
```

Check:

```bash
node tools/ops-logistics/deployment-evidence.mjs check \
  --ops /path/to/Ops \
  --require-current
```

A runtime at an older repository commit is shown as `OLDER_REPO_COMMIT`, not silently treated as current.

## Manifest v5 integration

`ops.manifest.yaml` v5 adds:

```yaml
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
```

Semantic mappings remain under `semantic_convergence`. Legacy accounting remains under `legacy_burndown`. The kernel composes these surfaces rather than creating parallel truth files.

## Machine scorecard

The reference scorecard evaluates:

```text
OPS_REALITY_MIRROR
SEMANTIC_CONVERGENCE_CHECKER
ABSORPTION_PLANNER
DOCS_SYNC_PROPOSAL
DEPLOYMENT_RUNTIME_EVIDENCE
LOGISTICS_CONTRACT_VERSION
```

A production Ops is 6/6 only when all six are backed by executable evidence. The score is a regression rubric, not proof that all product behavior is correct.

Reference command:

```bash
node tools/ops-logistics/scorecard.mjs \
  --ops /path/to/Ops \
  --chronica /path/to/Chronica
```

## Security and sovereignty

The Logistics Kernel must not place secrets, confidential agreements, tenant payloads, authentication tokens or unrestricted Holding data into generated reports.

Deployment evidence uses bounded identifiers/digests/references. Semantic fingerprints describe meaning and code ownership, not private production data.

## Completion condition

Ops logistics reaches the intended state when:

```text
real donor baseline is auditable
Reality Mirror observes actual code continuously
semantic convergence is exact-SHA and code/test evidenced
absorption plan is deterministic and blockers explicit
docs changes are proposal-only until admitted
CI executes the logistics checks
staging/production runtime identity is evidence-linked to Git/build artifacts
legacy and duplicate semantics monotonically burn down
```

## Final rule

**OBSERVE REAL OPS CODE -> VERIFY SEMANTICS AGAINST EXACT CHRONICA CODE/TESTS -> PLAN ABSORPTION -> PROPOSE DOC CHANGES WITHOUT AUTO-REWRITING TRUTH -> PROVE IN CI -> LINK DEPLOYED RUNTIME BACK TO EXACT GIT/ARTIFACT EVIDENCE.**
