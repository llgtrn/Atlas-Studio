---
id: atlas.references.index
type: reference
status: active
canonical: true
---
# Atlas References

External OSS, protocols, specifications, DeepWiki-style repository explanations, scientific papers and research material are admitted evidence/reference inputs. Presence in Atlas never grants runtime or semantic authority.

The canonical donor registry is `donor-corpus.toml`. Per-donor provenance records exact source identity/revision and license material; duplicate donor registries are forbidden.

Evidence roles remain distinct:

- source code describes current implementation reality;
- tests/runtime evidence support observed behavior;
- specifications/RFCs describe normative contracts;
- scientific papers support algorithms, theory and trade-offs;
- DeepWiki/docs support navigation and explanation;
- AI analysis is inference/candidate until independently verified.

Atlas preserves conflicts instead of silently merging incompatible claims.

## DeepWiki research synthesis

`deepwiki/DONOR-ARCHITECTURE-SYNTHESIS.md` records cross-donor architecture hypotheses and is deliberately `canonical: false`. DeepWiki may identify mechanisms and source locations, but donor implementation claims must be verified against the exact SHA in `donor-corpus.toml` and provenance before becoming ObservedEvidence.
