---
id: ops-production-readme
type: reference
status: active
canonical: true
---
# Atlas Ops Production

`ops_production` supports building, validating and operating Atlas-Systemizer itself.

It is not a product domain, not an organism layer, and not a second architecture universe. Its job is to make Atlas production work auditable:

```text
Atlas source
-> repository systemization
-> donor provenance and license checks
-> production-readiness evidence
-> bounded work preparation
-> verification reports
```

Canonical runtime ownership remains:

```text
core/             product-neutral semantics
runtime/          orchestration and work preparation
adapter/          external mechanics translated into Atlas facts
ops_production/   Atlas support/readiness kernels
apps/ui/          TypeScript UI projection
graph/            declarative graph definitions
bindings/         declarative binding definitions
```

The legacy `organism` lane is intentionally not part of Atlas-Systemizer. AI-assisted analysis belongs behind explicit provider/adaptation boundaries and remains proposal/inference until runtime validation converts it into evidence.

## OSS Donors

Donor source belongs only under:

```text
.atlas/temporary/donors/
```

License copies belong under:

```text
.atlas/licenses/
```

Provenance belongs under:

```text
.atlas/provenance/
```

Donor code is reference material and evidence. It is not a permanent runtime owner. Native promotion requires census, license review, target mapping, Atlas-owned implementation, tests and evidence.

## Evidence

Ops production work should emit or update:

```text
.atlas/evidence/
.atlas/reports/
.atlas/provenance/
.atlas/references/donor-corpus.toml
```

Claims without evidence stay pending. Legacy files with no remaining study value should be extincted rather than kept as working-tree noise.
