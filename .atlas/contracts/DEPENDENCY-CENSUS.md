---
id: atlas.contract.dependency-census
type: contract
status: active
canonical: true
---
# Dependency Census Contract

## Purpose

Atlas census does not stop at the repository boundary.

For Atlas itself and for every admitted external corpus, Census MUST account for the resolved dependency graph transitively for each admitted build/runtime context. A root repository is only the entry point into the engineering world that actually produces the system.

The dependency graph is part of canonical census truth, not an optional SBOM side report.

## Canonical rule

~~~text
root repository / product / donor
        ↓
workspace + manifests + lockfiles + build metadata
        ↓
resolved direct dependencies
        ↓
resolved transitive dependencies
        ↓
dependency source / binary / toolchain / system boundary
        ↓
repeat until fixed dependency closure
        ↓
inventory + semantic census of source-backed nodes
        ↓
normalize / reconcile
        ↓
CensusCertificate dependency closure
~~~

Stopping at a package name, manifest declaration or first-level dependency list is incomplete census.

## Resolution context

Dependency truth is contextual. Atlas MUST identify the resolution context that produced an edge, including where applicable:

- target platform / architecture;
- build profile;
- workspace/package member;
- feature/capability selection;
- optional dependency activation;
- test/dev/build/release mode;
- toolchain/compiler version;
- environment/configuration inputs that materially affect resolution.

Atlas does not need to enumerate an infinite configuration space. The Genome or census request defines the admitted context matrix. Every dependency edge active in an admitted context MUST be accounted for, and declared-but-inactive edges MUST have an explicit activation condition/disposition.

## Dependency identity

A resolved dependency instance MUST have enough identity to distinguish different concrete providers. Depending on source kind this includes:

- ecosystem/namespace/name;
- exact version;
- exact VCS repository + revision when VCS-backed;
- content checksum/hash when available;
- registry/source locator;
- workspace/path identity;
- binary/image/toolchain/system identity when source is not the provider;
- provenance and license evidence where applicable.

Two versions or two revisions of the same package are distinct dependency instances.

## Dependency edge

A dependency edge records at least:

- consumer scope;
- provider dependency identity;
- dependency kind;
- resolution context;
- manifest/build declaration evidence;
- lock/resolution evidence when available;
- activation condition/features/target;
- source availability;
- epistemic status and unresolved diagnostics.

Dependency kinds include, where applicable:

- runtime/library;
- build;
- dev/test;
- proc-macro/code-generation;
- compiler/toolchain;
- FFI/native/system package;
- dynamic/plugin;
- optional/feature-gated;
- generated/input artifact;
- container/image/runtime;
- external service/capability dependency.

The vocabulary may evolve, but a dependency class may not disappear merely because it is inconvenient to inspect.

## Transitive closure algorithm

For each admitted root and resolution context Atlas conceptually:

1. inventories workspace/package/build definitions;
2. resolves direct dependency instances deterministically from pinned/locked evidence where possible;
3. emits typed dependency edges;
4. enqueues every newly discovered source-backed dependency instance;
5. repeats manifest/build/dependency resolution for that instance;
6. de-duplicates only by stable dependency identity, not by package name;
7. continues until no unresolved dependency node can expand further;
8. records non-source terminal boundaries explicitly;
9. runs inventory/semantic census on source-backed nodes according to scope policy;
10. reconciles dependency evidence from independent sources where available.

Cycles are represented as graph cycles; they do not terminate accounting by omission.

## Source-backed dependencies

When exact dependency source is available to Atlas under the admitted policy, it becomes part of the census corpus for that context.

Atlas MUST inventory it and MAY adapt semantic depth according to policy, but it cannot silently treat external source as opaque merely because it lives in a package cache, vendored directory, Git checkout, submodule or registry extraction.

Source locations may include:

- workspace/path dependencies;
- vendored source;
- VCS checkouts/submodules;
- package-manager registry source;
- generated source;
- SDK/toolchain source when admitted and available.

Source location is not semantic identity.

## Non-source terminal boundaries

Some dependencies terminate at a boundary whose source is unavailable, intentionally external, or not admitted. Examples include an OS library, device driver, external SaaS capability, prebuilt SDK, container base, or proprietary binary.

Such a boundary is allowed only as an explicit typed terminal with identity, version/hash where possible, interface/capability/effect evidence, provenance, and UNKNOWN/UNSUPPORTED status where evidence is insufficient.

"External" is a disposition, not permission to omit the edge.

## Build-time and dynamic dependency discovery

Static lockfiles are evidence, not always complete truth.

Build scripts, proc-macros, generated code, environment probes, pkg-config/native linking, dynamic loading, plugin discovery and runtime capability selection may introduce dependencies not visible in a root manifest.

Atlas MUST account for these through static build metadata, compiler metadata, sandboxed observation, binary metadata, runtime traces or explicit UNKNOWN obligations. Census is analysis; execution requires separately authorized/sandboxed capability.

## Atlas self-census

Atlas Studio is subject to this contract itself.

Atlas self-census MUST start from the Atlas repository and expand through the complete admitted dependency contexts used to build, test, verify and release Atlas, including all ecosystems present in the workspace.

A lockfile alone is not self-census closure. Atlas must produce the dependency graph, stable resolved identities, source/boundary dispositions, transitive closure and census records required by this contract.

Atlas may not claim a CLOSED self-census while an active transitive dependency edge is silently outside the corpus.

## Atlas product feature

Transitive dependency census is a first-class Atlas capability for arbitrary admitted systems, not a one-off bootstrap procedure.

Conceptually:

~~~text
census(root, resolution_contexts, dependency_policy)
→ DependencyClosure
→ expanded admitted corpus
→ ordinary Atlas census/normalize/reconcile
~~~

The same typed dependency records feed query, graph, security, license/provenance, build reasoning, invention, compiler optimization and extinction analysis. An SBOM or dependency report is a projection from this canonical graph, not a parallel truth model.

## Donors and dependency attribution

Donor census follows the same rule.

If a donor capability is actually implemented partly or wholly by a transitive dependency, Atlas MUST attribute the mechanism to the correct dependency node instead of incorrectly crediting the top-level donor.

A discovered dependency is not automatically an approved donor for absorption. It is first a dependency-census subject. If Atlas decides to learn/absorb its mechanism, it is promoted through the normal donor admission/provenance process.

Technology Genomes must distinguish:

- donor-owned mechanism;
- dependency-provided mechanism;
- shared/cross-boundary mechanism;
- external terminal capability.

## Closure

Dependency closure is complete for an admitted context only when:

- every active direct edge is accounted;
- every active transitive edge is accounted;
- every resolved node has stable identity/provenance;
- every source-backed node has an inventory disposition and required census;
- every non-source terminal is explicitly typed;
- dynamic/build-generated dependency obligations are resolved or explicitly UNKNOWN/UNSUPPORTED;
- no new dependency nodes appear on another closure iteration;
- policy-forbidden unresolved dependency states are zero.

Dependency closure participates in fixed-point census closure.

## Implementation status

This contract was written before an implementation existed; `.atlas/coverage.BUILD` was a
permanent `UNSUPPORTED` stub (`runtime::census::build_census`).

**Materialized and verified**: the Cargo ecosystem, for Atlas's own workspace
(`.atlas/contracts/DEPENDENCY-CENSUS.md#atlas-self-census`). `adapter::census_cargo_workspace`
statically parses `Cargo.lock` (Cargo's own already-fully-resolved transitive graph -- no
`cargo tree`/`cargo metadata` execution in the production path) plus each workspace member's own
`Cargo.toml` (for real, evidenced `DependencyRole`/`DependencyActivation`, never defaulted by
assumption), producing a typed `DependencyClosureReport` (`core::census::dependency`) wired into
`SystemizeReport` and promoting `BUILD` to `OBSERVED` once `DependencyClosureState::Closed` is
actually reached (below).

Same-name multi-version disambiguation is materialized: a `Cargo.lock` dependency-array entry
disambiguated as `"name version"` resolves to the matching `[[package]]` block by version, a
disambiguated entry naming a version with no matching block is a real dangling reference (not a
guess), and a genuinely ambiguous entry with no version suffix at all (adversarial/malformed input
-- real Cargo.lock output always disambiguates when more than one resolved version exists) is
still reported rather than guessed.

**Dependency role and activation are separate, orthogonal typed facts, never collapsed into one
enum.** An earlier version of `DependencyEdge` carried a single `DependencyKind` with
`Runtime`/`Dev`/`Build` and `Optional`/`TargetConditional` as mutually-exclusive alternatives, so a
`[dev-dependencies]` entry declaring `optional = true` lost its `Dev` classification entirely
(reported only as `Optional`), and a target-conditional `[build-dependencies]` entry lost its
`Build` classification (reported only as `TargetConditional`) -- a real dependency can be `Build`
role, `optional`, and `target_conditional` all at once, and no single fact may overwrite another.
Fixed: `DependencyEdge.role: Option<DependencyRole>` (`Runtime`/`Dev`/`Build`/`ProcMacro`, which
table) and `DependencyEdge.activation: DependencyActivation` (`{optional: bool,
target_conditional: bool}`, under what condition) are independent fields; a `[dev-dependencies]`
entry with `optional = true` is `role: Some(Dev), activation: {optional: true, ..}`, and a
target-conditional `[build-dependencies]` entry is `role: Some(Build), activation: {..,
target_conditional: true}`. Neither flag means "active in this admitted resolution context" --
feature-selection resolution and target-selector-expression parsing against an admitted target
matrix both remain explicit TARGET work (below); this wave only records that the manifest declared
the condition at all.

**Closure state is explicit, not a bare boolean** (`DependencyClosureState`:
`NotApplicable`/`Blocked`/`Partial`/`Closed`). A prior version of this bootstrap represented
closure as `dangling_references.is_empty()` alone, which could not distinguish "no Cargo.lock
exists at this root" from "census never ran" from "a real, verified, dependency-free closure" --
`runtime::systemize` fabricated a zero-edge default report for the no-`Cargo.lock` case, whose
`is_closed()` then silently read `true`, so a repository outside the Cargo ecosystem entirely
looked identical to one with a genuinely closed Cargo census and never raised
`DEPENDENCY_CLOSURE_NOT_CLOSED`. Fixed: `NotApplicable` (no `Cargo.lock`) never blocks and leaves
`BUILD` at its prior status; `Blocked` (a present `Cargo.lock` that parsed to zero `[[package]]`
entries -- evidence of a malformed/truncated file, not a verified empty project) and `Partial` (a
dangling reference, unresolved ambiguity, or detected-but-unsupported manifest construct) both
block; `Closed` requires real resolved packages and zero unresolved issues of any kind. See
`runtime::build_coverage_from_dependency_closure` and its own falsification tests, one per state
transition.

**Source classification is evidence-based, not a two-way guess.** A prior version classified any
package with a `source` field as `Registry` and any without as `WorkspaceMember` -- wrong for a
`git+...` source (classified `Registry`) and for a non-workspace local `path = "..."` dependency
(classified `WorkspaceMember` merely for lacking a `source` field, which a real workspace member
also lacks). Fixed: `Vcs` for a `git+` source, `Other` for a source matching neither recognized
prefix, and `Path` for a source-less package whose name is not in the real, evidenced
workspace-member set (cross-referenced from the root manifest's own `workspace.members`) --
`WorkspaceMember` now requires that positive evidence, never merely the absence of a `source`
field.

**Unsupported manifest constructs are named explicitly, not silently mis-parsed.** A multi-line
`workspace.members` array is detected (previously silently treated as "no workspace declared")
and forces `Partial` state rather than a false `Closed`. A multi-line inline-table dependency
entry (e.g. `foo = {\n version = "1",\n optional = true\n}`) is now actually read correctly
(the continuation lines are joined before classification), closing what was a real, if
unexercised, misclassification risk rather than merely naming it.

**Dynamic/build-time dependency obligations are declared, not implied away.** Every
`DynamicDependencyObligation` class -- `BuildScript`/`ProcMacroExpansion`/`PkgConfig`/`NativeLinking`/
`GeneratedSource`/`EnvironmentProbe`/`DynamicLoading`/`PluginDiscovery`/`ExternalCapability` -- is
reported as still-unresolved (`DependencyClosureReport::dynamic_obligations`) whenever a census
actually runs. `BUILD: OBSERVED` from a `Closed` static package graph means exactly that: the
static package graph is resolved. It does not mean no build script, proc-macro, or native link
could introduce dependency behavior this closure does not see -- that remains a separate, always
explicit axis, none of it implemented yet.

**Independent verification is now genuinely independent.** A prior evidence record
(`dependency-census-cargo-bootstrap.json`) claimed the pre-existing `systemize` CLI as an
independent evidence channel. That claim was false: `systemize` calls
`adapter::census_cargo_workspace` internally -- the same candidate parser under test, reached
through a different entry point, not a second implementation or a second source of truth. See
`.atlas/evidence/verification/dependency-census-independent-validator-correction.json` for the
corrective record. `adapter::dependency::cargo_oracle` (test-only, never reachable from production
code) now runs `cargo metadata --format-version=1 --offline` -- Cargo's own resolver, sharing no
code with this crate's static parser -- and diffs its resolved edges and package identities
against the candidate's output on this repository's own real workspace. **Canonical census
extractor != independent verification oracle**: the oracle exists strictly as `#[cfg(test)]`
verification evidence and must never be called from `census_cargo_workspace`, `systemize`, or
anything gating coding admission, since it executes `cargo` as a subprocess and would otherwise
violate the "ingestion is not execution" security boundary this contract requires of the
production path.

**Still TARGET, not silently claimed done**: other ecosystems (npm, pip, ...); full
resolution-context modeling (feature-selection activation for `activation.optional` edges; parsing
the actual target-selector expression for `activation.target_conditional` edges against an admitted
target-context matrix -- this wave accounts every edge as unconditionally active, matching every
real edge in this workspace today, since none carry either flag); `ProcMacro` `DependencyRole`
emission (requires reading a dependency's own manifest, which this parser never does); the
`"name version (source)"`
lockfile form, needed only when the same name and version resolve from two different sources;
non-Cargo build metadata (compiler/toolchain version, native/FFI links); a single non-workspace
root crate (`[package]` with no `[workspace]` at all -- workspace-member discovery requires a
`[workspace] members = [...]` array; Atlas's own self-census, the only real corpus this bootstrap
is proven against, is always a workspace); actually implementing any dynamic-dependency-obligation
probe (all nine classes remain declared-but-unresolved); a second real evidence channel besides
`cargo metadata` (e.g. a from-scratch independent parser) for continuous production use -- the
`cargo metadata` oracle is verification-only and runs only in this crate's own test suite.

## Extinction interaction

A donor scope cannot be ABSORBED/EXTINCT based only on its top-level repository.

Atlas must know which required behavior came from the donor itself and which came from its dependency closure. Extinction proof must demonstrate that deleting the donor source does not leave a hidden requirement on deleted donor-owned source.

Independent third-party dependencies that Atlas intentionally adopts as explicit Atlas dependencies remain separately identified and censused; they are not silently relabeled as Atlas-native or as part of the extinct donor.
