---
id: atlas.contract.admission-transaction
type: contract
status: active
canonical: true
---
# Admission Transaction Contract

## Purpose

AdmissionTransaction is the canonical boundary from a validated/selected candidate to a new canonical Atlas repository revision.

SelectedDesign is a design decision. CandidateChangeSet is a proposed delta. Neither mutates canonical source by itself.

## Canonical path

~~~text
SelectedDesign + selected CandidateChangeSet
→ pin expected parent repository revision
→ validate authority event
→ validate verification/security/dependency/license evidence
→ apply candidate in isolated transaction workspace
→ build / test / verify
→ recensus changed Atlas + affected dependency cone
→ compare declared semantic delta vs observed semantic delta
→ re-check policy and closure
→ COMMIT new canonical revision
or
→ ROLLBACK with durable failure evidence
~~~

## Machine contract

The v1 transaction record is ../schemas/admission-transaction.schema.json.

## Preconditions

An admission transaction must bind the exact parent repository revision and reject a stale parent rather than silently rebasing a semantically material change.

Before commit it must have, where applicable:

- SELECTED design identity;
- selected CandidateChangeSet identity;
- authorized selection event;
- required CensusCertificate/closure state;
- security/dependency/license gate evidence;
- test/benchmark/proof evidence required by policy;
- no unresolved blocker for the affected coordinate;
- a rollback plan;
- a declared recensus plan.

## Embedded agent-host rule

When Atlas runs inside a coding-agent host, the provider may have physical write access to the current checkout.

Physical write access is not canonical mutation authority.

The admitted parent revision remains the transaction base. Uncommitted edits, provider-created commits and candidate branches are inputs to CandidateChangeSet/admission, not automatically admitted state.

Before accepting an embedded-host candidate Atlas MUST:

- verify the exact expected parent;
- freeze/hash the exact candidate tree used by verification;
- reject concurrent/stale mutation that changes that tree;
- rerun every gate invalidated by application context;
- recensus the exact resulting tree;
- compare expected versus observed semantic delta;
- commit or publish only through the authorized transaction boundary.

If the host cannot keep the candidate stable during final verification/seal, Atlas MUST snapshot/copy the candidate into a stable workspace or dispatch the transaction to a stronger backend.

Provider co-location with AtlasCore does not allow the provider to mark its own mutation admitted.

## Atomicity

Canonical mutation is fail-closed. Partial application must never be represented as an admitted revision.

Repository writes may use the underlying VCS transaction mechanics, but Atlas records one semantic admission transaction with one expected parent and one resulting revision.

## Post-apply verification

Pre-selection evidence is necessary but not sufficient. Atlas must verify the exact applied tree because patch application, merge context, generated files, dependency resolution or concurrent repository changes can alter semantics.

Required post-apply work includes targeted recensus, semantic-diff comparison, and every gate invalidated by the new tree.

## Semantic delta check

~~~text
CandidateChangeSet expected semantics
vs
observed semantics of exact applied revision
~~~

Unexpected executable-significant state/effect/ownership/concurrency/persistence/security/dependency changes block commit unless separately admitted.

## POLICY_AUTO

POLICY_AUTO may commit only when the work order and admission policy explicitly authorize the mutation class and every mandatory gate passes. The synthesis provider, decision provider and verification provider cannot grant this authority to themselves.

## Rollback

A failed transaction leaves the prior canonical revision authoritative and preserves failure evidence. A later repair is a new CandidateChangeSet/transaction identity, not a rewrite of the failed record.

## Final invariant

Self-modification is not complete when code is generated or selected. It is complete only when the exact new revision is atomically admitted, independently re-observed, verified, and attributable to its parent.
