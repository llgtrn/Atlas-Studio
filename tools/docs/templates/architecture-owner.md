# {{TITLE}}

Status: **DRAFT ARCHITECTURE OWNER**

## Purpose and ownership

Explain what durable semantic responsibility this document owns, why Chronica needs it, and the nearest responsibilities it explicitly does **not** own.

## Semantic model

Define the minimum vocabulary, representation, temporal behavior, and hard distinctions needed to understand the responsibility. Use natural language first. Add equations, state machines, examples, or diagrams only when they make the semantics clearer.

## Invariants and composition

State the properties implementations must preserve and how this owner composes with neighboring canonical owners. Do not create a second world, authority plane, execution spine, memory universe, or product silo.

## Runtime realization and maturity

Map the responsibility to current `crates/core`, `crates/runtime`, `crates/adapter`, `graph`, `bindings`, `apps`, `ops`, or `tools` ownership. Separate TARGET, CURRENT, and PROVEN when implementation maturity could be confused.

## Evidence and open work

Name the tests/evidence/recovery/observability required to prove the important claims and list only real current gaps.

<!-- Optional sections: worked examples, failure/recovery, sovereignty/safety, storage, performance, Mermaid, mathematics, chronica-contract. Add them only when they sharpen this responsibility. -->
