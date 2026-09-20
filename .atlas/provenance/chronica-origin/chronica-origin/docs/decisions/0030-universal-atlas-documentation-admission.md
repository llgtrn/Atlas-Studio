---
id: adr.0030
type: decision
status: accepted
canonical: true
---
# ADR-0030 — Universal Atlas Documentation Admission

## Status

Accepted.

## Context

Atlas Systemizer must guide coding across Chronica and the registered Development Cell fleet without inventing implementation intent from incomplete repositories. Chronica already established North-Star, architecture-owner, blueprint, contract, verification and reconciliation discipline.

## Decision

atlas.docs.v1 becomes a zero-exception coding-admission contract for every Atlas-managed repository. Every repository uses the same mandatory control paths and headings. Additional domain documents are allowed but follow the same metadata/canonicality/reference rules.

Atlas may inspect and analyze an incomplete repository, but it must not prepare coding work until RepoGate and DocsGate are both READY, the selected repository has an exact base SHA, and the coding session targets exactly one repository.

Chronica is the semantic source standard for this documentation discipline. Atlas Systemizer owns the executable validator and fleet-wide enforcement.

## Consequences

Code-first prototypes managed by Atlas are forbidden. Missing docs become an explicit engineering blocker rather than an invitation for the coding model to guess intent. Development Cells now document North Star, boundaries, blueprint and hard contracts even though they contain no product code.

## Alternatives / Supersession

The previous flexible/archetype-specific documentation approach is superseded for Atlas-managed repositories. Repository implementation roots may still reflect responsibility, but documentation control structure may not vary.

## Implementation or Verification Impact

Atlas docs audit must hard-fail incomplete control docs or maintained Markdown governance violations. Fleet work preparation must require DocsGate READY. CI and repository bootstrap must install the same documentation skeleton everywhere.
