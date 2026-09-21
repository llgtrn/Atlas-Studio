---
id: atlas.contract.census-certificate
type: contract
status: active
canonical: true
---
# Census Certificate Contract

CensusCertificate is the machine-checkable closure artifact for a pinned corpus, revision set, Genome, and census policy.

It proves accounting and closure conditions. It does not claim that every implementation is correct.

Machine schema:

~~~text
.atlas/contracts/schema/census-certificate.v1.schema.json
~~~

The Rust contract type is owned by core and must remain schema-compatible with the machine contract.

## Required identity

Every certificate records:

- schema version;
- certificate state;
- corpus identity;
- revision set;
- Genome hash;
- policy/profile identity when applicable;
- Atlas root hash when sealed.

## Required accounting

Every certificate records:

- inventory total and accounted total;
- semantic coverage by obligation class;
- unresolved artifact count;
- unsupported artifact count;
- dynamic edge count;
- binding gap count;
- conflict count;
- fixed-point iteration count;
- independent-pass agreement summary.

Counts must be derived from admitted records, not manually asserted.

## State machine

~~~text
DRAFT
→ CENSUSED
→ RECONCILED
→ CLOSED
→ SEALED
~~~

State transitions are monotonic for one certificate lineage. A changed corpus, revision set, Genome, or relevant policy creates a new certificate identity.

## Seal eligibility

SEALED requires, at minimum:

- inventory accounting closed;
- required semantic obligations satisfy policy;
- reconciliation completed;
- required conflict policy satisfied;
- required UNKNOWN limits satisfied;
- fixed point reached;
- Atlas root hash present;
- certificate content is deterministic for pinned inputs.

Critical scopes may require UNKNOWN = 0.

UNSUPPORTED may remain only when the active policy explicitly permits it.

## Evidence

A certificate references evidence; it does not replace evidence.

Independent-pass agreement must identify which passes participated. A scalar confidence score alone is insufficient.

## Recovery and reproducibility

A certificate is rebuildable from canonical census inputs and evidence.

Deleting caches, graph projections, indexes, or UI state must not prevent certificate regeneration.
