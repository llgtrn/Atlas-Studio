---
id: atlas.decision.0071.measured-support-levels
type: decision
status: accepted
canonical: true
---
# ADR 0071 — Measured support levels and the universal engineering world (G155)

## Context

The owner's direction is a universal engineering-world system:
- every language, document, media, UI, CAD and electronics artifact ingested with its fidelity preserved;
- one evidence-backed world;
- target-specific materializations for Android, iOS, Windows, Linux, the web and embedded targets.

It must not become a marketing label. Until G155:
- "support" existed only as prose in replay evidence (L0 to L3, never defined);
- nothing stopped a claim that a parsed language was understood;
- nothing stopped a claim that a hashed image was supported.

The G155 survey found:
- no TargetProfile type (`SelectedDesign.target_kind` is a free string);
- no semantics for any non-code artifact;
- AtlasX as contract only.

## Decision

1. **Measured, not declared.** `core::coverage::measure` places every language and every artifact class of a revision on one ladder:
   - L0 discovered;
   - L1 identified by a registered frontend;
   - L2 syntax (disposition PARSED);
   - L3 typed records;
   - L4 an obligation OBSERVED or DERIVED;
   - L5 composed functions;
   - L6 a composed function with a DERIVED relation.

   The inputs are that revision's inventory, census and composed world model. Each rung requires the ones below it.

   Semantic support starts at L3. An artifact class with no frontend stays at L0: its class is read from the extension only, and the list of classes is open. The CLI command is `coverage levels`.
2. **Claims are checked.** `validate_claims` refuses two kinds of claim:
   - a claim above the measured level;
   - a claim for a subject that was never measured.

   `roadmap/SUPPORT-LEVELS.toml` holds the repository's claims and the measured report they rest on. A test enforces three things:
   - no claim exceeds the report;
   - every subject measured at L3 or above is claimed;
   - the list of semantically supported subjects is exactly the claims at L3 or above.
3. **The contract.** `contracts/UNIVERSAL-ENGINEERING-WORLD.md` states the architecture and its invariants, and the current measured state:
   - meaning is universal, not one machine representation;
   - original fidelity is preserved;
   - the logical world may be sharded;
   - one sealed design may have many target materializations, without forking product identity;
   - transformation lineage is recorded;
   - providers build candidates, not truth.

   It also records the anti-cheat statements. What is conceptual is labelled conceptual.
4. **Two missing primitives become debts:**
   - DEBT-TARGET_PROFILE (M0), attack NA-TARGET-PROFILE: a typed profile and a design-compatibility check. A design requiring a capability its target lacks is refused, never silently degraded.
   - DEBT-ARTIFACT_SEMANTICS (M0), attack NA-ARTIFACT-SEMANTICS-IMAGE: the first non-code adapter, lifting images from L0 to L3 with the original payload preserved by digest.

## Measured at G155 (Atlas Studio)

| Level | Subjects |
|---|---|
| L6 | Rust (125 artifacts, 5,788 composed functions); JavaScript (3 browser scripts, 48 composed functions) |
| L2 | CSS, HTML, JSON, Markdown, TOML, Rust include fragments (syntax only) |
| L0 | unidentified artifact classes |

On the tree-sitter pin (replay R3), C and headers were L0.

## Falsification

Mutants of the measurement:
- a rung skipped;
- syntax counted as semantics;
- an artifact class promoted by records;
- a claim above the measured level accepted;
- a claim for an unmeasured subject accepted.

The core ladder tests and the fixture measurement kill them. The fixture has Rust and JavaScript at L6, Markdown at L2, and C and PNG at L0; claims for Markdown at L3, image at L1 and Python are refused.

## Consequences

- No document, ledger or report may call a language or artifact class supported above its measured level.
- The next TypeScript donor replay measures TypeScript with this ladder, since Atlas Studio holds none.
- Construction proceeds narrowly: seal, minimum AtlasX, one delegated target, the first artifact. Then a second target through NA-TARGET-PROFILE, and non-code artifacts through NA-ARTIFACT-SEMANTICS-IMAGE.
