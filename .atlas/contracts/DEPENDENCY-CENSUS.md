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

## Extinction interaction

A donor scope cannot be ABSORBED/EXTINCT based only on its top-level repository.

Atlas must know which required behavior came from the donor itself and which came from its dependency closure. Extinction proof must demonstrate that deleting the donor source does not leave a hidden requirement on deleted donor-owned source.

Independent third-party dependencies that Atlas intentionally adopts as explicit Atlas dependencies remain separately identified and censused; they are not silently relabeled as Atlas-native or as part of the extinct donor.
