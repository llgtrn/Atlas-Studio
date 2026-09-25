---
id: atlas.contract.self-recensus
type: contract
status: active
canonical: true
---
# Self-Recensus: the proof of NEXTGEN

Every production generation must prove, through Atlas's own census machinery, that the Atlas it produced understands itself and its world at least as well as the Atlas that created it, and better in the intended way.

~~~text
Atlas_N ── self census (pre) ── one bounded change ── Atlas_N+1 candidate
                                                          │
                              self census (post) + replay census
                                                          │
                      semantic diff vs pre ── intent ── regressions
                                                          │
                                        PROVEN | GENERATION_NOT_PROVEN
~~~

## Census snapshot (`atlas_core::recensus::CensusSnapshot`)

A snapshot is the revision-independent projection of one full census (`systemize`). For each inventoried artifact it records:
- path, disposition and language;
- BLAKE3 content digest;
- fact, typed-record and obligation counts by kind or dimension and by epistemic status;
- explicit unknowns;
- a BLAKE3 digest of the artifact's revision-free semantic projection.

Across the whole scope it also records:
- coverage;
- totals: facts, records, obligations, evidence, diagnostics by code, graph nodes/edges/bindings, normalization, docs;
- dependency closure: state, edges, dangling references, unsupported constructs, dynamic obligations;
- ADL: nodes, edges, diagnostics, constraint verdicts;
- coding admission, docs gate and typed-semantic closure.

Record identities embed the revision by design, so they are excluded. An unchanged artifact at a new revision has an identical projection, and `census_digest` covers everything except `revision` and `dirty`. A snapshot whose digest does not match its content is refused.

## Generation proof (`atlas_core::recensus::prove`)

Inputs: the pre-change snapshot, the post-change snapshot, an independent replay snapshot of the same candidate, and a declared `RecensusIntent`. The intent contains:
- the objective;
- expected changed paths;
- expected total, dependency, ADL and coverage changes;
- explained unknowns;
- unexpected changes accepted, each with a reason.

A generation is `PROVEN` only if all of the following hold:
- every observed change is intended, or accepted with a non-empty reason;
- every intended change is observed;
- the replay digest equals the post digest (deterministic replay);
- all three snapshot digests verify;
- a non-empty objective is declared;
- no forbidden regression remains.

The forbidden regressions are:
- a coverage downgrade that was not intended;
- dependency closure leaving CLOSED;
- a new dangling dependency reference;
- broken typed-semantic closure;
- coding admission becoming blocked;
- the docs gate being lost;
- an increase in semantics that cannot be attributed to an inventoried artifact;
- an ADL constraint newly VIOLATED;
- new unknowns on an artifact that the intent does not explain.

Anything else is `GENERATION_NOT_PROVEN`, and the ledger does not advance.

## Procedure (every production generation, from G57)

1. On the clean base commit: `atlas-systemizer recensus snapshot --root . --out .atlas/evidence/census/<G>/pre.json`.
2. Make one bounded change, including every markdown document it needs, since docs are in census scope.
3. Write `.atlas/evidence/census/<G>/intent.json`.
4. `atlas-systemizer recensus prove --root . --generation <G> --before …/pre.json --intent …/intent.json --out …/recensus.json --after-out …/post.json`.
5. Commit only on `PROVEN`. The ledger entry records `priority`, `serves` and `self_recensus`.

The chain is auditable. `generation_ledger_self_recensus_chain` requires:
- every generation from G57 has a PROVEN report whose digests match its recorded pre and post snapshots;
- each generation's pre digest equals the previous generation's post digest. The next Atlas starts exactly where the last one was proven.

Evidence JSON and ledger TOML are outside census scope, so writing them after the post-census does not break the chain. A census change to that scope boundary must itself be a proven generation.

## Known limits (recorded, not hidden)

- The semantic projection of a typed record is `dimension | status | scope | span | extractor`. A payload change at the same span and scope is detected through the content digest, not through the projection.
- `.atlas` markdown docs are counted, not content-identified.
- Closed in G60: the `atlas-systemizer` crate (`apps/cli`) was outside the manifest's source roots, so G57–G59's CLI changes were invisible to their proofs. It is now censused: 4 of 4 workspace members.
- Closed in G63: facts from ADL sources (`.atlas/declared`, which the inventory excludes by policy) counted as unattributed semantics. Any ADL growth therefore read as a forbidden regression. Snapshot v2 (`atlas.census-snapshot.v2`) attributes them to `AdlState.sources`, a per-source digest. A change to any ADL source must be declared as `source <path>` in `adl_changes`. A snapshot schema change between pre and post is itself an observed change that needs a reasoned acceptance. v1 snapshots keep their recorded digests.
- Closed in G66 (ADR 0028): the proof was file-granular, and function identities embedded the revision and the source position. Snapshot v3 adds `entities`: a revision-stable descriptor per function with signature and body fingerprints. Every non-SAME correspondence (CHANGED, RENAMED, MOVED, MOVED_RENAMED, DELETED, CREATED, AMBIGUOUS) is an observed change that `entity_changes` must declare, exactly or as `<KIND|*> <descriptor prefix>*`. So an undeclared change to another function inside a declared file is no longer accepted.

