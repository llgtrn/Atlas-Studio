---
id: atlas.decision.0036.ambient-input-effects
type: decision
status: accepted
canonical: true
---
# ADR 0036 — Ambient-input effects (IRIS, absorbed)

## Context

ADR 0032 made EFFECT observable for resolved standard-library filesystem calls and recorded what it left out: "environment and time reads have no EffectCategory". The miri cycle (G76) had counted them: 50 environment and 17 time call sites. Mapping them onto an existing category would mislabel them. Reading `TMPDIR` or the clock is not `EXTERNAL_IO`.

The IRIS census (first-50 #27) recorded a recensus requirement: re-examine IRIS's effect taxonomy once Atlas's EFFECT dimension is real. It has been real since G77. IRIS's 44 effect tags separate ambient inputs (`EnvGet`, `ClockNs`, `Timestamp`, `Random`) from host I/O (`FileRead`, `TcpConnect`, ...). Its evaluator treats them as a distinct class that can be supplied as a value instead of performed. Miri's shims group the same inputs separately (`env`, `time`; G76).

## Decision

1. **Two categories.** `EffectCategory` gains `ENVIRONMENT_READ` (process environment variables, arguments, working and well-known directories) and `CLOCK_READ` (a read of a clock). `SEMANTIC-FACTS.md#effectfact` names them.
2. **Declared std paths.** The std-path effect table (ADR 0032) declares:
   - `std::env::{args, args_os, current_dir, home_dir, temp_dir, var, var_os, vars, vars_os}` as `ENVIRONMENT_READ`;
   - `std::time::{Instant::now, SystemTime::now}` as `CLOCK_READ`.
3. **Still undeclared.** Environment writes (`set_var`, `set_current_dir`) and `current_exe`, a filesystem lookup on most platforms, stay undeclared: absent means "declares nothing", never "no effect".
4. **No other change.** The resolution engine is unchanged: a declared path call is a DERIVED effect site of its caller, as in G77.

Nothing from IRIS is copied. Its code is AGPL-3.0-or-later, and the absorbed mechanism is the vocabulary distinction, re-derived natively.

## Consequences

- EFFECT|DERIVED records rise from 324 to 383. Exactly the 59 resolved `std::env`/`std::time` path calls of the workspace (env 50, time 9) become effect sites. All 59 agree with rust-analyzer 1.90.0 SCIP at the same anchors.
- Method-level time reads (`.elapsed()`, `.duration_since()`) remain outside the engine, which resolves path calls only. The EFFECT obligation stays UNKNOWN.
- Mutation battery: 6/6 killed.
  - dropping `env::var`;
  - declaring the clock as the environment;
  - a wrong contract name;
  - declaring `current_exe`;
  - dropping `SystemTime::now`;
  - declaring `temp_dir` as a filesystem read.
