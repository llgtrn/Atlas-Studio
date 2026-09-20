---
id: ops-production-donor-readme
type: reference
status: active
canonical: true
---
# Ops Donor Intake Router

This directory applies to external sources whose source-admission disposition is:

```text
ADMIT_DEEP_FORK
```

or another explicitly approved mode that intentionally imports a substantial source tree into Ops ownership.

Do **not** apply full donor-clone rules automatically to:

```text
PACKAGE_DEPENDENCY
EXTERNAL_PROVIDER
REFERENCE_IMPLEMENTATION
SPEC_ONLY
```

Those modes are governed first by:

```text
docs/ops_production/SOURCE-ADMISSION-GOAL.md
docs/ops_production/contracts/SOURCE-ADMISSION.md
```

For an admitted deep fork, continue with:

```text
docs/ops_production/donor/OSS-DONOR-CLONE-REFACTOR.md
docs/ops_production/donor/DONOR-CENSUS-PROVENANCE.md
docs/ops_production/EXECUTE-AUDIT-GOAL.md
```

Core distinction:

```text
external source admission
!=
donor semantic convergence
!=
execution authority
```

The full donor baseline is migration/provenance evidence for a deep fork. It is not a universal requirement to clone every package, SDK or provider repository into Ops.
