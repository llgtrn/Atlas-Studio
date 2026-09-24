---
id: atlas.census.ladybird
type: census
status: active
canonical: true
---
# Ladybird: census (G55)

- **Upstream:** `https://github.com/LadybirdBrowser/ladybird.git` at `1647fd9789ef06a6200d6a52b9aa5f3366c480b3` (default branch `master`).
- **License:** BSD-2-Clause (`LICENSE`, copied to `.atlas/licenses/donors/ladybird/`).
- **Size:** 31,788 files, 174,566,061 bytes. Of that, `Tests/` is 112 MB and `Libraries/LibWeb` is 32.7 MB.
- **Census mode:** a remote census plus transient sparse slices in lane scratch. The peak footprint was 62.7 MB, and it was deleted afterwards. `storage_state = SOURCE_DELETED`. The source was never materialized under an Atlas donor root (ADR 0021).

## Two separate questions

The directive keeps apart (A) using Ladybird as an external instrument backend and (B) absorbing Ladybird mechanisms natively. Using Ladybird as a browser is not absorption.

## A. Instrument backend: BLOCKED_BUILD_PREREQUISITES

- **No prebuilt binary is published.** The only distribution recipe is an in-tree Flatpak manifest, which you build yourself.
- **Compiler minimums.** gcc 14 or clang 19 are required (`Meta/Utils/find_compiler.py`). The container has gcc 13.3 and clang 18.1.
- **Qt 6.9+ is required, even for WebDriver.**
  - `CMakeLists.txt` builds `Services` (which contains WebDriver) and `UI` under one `ENABLE_GUI_TARGETS` switch.
  - `UI/cmake/GUIFramework.cmake` makes Qt the only Linux GUI framework.
  - No Qt is installed here.
- **vcpkg** builds about 40 ports from source, including skia, angle, ffmpeg and icu. No disk or time figure is documented. Under the current working-set budget (5.28 GB, PRESSURE band, ADR 0021), a build would need a measured footprint before admission.
- **Automation surface: present and standard.** `Services/WebDriver` implements W3C WebDriver: new session with `ladybird:headless`, navigate, execute script, set window rect, element rect, actions and screenshot.
  - Atlas's `WebDriverInstrument` uses only those standard endpoints.
  - The same harness was verified end-to-end against chromedriver 141 and Chromium 141 on all four Creator fixtures.
  - The Ladybird-specific delta is the binary path and the `ladybird:headless` capability.
- **Layout resolution: established from source.** `Libraries/LibCompositing/PixelUnits.h:69-70` gives `CSSPixels` `fractional_bits = 6`, which is 1/64 px, the same grid as Blink's LayoutUnit.
- **Network isolation: not available natively.** `--resource-map` substitutes URLs but blocks nothing. A block-everything-except-subject policy would need the W3C `proxy` capability routed through an Atlas-controlled filtering proxy. Until then, the backend records `network = NOT_ENFORCED`.

## B. Mechanism decisions

| mechanism | source slice | decision | reason / blocking question |
|---|---|---|---|
| W3C WebDriver automation boundary | `Services/WebDriver/*` (116,705 B) | EXTERNAL_BOUNDARY | A standard protocol. Atlas speaks it natively (`adapter/src/browser/webdriver.js`); nothing Ladybird-specific is absorbed. |
| Headless operation | `Libraries/LibWebView/HeadlessWebView.*` | EXTERNAL_BOUNDARY | A property of the engine binary. |
| Layout and computed style | `Libraries/LibWeb/` (32.7 MB) | REFERENCE_ONLY | Absorbing a browser engine is out of scope. The one source-backed fact Atlas needed, the 1/64 px resolution, is recorded. |
| Process isolation / sandbox | `LibIPC`, `LibSandbox`, `Services/RendererSandbox*`, `WebContent`, `RequestServer` (1.10 MB) | REFERENCE_ONLY | Blocking question: does any Atlas instrument need its own renderer-process isolation beyond the engine's? None does today. |
| URL-to-local-file substitution | `--resource-map` (`Services/WebDriver/main.cpp`) | ABSORB_LATER | Blocking question: hermetic replay of subjects that reference remote assets. The current fixtures are self-contained and nothing consumes substitution. |

## Next

The Ladybird backend becomes testable, and the promotion gate evaluable, on any host that provides:
- a Ladybird `WebDriver` binary (`ATLAS_LADYBIRD_WEBDRIVER`);
- gcc 14+ or clang 19+;
- Qt 6.9+;
- a measured build footprint admitted by the working-set gate.

No part of the parity gate has been evaluated, and no parity is claimed.
