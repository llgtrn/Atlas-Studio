---
id: atlas.contract.donor-workbench-isolation
type: contract
status: active
canonical: true
---
# Donor Workbench Isolation Contract

## Trigger

During the semantic-graph-native-language donor census (`.atlas/census/donors/semantic-graph-language-lane-2026-09-22.md`), DUUMBI's clone (`.atlas/temporary/donors/duumbi/`) was found to ship a `.claude/skills/jsonld-schema/SKILL.md` file that auto-surfaced as an available skill in the censusing agent's session, purely because it existed on disk under a path the agent-tooling convention scans. A broader repository-wide check then found the same class of risk across at least nine other admitted donors: `.claude/skills/`, `.claude/agents/`, `.claude/hooks/*.sh` (a literal shell script, `iris/.claude/hooks/log-conversation.sh`), `.claude/settings.json`, `.claude/CLAUDE.md`, a `.cursor/environment.json`, and nineteen bare `CLAUDE.md`/`AGENTS.md` files, plus two `.mcp.json` files, scattered across donor roots.

None of this was ever executed or acted on by any census agent -- every one was correctly treated as inspected data. But the exposure was real: donor content is untrusted external input (`.atlas/contracts/SEMANTIC-EXTRACTION.md`: "OSS is untrusted input"), and a file shaped like a skill, hook, agent definition, MCP server config, or project-memory file is a live prompt-injection / arbitrary-execution surface the moment it sits on a path an agent-tooling convention scans -- regardless of whether the donor's own author intended harm. A hook script in particular can execute on an agent-tooling event trigger, not merely on request.

**2026-09-22 update, during the ASIR/ACP semantic-representation donor lane:** a repository-wide sweep for related path classes (prompted by staging `mlir`/`xdsl`/`egglog`/`wasm-component-model`/`wasm-spec`/`capnproto`/`rkyv`/`agentir`) found a second live agent-tooling-shaped class this contract's original pattern list did not cover: **GitHub Copilot's ambient project-instruction files**, `.github/copilot-instructions.md` and `.github/instructions/*.instructions.md`. These function exactly like `CLAUDE.md`/`AGENTS.md` (auto-loaded, ambient, no explicit per-session opt-in) for any tooling that honors the Copilot custom-instructions convention. Found live in five places: `ast-grep/.github/copilot-instructions.md`, `arrow/.github/copilot-instructions.md`, `security/keycloak/.github/copilot-instructions.md`, and `llvm-project/.github/instructions/{llvm,lldb}.instructions.md`. Quarantined under the existing file convention (suffix `.donor-untrusted`) immediately on discovery; a plain `.github/` directory itself is unaffected (workflows/issue templates/CI config are not ambient agent instructions and are not quarantined).

## Invariant

Everything under `.atlas/temporary/donors/**` is census-visible, staged workbench source -- never repository configuration and never agent authority. Concretely:

1. No path under `.atlas/temporary/donors/**` may be named exactly `.claude`, `.codex`, or `.cursor` (as a directory), or exactly `CLAUDE.md`, `AGENTS.md`, `.mcp.json`, `copilot-instructions.md`, or any file matching `*.instructions.md` under a `.github/instructions/` directory, at any depth. Donor content that arrives in one of these shapes is quarantined (renamed, contents fully preserved, never deleted or truncated) immediately upon staging, before census begins:
   - a `.claude`/`.codex`/`.cursor` directory is renamed to `_donor-quarantine.<name-without-leading-dot>` (e.g. `.claude` -> `_donor-quarantine.claude`) in its same parent directory;
   - a `CLAUDE.md`/`AGENTS.md`/`.mcp.json`/`copilot-instructions.md`/`*.instructions.md` file gets the suffix `.donor-untrusted` appended (e.g. `CLAUDE.md` -> `CLAUDE.md.donor-untrusted`, `.github/copilot-instructions.md` -> `.github/copilot-instructions.md.donor-untrusted`). A plain `.github/` directory itself is never quarantined -- only the specific ambient-instruction file(s) it may contain.
2. Quarantining is a rename, never a deletion and never a content edit -- every byte of donor evidence stays readable and greppable for census; only the path stops matching a live agent-tooling discovery convention.
3. This applies to every current and future donor. Any agent that clones/stages a new donor into `.atlas/temporary/donors/` MUST run `.atlas/scripts/verify-donor-quarantine.sh` before committing that donor's staging, and quarantine any violation it reports using the exact convention above.
4. Census, license evidence, and provenance recording (`.atlas/census/donors/`, `.atlas/licenses/donors/`, `.atlas/provenance/donors/`) are unaffected by this contract -- they operate on donor source as data either way. This contract only concerns paths that a live agent-tooling convention would otherwise treat as instructions/configuration.
5. This is a repository-level, filesystem-based control. It does not depend on, and is not a substitute for, any specific agent runtime's own sandboxing; it exists so that *no* agent-tooling convention (present or future, Claude Code's or any other's) can pick up donor content as configuration merely by walking the filesystem.

## Verification

`.atlas/scripts/verify-donor-quarantine.sh` scans `.atlas/temporary/donors/` for any live (non-quarantined) match and exits non-zero if one is found, printing the exact violating path(s) and the rename convention to apply. Run it any time a donor is staged or re-staged:

```sh
.atlas/scripts/verify-donor-quarantine.sh
```

As of this contract's introduction, the scan is clean: 54 paths across 10 donors (`arrow`, `borg`, `buck2`, `duumbi`, `fastcdc-rs`, `iris`, `openrewrite`, `security/keycloak`, `wasmtime`, plus scattered root-level `CLAUDE.md`/`AGENTS.md`/`.mcp.json` files in several others) were quarantined under this contract's introduction. The 2026-09-22 `copilot-instructions.md`/`.github/instructions/*.instructions.md` extension (see Trigger, above) quarantined 5 additional paths (`ast-grep`, `arrow`, `security/keycloak`, and two under `llvm-project/.github/instructions/`); the scan is clean again after that quarantine and after staging the `mlir`/`xdsl`/`egglog`/`wasm-component-model`/`wasm-spec`/`capnproto`/`rkyv`/`agentir` donors (`egglog` alone contributed a live `.claude/skills/` directory and a root `CLAUDE.md`, both quarantined under the pre-existing convention).

## Non-goals

- This contract does not evaluate whether any quarantined content is itself malicious -- that is a census/disposition question (`.atlas/roadmap/DONOR-ABSORPTION-PLAN.toml`), separate from whether it is *discoverable as live configuration*, which is what this contract closes.
- This contract does not attempt to enumerate every possible agent-tooling convention's file-naming scheme beyond what has actually been observed to matter (`.claude`, `.codex`, `.cursor`, `CLAUDE.md`, `AGENTS.md`, `.mcp.json`, `copilot-instructions.md`, `.github/instructions/*.instructions.md`). If a new class of risk is discovered, extend the verification script's pattern list and this contract's invariant list together, and re-run the scan across all existing donors.
