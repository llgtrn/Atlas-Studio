# Atlas Systemizer Boundary

Status: **ACTIVE**

Canonical decision: ADR-0029.

## Purpose

Atlas Systemizer is the external engineering subsystem for Chronica.

It exists to understand, graphinize, standardize, audit, plan, and verify engineering work. It is not part of the Chronica runtime and it is not a second canonical world.

## Stable integration

Chronica integrates with one stable executable contract:

~~~text
atlas-systemizer
cli_api = atlas.systemizer.cli.v1
~~~

Chronica does not import Atlas crates.

Chronica does not depend on Atlas graph storage.

Chronica does not know Atlas parser implementations.

Chronica does not know which OSS references Atlas uses internally.

### Required commands

~~~text
atlas-systemizer contract --format json

atlas-systemizer systemize
  --root .
  --config .atlas/systemizer.toml
  --out .atlas/evidence/systemize.json

atlas-systemizer docs audit
  --root docs
  --config .atlas/systemizer.toml
  --format json

atlas-systemizer code analyze
  --root .
  --config .atlas/systemizer.toml
  --format json
~~~

The command surface is compatibility API. Internal Atlas implementation may be completely replaced without requiring Chronica product changes.

## Ownership matrix

| Concern | Chronica | Atlas Systemizer |
| --- | --- | --- |
| Canonical product/runtime source | owns | reads |
| World/authority/execution semantics | owns | analyzes |
| Architecture decisions | owns meaning | validates structure/links |
| Documentation content | owns meaning | standardizes/audits |
| Documentation taxonomy/schema tooling | consumes | owns |
| Source/code graph | source evidence | owns derived projection |
| Fact/Technology/Design graph | no runtime dependency | owns derived projection |
| Donor/reference technology for Atlas | no | owns |
| Build/affected-CI graph | consumes plan | owns analysis |
| Network/Fleet engineering audit | consumes evidence | owns tooling |
| Refactor/migration plan | reviews/acts | proposes |
| Merge authority | owns via normal Git/policy | never owns |
| Generated Atlas cache/index | rebuildable | owns |
| Runtime state | owns | never owns |

## What remains in Chronica

Permanent Chronica-side Atlas material is deliberately small:

~~~text
.atlas/systemizer.toml

docs/decisions/0029-atlas-systemizer-external-development-subsystem.md

docs/architecture/operations/atlas-systemizer-boundary.md

normal Chronica docs / contracts / source
~~~

Derived output defaults to:

~~~text
.atlas/evidence/
~~~

and should not be treated as canonical truth.

## What leaves Chronica

Implementation currently under:

~~~text
tools/system-atlas/
tools/docs-atlas/
tools/reality-atlas/
~~~

is a migration baseline.

New Atlas features belong only in:

~~~text
llgtrn/Atlas-Systemizer
~~~

The legacy implementation is retired by measured parity, not copied forward as a competing implementation.

## Docs standard

Atlas Systemizer owns a stable documentation grammar.

Chronica documents use semantic roles such as:

~~~text
decision
blueprint
architecture
contract
runbook
reference
generated-evidence
~~~

Canonical documents should converge on frontmatter carrying at least:

~~~text
id
type
status
canonical
~~~

Atlas may normalize mechanics such as frontmatter, indexes, references, stale/superseded markers, and graph links.

Atlas may not silently rewrite architectural meaning.

Meaning-changing normalization remains reviewable ACT.

## Engineering graph

Atlas may derive:

~~~text
Source Graph
  -> Fact Graph
  -> Behavior Graph
  -> Technology Graph
  -> Design Graph
  -> Engineering Graph
  -> Build / CI / Proof Plan
~~~

Every graph is:

~~~text
derived
rebuildable
evidence-linked
non-canonical
~~~

Chronica runtime truth remains independent.

## Runtime isolation

The invariant is:

~~~text
Chronica runtime
  -X-> Atlas Systemizer libraries/services

developer / CI
  -> atlas-systemizer CLI
~~~

If Atlas is unavailable, Chronica runtime must continue to operate. Development automation may be degraded or blocked, but product execution does not depend on Atlas availability.

## Versioning

Breaking the existing CLI requires a new explicit contract such as:

~~~text
atlas.systemizer.cli.v2
~~~

Chronica may pin a supported CLI API without pinning Atlas internal architecture.

This is the central mechanism that lets Atlas evolve aggressively while Chronica code remains stable.
