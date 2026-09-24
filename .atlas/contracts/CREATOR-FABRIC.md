---
id: atlas.contract.creator-fabric
type: contract
status: active
canonical: true
---
# Creator Fabric Contract

Creator Fabric is the Atlas capability family that turns authorized visual observation and intent into original, verified visual and media artifacts.

```text
authorized subject
→ sandboxed instrument
→ OBSERVED records
→ DERIVED/INFERRED semantics
→ design genome
→ original synthesis
→ construction
→ render/replay
→ verification
→ nextgen
```

Machine-readable lane state lives in `.atlas/roadmap/FRONTIER.toml`, in lanes `creator_*`.

## Authorization boundary (MUST)

1. Atlas observes only subjects it is authorized to observe:
   - local fixtures (`LOCAL_FIXTURE`);
   - user-provided or explicitly authorized environments;
   - public open demos.

   Every observation record names its authorization.
2. The instrument never bypasses authentication, paywalls, robots/security restrictions, anti-abuse systems or private-data boundaries. It never impersonates a user. "Headless" means reproducible measurement, never evasion.
3. The v1 instrument is hermetic. It creates one fresh browser context per viewport, with no cookies, no credentials and no persistent state. Every network request other than the subject is aborted. A test proves that a sibling stylesheet is blocked.

## Epistemic discipline (MUST)

| Status | Meaning in Creator records |
|---|---|
| `OBSERVED` | Measured by the instrument: box geometry and computed style. What rendered, not how it was implemented. |
| `DERIVED` | Computed from observations under a named rule, with a propagated uncertainty bound: layout ratios, responsive rules. |
| `INFERRED` | A parameter estimated from behaviour, such as a spring constant from frame sequences. It carries alternatives and uncertainty. |
| `HYPOTHESIS` | An Atlas-generated design candidate. |

Additional rules:

- A black-box inference is never presented as knowledge of source code.
- A responsive change observed between two viewports is stated as the width interval containing the change. It is never stated as an exact breakpoint the instrument did not measure.
- Aesthetic scores are never objective truth. When no candidate dominates, multi-objective selection preserves the Pareto alternatives.

## Originality (MUST)

Creator Fabric follows UNDERSTAND → ABSTRACT → RECOMBINE → CREATE, never SCRAPE → COPY.

- Observations are reduced to mechanisms and normalized relations: ratios, structural responsive rules, interaction and motion grammar. Pixels and verbatim content are not the genome.
- Logos, proprietary illustrations, branded assets and exact copy are not preserved or reproduced unless they are explicitly provided and authorized.
- Observation records hold only a content-free text size (`text_chars`), never the text itself.
- Reference-inspired output is verified on semantic relationships, not by pixel cloning.

## Provenance (MUST)

- **Observations.** Every observation records:
  - the subject's BLAKE3 digest, taken over the exact bytes loaded, before and after observation (a subject that changes mid-observation is refused);
  - the instrument's engine and version;
  - the driver and version;
  - the network policy.
- **Generated media.** Generated media records will carry intent, provider, model and version, seed, transformations, source assets, both the framework license and the model-weight license (tracked separately), the output digest, and the verification result.
- **Providers are compute providers.** Atlas owns intent, media graph, state, provenance and verification.

## External boundaries

- **Browser instrument (EXTERNAL_BOUNDARY, recorded per report).** Playwright drives headless Chromium. The browser is an instrument in the same sense a compiler binary is an oracle.
- **Hosted generation products are external providers, never donors.** An absorbable donor requires real source plus a compatible license. Higgsfield, for example, publishes no open generation code (G42 lane).

## Implementation status

- **G43 (ADR 0011).** Implemented:
  - `core::visual`: observation records, ratio-first `LayoutRelation`s with resolution-bounded uncertainty, and interval-stated `ResponsiveRule`s.
  - `adapter::browser`: the hermetic instrument.
  - `runtime::visual::observe_fixture`.
  - `atlas-systemizer observe --fixture F [--viewport WxH]... [--out O]`.
- **G45–G49 (ADRs 0013, 0015, 0016, 0017):**
  - breakpoint bisection (`observe --bisect`);
  - interaction stimuli and mechanisms (`observe --interact`);
  - motion curve sampling and validated easing inference (`observe --motion`);
  - the construction loop (`create`): a ratio-first `DesignIntent` is lowered to placeholder-only HTML/CSS, rendered, re-observed and verified relation by relation.
- **G50 (ADR 0018):** the design genome and recombination.
  - `genome` abstracts mechanisms with provenance.
  - `create --genome --recipe` enforces originality mechanically: at least 2 sources, and no mechanism reproduced wholesale. Every parameter records whether it was inherited, varied or defaulted.
- **G52 (ADR 0020):** multi-candidate search.
  - `search` enumerates originality-valid recipes over a genome, creates and verifies each, and measures objectives with stated meanings.
  - Only verified, fully measured candidates compete, and the Pareto front is kept as a set.
- The media lanes (image, video, 2D, 3D) and self-improvement are not yet implemented.
