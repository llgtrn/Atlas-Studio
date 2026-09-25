---
id: atlas.contract.census-certificate
type: contract
status: active
canonical: true
---
# Census Certificate Contract

## Purpose

CensusCertificate is the machine-verifiable closure record for a pinned corpus, revision set and Genome identity.

It is not a prose summary and it does not manufacture completeness. It records whether the prerequisite accounting, semantic coverage, reconciliation and fixed-point conditions have actually been satisfied.

The machine schema is:

**.atlas/schemas/census-certificate.schema.json**

## Lifecycle

~~~text
DRAFT
  ↓
CENSUSED
  ↓
RECONCILED
  ↓
CLOSED
  ↓
SEALED
~~~

State transitions are monotonic for one certificate lineage. If inputs, Genome, extractor versions or required policy change, Atlas creates a new certificate identity rather than silently reinterpreting the old one.

## Required identity

A certificate identifies:

- certificate schema/version;
- certificate identity;
- corpus identity and corpus root hash;
- exact repository revision set;
- admitted dependency-resolution context set;
- exact Genome schema/hash;
- evidence/provenance root;
- normalized semantic root when available;
- Atlas root hash when SEALED.

## Inventory accounting

The certificate records:

- admitted artifact total;
- accounted artifact total;
- disposition counts;
- whether accounting is closed.

SEALED requires admitted total = accounted total.

## Dependency closure accounting

The certificate records, across all admitted resolution contexts:

- resolution context total;
- resolved dependency node total;
- dependency edge total;
- source-backed dependency total;
- source-backed dependency nodes inventoried/censused;
- explicit non-source terminal total;
- unresolved dependency total;
- whether dependency closure reached fixed point;
- deterministic dependency closure root/hash when available.

A dependency node is not omitted because it is outside the root repository.

SEALED requires dependency closure to be closed. Policy may allow explicit UNKNOWN/UNSUPPORTED terminal states, but silent unresolved dependency edges are forbidden.

## Semantic accounting

The certificate records:

- semantic facts total;
- obligation total and accounted total;
- per-dimension coverage status;
- unknown total;
- unsupported total;
- ignored total;
- conflict total;
- unresolved dynamic edge total;
- binding gap total.

Coverage values use the canonical EpistemicStatus vocabulary from SEMANTIC-FACTS.md. Coverage is not a free-form second status language.

## Normalization accounting

The certificate records:

- input record total;
- normalized record total;
- exact duplicates merged;
- equivalence classes formed;
- conflict candidates;
- provenance completeness;
- deterministic normalized root/hash when available.

Normalization closure must satisfy NORMALIZATION.md.

## Reconciliation and fixed point

The certificate records:

- reconciliation state;
- fixed-point iteration count;
- whether convergence was reached;
- delta remaining at exit;
- independent extraction pass count;
- pass agreement state;
- unresolved cross-pass conflict total.

A fixed-point claim is invalid if convergence is false.

## Blockers

Every policy-blocking unresolved condition is emitted as a typed blocker code. A CLOSED or SEALED certificate cannot hide blockers in logs.

## Seal eligibility

A certificate may be SEALED only when all Genome-required gates pass.

At minimum:

~~~text
inventory_closed
AND dependency_closure_closed
AND semantic_obligations_accounted
AND normalization_closed
AND reconciliation_complete
AND fixed_point_converged
AND provenance_complete
AND policy_forbidden_unknowns = 0
AND policy_forbidden_unsupported = 0
AND policy_forbidden_conflicts = 0
AND blockers = []
~~~

Some scopes may permit UNKNOWN, UNSUPPORTED or IGNORED. The permission is policy/scoped and must itself be recorded; those states never disappear.

## Determinism

The semantic content of a certificate is deterministic for pinned inputs and policies. Wall-clock metadata, transport metadata or UI display state MUST NOT participate in certificate identity.

## Production rule

Only policy-eligible SEALED certificates may authorize production Atlas publication/materialization where Genome requires sealing.

CensusCertificate records closure; it does not itself grant execution authority.

## Implementation status

G59 (ADR 0025): `atlas_core::certificate::certify` and `atlas-systemizer census certificate` produce v2 certificates, schema-conformant field for field and test-enforced. States follow typed blockers.

Current limits:
- replay fixed point: census passes are compared by census digest;
- independent passes: counted per (artifact, dimension). Since G75, MULTI_ENGINE_RECONCILIATION_ABSENT names every evaluated dimension no artifact has two engines for. A second engine on one dimension reconciles that dimension only; CALL has two engines since G75 (ADR 0031);
- SEALED: impossible until a sealed `.atlas` root exists. Since G64, `census certificate --atlas F` verifies an unsealed census container against the fresh census and records its root identity, replacing ATLAS_ROOT_ABSENT with ATLAS_ROOT_UNSEALED (ADR 0027).

The Atlas self-scope certificate is CENSUSED (see ADR 0025 for its blockers).

Blockers closed since then, each proven by self-recensus:
- G60: WORKSPACE_MEMBER_OUTSIDE_INVENTORY (`apps/cli` brought into scope);
- G61: ARTIFACTS_WITHOUT_SOURCE_FRONTEND. The html and css fixtures now have registered frontends. The three BLAKE3 vector files are classified as `rust-include-fragment` through their observed `include!`. None of these languages has an extractor, so their semantics are explicitly UNSUPPORTED, not UNKNOWN.
- G62: PROVENANCE_INCOMPLETE. Census-level facts (artifact disposition and language) and every ADL-derived fact now carry the census revision. One helper, `CensusInputs::census`, builds the census for every entry point. Unsupported-language obligations name the dispatch that asserted them (`atlas.extraction.unsupported-language`). An independent engine is an extractor that *evaluated* an obligation, so an UNSUPPORTED assertion never counts as a pass. The self-scope certificate is now RECONCILED.
