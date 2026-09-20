---
id: atlas.architecture.invention-pipeline
type: architecture
status: active
canonical: true
---
# Invention Pipeline

## Purpose

Atlas treats system creation as a graph-backed invention process rather than prompt-to-code generation.

## Pipeline

Idea -> North Star -> Documentation Plan -> System Architecture -> Blueprint -> Contracts -> OSS/Research Search -> Source/Technology Graph -> Design Alternatives -> Target Design Graph -> Implementation Slices -> Optional Mirrors -> Code/Design Translation -> CI/Proof -> Reconvergence -> Dependency Extinction.

## Blank Repository Mode

For a new blank target, Atlas first creates the documentation/control skeleton and target design graph. Only after DocsGate and graph admission may it propose the initial repository structure and code.

## Existing Repository Mode

Atlas reads current docs/source, detects drift and missing primitives, then creates a bounded design delta rather than rewriting unrelated parts.

## Creativity

Atlas may compose primitives from multiple donors, reject donor architecture, or propose a new graph that is materially different when evidence supports better invariants, performance, simplicity, safety or zero-dependency goals.
