---
id: ops-production-source-admission-goal
type: reference
status: active
canonical: true
---
# Ops External Source Admission Goal

This is the **single execution prompt for agents deciding whether and how external code may enter an Ops/Chronica trust boundary**.

It runs before donor refoundation, dependency adoption, provider adaptation or reference-code reuse.

Read and obey:

```text
docs/ops_production/contracts/SOURCE-ADMISSION.md
docs/ops_production/contracts/OPS-MANIFEST.md
docs/ops_production/contracts/SEMANTIC-CONVERGENCE.md
```

Core invariant:

```text
ExternalCode != TrustedCode != CanonicalSemantics
SourceAdmission(code) != SemanticAdmission(meaning) != ExecutionAuthority(effect)
```

## Execution loop

```text
READ REAL REPO / TASK STATE
-> IDENTIFY EXTERNAL SOURCE
-> CLASSIFY ACQUISITION MODE
-> PIN EXACT SOURCE IDENTITY
-> VERIFY PROVENANCE / INTEGRITY
-> VERIFY RIGHTS + LICENSE COMPATIBILITY
-> QUARANTINE SECURITY / BUILD HOOKS
-> SCAN SECRETS / PII / DATA CONTAMINATION
-> INVENTORY SUPPLY CHAIN / SBOM
-> BUILD / TEST IN BOUNDED ENVIRONMENT WHEN SAFE
-> CLASSIFY UPSTREAM LIFECYCLE
-> SELECT ADMISSION DISPOSITION
-> RECORD MANIFEST / EVIDENCE
-> ROUTE TO MODE-SPECIFIC NEXT STEP
```

Do not start architectural assimilation before this gate is dispositioned.

## 1. Inspect real state

Run:

```text
pwd
git remote -v
git branch --show-current
git rev-parse HEAD
git status --short
git log --oneline -20
```

Preserve concurrent/later work. Never reset/force-push/discard unrelated changes.

## 2. Classify the source mode first

Choose exactly one primary mode:

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

Do not default everything to `DEEP_FORK`.

Use `DEEP_FORK` only when Ops intends to refound/own a substantial external product/source tree and later burn donor ownership down.

Use `PACKAGE_DEPENDENCY` for ordinary ecosystem packages.

Use `EXTERNAL_PROVIDER` when the implementation remains external and Chronica consumes it through an adapter/binding.

Use `REFERENCE_IMPLEMENTATION` when code is studied but not admitted as production source.

Use `SPEC_ONLY` when only a versioned protocol/API/specification is relied upon.

## 3. Pin exact source identity

Record as applicable:

```text
SOURCE_KIND
SOURCE_URL_OR_REGISTRY
SOURCE_OWNER_OR_PUBLISHER
EXACT_COMMIT_TAG_VERSION
CONTENT_DIGEST_OR_LOCKFILE_IDENTITY
SUBMODULES
LFS_BOUNDARIES
GENERATED_OR_BINARY_ARTIFACTS
```

Do not accept floating `main`, `latest`, unbounded package ranges or mutable container tags as production admission identity.

If exact identity cannot be established:

```text
STOP: SOURCE_IDENTITY_UNRESOLVED
```

## 4. Verify integrity and provenance

Verify what is actually available:

```text
commit/tag identity
registry checksum/integrity
release checksum
signature/signed tag
container digest
lockfile resolution
SBOM identity
```

Classify:

```text
INTEGRITY_VERIFIED
INTEGRITY_PARTIALLY_VERIFIED
INTEGRITY_UNVERIFIED
INTEGRITY_FAILED
```

Do not claim verified authenticity when signatures/provenance do not prove it.

`INTEGRITY_FAILED` blocks admission.

## 5. Verify rights/license compatibility for the chosen mode

Inspect as applicable:

```text
primary license
transitive licenses
NOTICE requirements
copyleft/source-offer obligations
network copyleft
patent clauses
trademark restrictions
redistribution/modification/sublicensing/white-label rights
static/dynamic linking implications
territorial/temporal/field-of-use limits
acquisition/assignment evidence
third-party proprietary components
```

Use:

```text
RIGHTS_VERIFIED_FOR_MODE
RIGHTS_PARTIALLY_VERIFIED
THIRD_PARTY_OBLIGATIONS_REMAIN
RIGHTS_SCOPE_UNRESOLVED
RIGHTS_INCOMPATIBLE
```

Acquisition of a company or source tree does not automatically eliminate third-party obligations.

`RIGHTS_INCOMPATIBLE` blocks the proposed mode.

## 6. Quarantine security before execution

Treat new external code as untrusted.

Initial inspection/build must not receive:

```text
production credentials
customer/tenant secrets
production databases
production signing keys
unrestricted cloud credentials
privileged host mounts without justification
```

Audit executable surfaces:

```text
install/postinstall/preinstall scripts
build.rs
Makefile/task runners
Git hooks
CI workflows
Dockerfiles/entrypoints
shell/PowerShell scripts
codegen
native extensions / FFI
binary blobs
prebuilt executables
runtime/build-time downloads
self-update logic
telemetry/network egress
```

Do not run unknown install/build hooks with secrets merely to test whether the source works.

## 7. Scan secret/data contamination

Inspect for:

```text
.env files
API keys/tokens/passwords
private keys/certs
cloud credentials
production DB URLs
customer/tenant exports
PII/payment data
confidential agreements embedded in source
private screenshots/logs/fixtures
unlicensed/proprietary datasets
```

Disposition material findings:

```text
REMOVE_FROM_ADMITTED_TREE
ROTATE_AND_INVALIDATE
REDACT_FIXTURE
MOVE_TO_SECURE_PROVENANCE_STORE
RETAIN_WITH_EXPLICIT_AUTHORIZED_SCOPE
REJECT_SOURCE
```

Credential safety overrides preservation of leaked secrets for historical fidelity.

## 8. Supply-chain / SBOM audit

Account for material:

```text
direct dependencies
transitive dependencies where tooling permits
registries/sources
vendored code
native libraries
container bases
runtime downloads
build-time downloads
plugins/extensions
```

Prefer generated lockfile/SBOM evidence.

Classify material vulnerability state:

```text
NO_KNOWN_BLOCKING_FINDING
PATCH_REQUIRED
MITIGATION_ACCEPTED_WITH_EVIDENCE
UNRESOLVED_SECURITY_RISK
```

No scan is not equivalent to `NO_KNOWN_BLOCKING_FINDING`.

## 9. Bounded baseline build/test

Only after safe quarantine, build/test the pinned source when feasible.

Record:

```text
BASELINE_BUILD_RESULT
BASELINE_TEST_RESULT
ENVIRONMENT
NETWORK_ACCESS
SECRETS_AVAILABLE = none | bounded-test-only
FAILURE_SUMMARY
```

Do not promote a failed/unknown baseline to passed.

## 10. Classify upstream lifecycle

Choose exactly one:

```text
PIN_AND_DIVERGE
TRACK_UPSTREAM
SECURITY_PATCH_ONLY
PROVIDER_MANAGED
ABSORB_AND_RETIRE
REFERENCE_ONLY
```

This decision is required so future security fixes/upstream changes have an explicit policy.

## 11. Select admission disposition

Choose exactly one:

```text
ADMIT_DEEP_FORK
ADMIT_VENDORED_SOURCE
ADMIT_PACKAGE_DEPENDENCY
ADMIT_SUBMODULE
ADMIT_EXTERNAL_PROVIDER
ADMIT_REFERENCE_ONLY
ADMIT_SPEC_ONLY
QUARANTINE_PENDING_EVIDENCE
REJECT
```

Blocking conditions include:

```text
SOURCE_IDENTITY_UNRESOLVED
INTEGRITY_FAILED
RIGHTS_INCOMPATIBLE
UNRESOLVED_SECURITY_RISK
UNREMEDIATED_SECRET_EXPOSURE
UNAUTHORIZED_DATASET
```

Do not bypass a blocked admission merely because the code is useful.

## 12. Record evidence truthfully

Update `ops.manifest.yaml` using Manifest v6 source-admission fields when the Ops repository uses this contract.

Do not place secrets or confidential agreement text in the manifest.

Record evidence refs rather than unsupported claims.

## 13. Route by mode

For `ADMIT_DEEP_FORK`:

```text
read docs/ops_production/donor/OSS-DONOR-CLONE-REFACTOR.md
clone/fetch complete real donor
pin exact SHA/tag
import real donor baseline into Ops Git
push baseline
record provenance
then run docs/ops_production/EXECUTE-AUDIT-GOAL.md
```

For `ADMIT_VENDORED_SOURCE`:

```text
record bounded source subtree + checksum/revision
preserve license/provenance
place by actual responsibility
maintain update/security policy
```

For `ADMIT_PACKAGE_DEPENDENCY`:

```text
pin via lockfile/exact version
retain license/SBOM/security evidence
keep provider/implementation details behind semantic boundaries where needed
```

For `ADMIT_SUBMODULE`:

```text
pin exact commit
record upstream policy
avoid floating refs
```

For `ADMIT_EXTERNAL_PROVIDER`:

```text
keep provider external
implement crates/adapter + bindings boundary
map provider vocabulary into Chronica semantics
never grant provider execution authority
```

For `ADMIT_REFERENCE_ONLY`:

```text
use for comparison/parity/evidence
no production source dependency
```

For `ADMIT_SPEC_ONLY`:

```text
pin spec/protocol version
implement through adapter
```

## 14. Semantic convergence still applies after admission

Admission of code does not admit its domain model as canonical Chronica meaning.

After source admission, compare material semantics against current Chronica at an exact SHA using `SEMANTIC-CONVERGENCE.md`.

Keep:

```text
provider DTO != core semantic
provider action != authority
package API != canonical capability
source success != reconciled effect
```

## 15. End-of-run report

Report facts only:

```text
OPS_HEAD
SOURCE_ID
SOURCE_MODE
EXACT_REVISION_OR_VERSION
INTEGRITY_STATUS
RIGHTS_STATUS
SECURITY_STATUS
SECRET_DATA_STATUS
SBOM / SUPPLY_CHAIN_EVIDENCE
BASELINE_BUILD_TEST_RESULT
UPSTREAM_POLICY
ADMISSION_DISPOSITION
EVIDENCE_REFS
NEXT_MODE_SPECIFIC_STEP
BLOCKERS
```

Do not say `ADMITTED` when evidence only supports quarantine.

## Final rule

**CLASSIFY BEFORE CLONING EVERYTHING. PIN BEFORE TRUST. QUARANTINE BEFORE EXECUTION. VERIFY RIGHTS, INTEGRITY, SUPPLY CHAIN, SECRETS/DATA AND SECURITY BEFORE PRODUCTION USE. ADMIT SOURCE CODE SEPARATELY FROM SEMANTIC MEANING AND EXECUTION AUTHORITY. THEN ROUTE THE SOURCE INTO DEEP-FORK REFOUNDATION, PACKAGE USE, PROVIDER ADAPTATION, REFERENCE-ONLY STUDY OR REJECTION.**
