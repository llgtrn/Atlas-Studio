---
id: atlas.decision.0022.browser-instrument-abstraction
type: decision
status: accepted
canonical: true
---
# ADR 0022 — Browser instruments are replaceable; Atlas semantics sit above them

## Context

Creator observation (ADRs 0011–0020) ran through one instrument: Playwright driving Chromium. An audit found Chromium assumptions inside engine-neutral code:
- **Layout resolution:** `LAYOUT_RESOLUTION_PX` (Blink's 1/64 px LayoutUnit) fed every ratio uncertainty in `core::visual` and in construction verification.
- **Network policy:** the policy was written as `BLOCKED_EXCEPT_SUBJECT` at parse time, whatever the instrument actually did.

The directive also asks for Ladybird as an independent engine.

## Decision

1. **The instrument declares its own properties.**
   - `ObservationInstrument` carries the instrument's own `network` policy and its `layout_resolution_px`. The serde default is Chromium's 1/64 px, because every record written before this change came from Chromium.
   - `analyze`, the responsive rules and `verify_intent` propagate the declared resolution.
   - `network_isolated()` reads the enforced policy.
2. **`adapter::browser::BrowserInstrument`** declares, per backend:
   - an id;
   - `InstrumentCapabilities`: layout, interaction, motion sampling, network isolation, and a fresh context per viewport;
   - `availability()`, which returns explicit reasons;
   - `observe_layout`, `observe_interactions` and `observe_motion`. An unsupported operation returns `Unsupported`; nothing is attempted silently.

   All backends evaluate the same in-page procedure (`measure_layout.js`) and emit one raw contract.
   - **`PlaywrightChromium`** is the reference: preferred, reference oracle and fallback.
   - **`WebDriverInstrument`** uses standard W3C WebDriver endpoints only (`webdriver.js`: session, window rect, navigate, execute/sync):
     - chromedriver, whose major version must match the Chromium binary;
     - Ladybird's `WebDriver --headless`, with 1/64 px resolution established from source.
   - The existing entry points are unchanged and delegate to Playwright. `observe_fixture_with` and `bisect_breakpoints_with` take any instrument.
3. **`core::visual::differential`** provides `EngineDifferentialReport`:
   - `compare_layout`: structural computed style, grid track count, every Atlas relation agreeing within the combined declared uncertainty, and responsive rules;
   - `compare_breakpoints`: the same one-pixel boundary;
   - `compare_interactions`: abstract mechanisms and state-graph transitions, not engine-internal change lists.

   Every disagreement starts `UNCLASSIFIED`, and classification comes only from recorded investigations. The classes are:
   - `ATLAS_BUG`
   - `ENGINE_SPECIFIC`
   - `ENGINE_LIMITATION`
   - `INSTRUMENT_LIMITATION`
   - `SPEC_AMBIGUITY`
   - `MEASUREMENT_VARIANCE`
   - `UNKNOWN`

   Classifying never turns a disagreement into agreement. `Independence` keeps two drivers of one engine (`SAME_ENGINE_DIFFERENT_DRIVER`) apart from `INDEPENDENT_ENGINES`.
4. **Runtime and CLI.**
   - `runtime::visual::cross_instrument_layout` observes and bisects through two instruments, records wall time, and compares.
   - `atlas-systemizer observe --differential A,B` and `--instrument ID`.
   - `apply_recorded_classifications` reads `CREATOR-INSTRUMENTS.toml`.
5. **Registry.** `.atlas/roadmap/CREATOR-INSTRUMENTS.toml` holds per-capability parity states and the Ladybird promotion gate, where every item must PASS. A test checks it against the declared backend capabilities.

## Evidence

- **Driver independence (OBSERVED).** All four Creator fixtures were observed through `chromium-playwright` and `chromium-webdriver`, with chromedriver 141.0.7390.37 on Chromium 141.0.7390.37:

  | fixture | result |
  |---|---|
  | editorial | 137/137 layout agreements; 5/5 breakpoints, each at (699, 700) through both drivers |
  | interactive | 128/128 |
  | motion | 94/94 |
  | hermetic probe | 24 agreements, 6 disagreements |

- **The hermetic-probe disagreements are real.** The WebDriver backend cannot abort local-file requests, so `hermetic-probe.css` loads and `.probe` measures 300 px instead of 100 px. They are classified `INSTRUMENT_LIMITATION` (chromium-webdriver), consistent with that backend's recorded network policy, `HOSTS_BLOCKED_LOCAL_FILES_ALLOWED`.
- **This is not cross-engine evidence.** Both instruments drive Blink, and the report says `SAME_ENGINE_DIFFERENT_DRIVER`.
- **Ladybird: BLOCKED_BUILD_PREREQUISITES** (`.atlas/census/ladybird.md`).
  - The build needs gcc 14+ or clang 19+; the host has gcc 13.3 and clang 18.1.
  - Qt 6.9+ is needed even for WebDriver, and none is installed.
  - No binary is published.
  - The W3C harness Ladybird would use is verified on chromedriver. No parity is claimed, and the promotion gate is `NOT_EVALUATED`.
- **Mutation testing: 16 mutants killed.** A first-battery survivor (grid track count ignored) was closed by a new test.
- **Robustness fix.** An intermittent viewport-fit failure was root-caused to reading the viewport before the asynchronous resize applied, and fixed. Six stress runs were then clean.

## Limits and next

- Only Playwright has interaction and motion. The WebDriver backend declares both ABSENT and refuses them.
- WebDriver network isolation needs an Atlas-controlled filtering proxy via the W3C `proxy` capability.
- Chromium stays the preferred backend, reference oracle and fallback. It is not extinct, and no Ladybird evidence exists yet.
- **Next:** a Ladybird binary on a capable host. Then run the four-fixture differential and the promotion gate, and extend WebDriver with W3C Actions for interaction.
