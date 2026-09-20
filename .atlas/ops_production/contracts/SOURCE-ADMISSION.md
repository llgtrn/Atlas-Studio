# Ops External Source Admission Contract

Status: **ACTIVE OPS PRODUCTION CONTRACT — v1**

This contract governs how external code, repositories, packages, SDKs, provider clients, reference implementations and acquired/deep-forked products are admitted into Chronica Ops work.

The core invariant is:

```text
ExternalCode != TrustedCode != CanonicalSemantics
```

External code is initially an untrusted resource with provenance, rights, integrity, supply-chain and semantic-admission requirements. It does not become trusted runtime code, canonical Chronica meaning or execution authority merely because it compiles, has a permissive license, comes from a known vendor, or has been acquired.

The admission sequence is:

```text
DISCOVER SOURCE
-> CLASSIFY ACQUISITION MODE
-> PIN EXACT VERSION / REVISION
-> VERIFY INTEGRITY + PROVENANCE
-> VERIFY RIGHTS / LICENSE COMPATIBILITY
-> QUARANTINE SECURITY + SECRETS / DATA SCAN
-> INVENTORY SUPPLY-CHAIN / BUILD EXECUTION SURFACES
-> BUILD / TEST IN A BOUNDED ENVIRONMENT
-> CLASSIFY UPSTREAM LIFECYCLE
-> ADMIT / ADAPT / ABSORB / REJECT
-> ONLY THEN ENTER DONOR REFOUNDATION OR NORMAL OPS IMPLEMENTATION
```

A source that fails admission must not be silently promoted into trusted Ops or Chronica runtime code.

## 1. Acquisition mode is mandatory

Every external source must be classified before intake as exactly one primary mode:

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

### DEEP_FORK

Use when Chronica/Ops intends to own and refound a substantial product or source tree.

Required path:

```text
full repository clone/fetch
-> exact SHA/tag
-> real Ops Git baseline
-> complete source/data/UI/docs/brand census
-> legacy burn-down
-> semantic convergence
-> eventual native absorption
```

The donor full-repository contracts apply.

### VENDORED_SOURCE

Use when a bounded source subtree/library is intentionally copied into controlled source ownership.

Required evidence:

```text
source origin
exact revision/archive checksum
included files
license/notice obligations
update policy
security review
local modifications
```

Do not disguise a whole donor product as a vendored library to bypass full-donor intake.

### PACKAGE_DEPENDENCY

Use for ecosystem packages resolved through Cargo/npm/pnpm/PyPI/etc.

Do not import the entire upstream repository merely to satisfy donor-baseline rules.

Required evidence includes:

```text
package name
resolved exact version
lockfile identity
registry/source
license compatibility
transitive dependency/SBOM visibility
known vulnerability review where material
execution hooks/build scripts
upstream maintenance policy
```

### SUBMODULE

Use only when retaining upstream Git identity is operationally useful and the trust/update boundary is explicit.

Pin exact commit. Floating branches are not admission evidence.

### EXTERNAL_PROVIDER

Use when the external system remains independently operated and Chronica consumes it through an adapter/binding.

Do not clone/refound the provider merely because an SDK exists.

Provider semantics remain boundary-local and map into canonical Chronica semantics before crossing application/core boundaries.

### REFERENCE_IMPLEMENTATION

Use when code is studied for behavior, algorithms, compatibility or migration evidence but is not admitted directly as production source.

Reference code must not be copied into production merely because it was inspected.

### SPEC_ONLY

Use when only an external protocol/specification/API contract is relied upon.

Record exact spec/version/source and implement behind an adapter.

### REJECTED_SOURCE

Use when provenance, integrity, rights, supply-chain risk, secret/data contamination or other admission requirements cannot be made acceptable.

## 2. Fail-closed source identity and provenance

Before execution or mutation of imported code, record as applicable:

```text
SOURCE_KIND
SOURCE_URL_OR_REGISTRY
SOURCE_OWNER_OR_PUBLISHER
EXACT_COMMIT_TAG_VERSION
CONTENT_DIGEST_OR_LOCKFILE_IDENTITY
FETCHED_AT
SUBMODULES
LFS_BOUNDARIES
GENERATED_OR_BINARY_ARTIFACTS
PROVENANCE_EVIDENCE
```

Floating `main`, `latest`, unpinned container tags or package ranges are not sufficient admission identity for a production dependency.

If source identity cannot be pinned:

```text
STOP: SOURCE_IDENTITY_UNRESOLVED
```

## 3. Integrity verification

Where available, verify:

```text
Git commit/tag identity
registry integrity/checksum
release checksum
signed tag/release/signature
package lockfile resolution
container digest
SBOM identity
```

Absence of signatures is not automatically rejection, but it must not be represented as verified authenticity.

Use bounded states:

```text
INTEGRITY_VERIFIED
INTEGRITY_PARTIALLY_VERIFIED
INTEGRITY_UNVERIFIED
INTEGRITY_FAILED
```

`INTEGRITY_FAILED` blocks admission.

## 4. Rights and license compatibility gate

Rights verification is source-mode-aware.

Assess as applicable:

```text
primary license
transitive licenses
copyright notices
NOTICE obligations
copyleft/source-offer obligations
network copyleft where relevant
patent clauses
trademark restrictions
redistribution rights
modification rights
sublicensing rights
white-label rights
static/dynamic linking implications
territorial/temporal/field-of-use limits
acquisition/assignment evidence
third-party proprietary components
```

A company/source-code acquisition does not automatically extinguish third-party obligations.

Use bounded states:

```text
RIGHTS_VERIFIED_FOR_MODE
RIGHTS_PARTIALLY_VERIFIED
THIRD_PARTY_OBLIGATIONS_REMAIN
RIGHTS_SCOPE_UNRESOLVED
RIGHTS_INCOMPATIBLE
```

`RIGHTS_INCOMPATIBLE` blocks admission for the proposed mode.

## 5. Security quarantine before trust

New external code must be treated as untrusted until security intake is complete.

Initial inspection/build should occur in a bounded environment with:

```text
no production credentials
no tenant/customer secrets
no production databases
no production signing keys
no unrestricted cloud credentials
minimum necessary network access
no privileged host mounts unless explicitly justified
```

Audit execution-capable surfaces including:

```text
install/postinstall/preinstall scripts
build.rs
Makefile/task runners
Git hooks
GitHub Actions/CI definitions
Dockerfiles/container entrypoints
shell/PowerShell scripts
code generation
native extensions
FFI
binary blobs
prebuilt executables
package lifecycle hooks
download-at-build behavior
self-update behavior
telemetry/network egress
```

Do not run unknown scripts with secrets merely to see whether the project builds.

## 6. Secrets, PII and data-contamination gate

Before donor/vendor source becomes an accepted baseline, scan for material contamination such as:

```text
.env files
API keys/tokens/passwords
private keys/certificates
cloud credentials
production connection strings
customer/tenant exports
PII
payment data
confidential agreements embedded in source
private screenshots/logs/fixtures
proprietary datasets without admitted rights
```

Disposition each material finding:

```text
REMOVE_FROM_ADMITTED_TREE
ROTATE_AND_INVALIDATE
REDACT_FIXTURE
MOVE_TO_SECURE_PROVENANCE_STORE
RETAIN_WITH_EXPLICIT_AUTHORIZED_SCOPE
REJECT_SOURCE
```

Do not preserve leaked credentials merely for historical fidelity.

Git history containing real leaked secrets requires explicit remediation/rotation judgment; provenance preservation does not override credential safety.

## 7. Supply-chain inventory / SBOM gate

For admitted production source or dependencies, account for the material supply chain:

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

Prefer machine-generated SBOM/lockfile evidence where available.

Record known material vulnerability disposition as:

```text
NO_KNOWN_BLOCKING_FINDING
PATCH_REQUIRED
MITIGATION_ACCEPTED_WITH_EVIDENCE
UNRESOLVED_SECURITY_RISK
```

Do not make `NO_KNOWN_BLOCKING_FINDING` mean "no scan was performed".

## 8. Baseline build/test is bounded execution evidence

After source admission reaches a safe quarantine state, build/test the exact pinned source when feasible.

Record:

```text
BASELINE_BUILD_RESULT
BASELINE_TEST_RESULT
ENVIRONMENT
NETWORK_ACCESS
SECRETS_AVAILABLE = none | bounded-test-only
FAILURE_SUMMARY
```

A failing donor build does not automatically reject a source if the failure is understood and bounded, but it must not be recorded as passed.

## 9. Upstream lifecycle classification

Every admitted external source must have exactly one primary upstream policy:

```text
PIN_AND_DIVERGE
TRACK_UPSTREAM
SECURITY_PATCH_ONLY
PROVIDER_MANAGED
ABSORB_AND_RETIRE
REFERENCE_ONLY
```

### PIN_AND_DIVERGE

Chronica/Ops owns forward development from a pinned source; upstream updates are deliberate imports, not automatic merges.

### TRACK_UPSTREAM

Periodic upstream updates are expected. Define update cadence and conflict/parity policy.

### SECURITY_PATCH_ONLY

Normal feature divergence is local, but upstream security fixes are monitored and selectively imported.

### PROVIDER_MANAGED

The external provider/vendor owns implementation lifecycle; Chronica maintains adapter compatibility.

### ABSORB_AND_RETIRE

The source is a temporary donor universe and terminal ownership converges into Chronica.

### REFERENCE_ONLY

No production source dependency remains.

## 10. Admission decision

After gates, select exactly one disposition:

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

Admission answers whether the external source may enter the chosen trust/ownership boundary.

It does NOT answer whether its domain semantics are canonical.

After admission:

```text
semantic convergence still required
runtime authority still required
provider isolation still required
canonical-state rules still required
```

## 11. Mode-specific next step

```text
ADMIT_DEEP_FORK
    -> OSS-DONOR-CLONE-REFACTOR.md
    -> full donor Ops baseline
    -> EXECUTE-AUDIT-GOAL.md

ADMIT_VENDORED_SOURCE
    -> bounded vendored ownership + provenance
    -> adapter/core/runtime placement by responsibility

ADMIT_PACKAGE_DEPENDENCY
    -> lockfile + license/security policy
    -> wrap behind semantic boundary where implementation-specific

ADMIT_SUBMODULE
    -> pinned submodule + update policy

ADMIT_EXTERNAL_PROVIDER
    -> crates/adapter / bindings
    -> no second canonical universe

ADMIT_REFERENCE_ONLY
    -> analysis/parity evidence only
    -> no production dependency

ADMIT_SPEC_ONLY
    -> versioned protocol/spec reference
    -> adapter implementation
```

## 12. Semantic admission is separate from source admission

Keep this distinction:

```text
SourceAdmission(code)
!=
SemanticAdmission(meaning)
!=
ExecutionAuthority(effect)
```

A secure, legally usable library may still express semantics that conflict with Chronica.

A semantically useful donor implementation may still fail source-admission security or rights gates.

An admitted implementation still receives no ACT authority merely because it is linked into the process.

## 13. Machine-readable evidence

Ops repositories using external code should expose source-admission evidence in `ops.manifest.yaml` or an explicitly referenced rebuildable/provenance artifact.

Recommended manifest shape:

```yaml
source_admission:
  contract_version: 1
  sources:
    - id: upstream-helpdesk
      mode: DEEP_FORK
      source: https://github.com/example/helpdesk
      revision: <exact-sha>
      integrity_status: INTEGRITY_VERIFIED
      rights_status: RIGHTS_VERIFIED_FOR_MODE
      security_status: NO_KNOWN_BLOCKING_FINDING
      secret_data_status: CLEAN_OR_REMEDIATED
      sbom_ref: provenance/sbom/upstream-helpdesk.cdx.json
      upstream_policy: ABSORB_AND_RETIRE
      admission: ADMIT_DEEP_FORK
      evidence_refs:
        - provenance/source/upstream-helpdesk.yaml
```

Do not put credentials, confidential agreement text or sensitive customer data in this manifest.

## 14. Release / refactor blocker states

The following block production-source admission or the claimed acquisition mode:

```text
SOURCE_IDENTITY_UNRESOLVED
INTEGRITY_FAILED
RIGHTS_INCOMPATIBLE
UNRESOLVED_SECURITY_RISK
UNREMEDIATED_SECRET_EXPOSURE
UNAUTHORIZED_DATASET
```

The following require explicit bounded disposition rather than silent success:

```text
INTEGRITY_UNVERIFIED
RIGHTS_SCOPE_UNRESOLVED
RIGHTS_PARTIALLY_VERIFIED
PATCH_REQUIRED
QUARANTINE_PENDING_EVIDENCE
```

## 15. Anti-patterns

Forbidden without explicit evidence-backed exception:

```text
clone arbitrary repository -> run install with production secrets
package name exists -> trust latest version
acquired company -> assume all embedded dependencies are owned
permissive top-level license -> ignore transitive obligations
known vendor -> skip integrity pinning
full donor contract applied to every small package dependency
package dependency treated as canonical domain semantics
SDK DTOs leaking into core/domain types
external provider action granting runtime authority
reference implementation copied into production without admission
binary/generated blobs accepted without provenance/accounting
```

## 16. Self-audit

Before admitting external source answer:

```text
What acquisition mode is this?
What exact source/version/revision is admitted?
How was integrity checked?
What rights/license scope permits this mode?
What third-party obligations remain?
Was code inspected/built in a bounded environment?
Were lifecycle/build hooks reviewed?
Were secrets/PII/proprietary datasets scanned and dispositioned?
What supply-chain/SBOM evidence exists?
What known security findings remain?
What is the upstream lifecycle policy?
Does this source stay external, become a dependency, or enter deep-fork assimilation?
What semantic boundary prevents source vocabulary from becoming canonical by accident?
```

If these cannot be answered, keep the source quarantined.

## Final rule

> **Classify external code before intake. Pin it. Verify provenance, integrity, rights, supply chain, secrets/data and execution risk in quarantine. Choose the correct ownership mode instead of deep-forking everything. Only after source admission may code enter donor refoundation, dependency use or provider adaptation. External code never becomes trusted code, canonical semantics or execution authority by declaration alone.**
