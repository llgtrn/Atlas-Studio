---
id: atlas.census.donors.opendesign
type: donor-census
status: active
canonical: true
---
# OpenDesign Donor Census

## Source

- Donor: `OpenDesign`
- Remote: `https://github.com/vustudio/opendesign.git`
- Commit: `b4e69ac61b50576298f9f564603e5a4beb27417f`
- Tree: `9e93376bfd915ddb2df03ed6ac1d043e6141c3ae`
- Clone path: `.atlas/temporary/donors/opendesign`
- License: Apache-2.0, preserved at `.atlas/licenses/donors/opendesign/LICENSE`
- Donor gap: `ATLAS_NATIVE_DESIGN_AUTHORING_SUBSTRATE`

## Census State

This is a bounded first deep census for design-lane admission. It is sufficient to classify major subsystems and map Atlas target responsibilities. It does not authorize backend reuse or permanent OpenDesign architecture.

## Repository Topology

- `apps/daemon/`: local privileged daemon, Express server, project filesystem, SQLite persistence, agent spawning, provider/BYOK proxy, media generation dispatch, artifact lint, deployment helpers.
- `apps/web/`: Next/React web UI surface for chat, project/artifact workspace and preview.
- `apps/desktop/`: optional Electron shell and sidecar integration.
- `packages/contracts/`: shared API/SSE/types exported through `src/index.ts`, including app config, artifacts, chat, comments, files, projects, proxy, registry, version, critique and prompts.
- `packages/platform/`: process spawning/platform helpers with Windows command-line safety handling.
- `packages/sidecar-proto/`: sidecar stamp and IPC protocol. Exact stamp fields are `app`, `mode`, `namespace`, `ipc`, `source`.
- `packages/sidecar/`: runtime sidecar support.
- `design-systems/`: many `DESIGN.md` systems with brand/taste/token guidance.
- `skills/`: many `SKILL.md` artifact workflows and examples, including prototype, deck, dashboard, mobile, motion, critique and HTML-PPT skills.
- `prompt-templates/`: image/video/audio prompt templates with metadata and provenance.
- `templates/`: HTML deck/framework seed templates.
- `specs/`, `story/`, `tools/`: design product planning, story material and repository tooling.

## Central Mechanisms Observed

- `package.json` describes the product as local-first design software that auto-detects installed code-agent CLIs, runs design skills and design systems, and streams artifacts into a sandboxed preview.
- `apps/daemon/src/server.ts` composes the daemon surface: project directories, upload handling, SQLite-backed project/conversation/template/tab persistence, artifact linting, agent/provider routing, BYOK proxy, media dispatch and preview-serving APIs.
- `apps/daemon/src/prompts/system.ts` composes the active prompt stack from official designer prompt, active design system, craft references, active skill, project metadata and template reference.
- `apps/daemon/src/prompts/official-system.ts` defines the artifact-first workflow: ask focused questions, plan, write files, verify, and emit a final `<artifact>` wrapper.
- `packages/contracts/src/index.ts` exports API and SSE contracts as a dedicated contract package.
- `packages/sidecar-proto/src/index.ts` validates sidecar stamp fields and IPC paths, including null-byte and absolute-path checks.
- `packages/platform/src/index.ts` centralizes process spawn behavior and command-invocation safety, including Windows command-line-budget concerns.

## KEEP

- Artifact-first interaction loop: structured user brief, visible plan, generated project files, sandboxed preview, final artifact handoff.
- Design system corpus shape: `DESIGN.md` as a durable design-system knowledge unit with palette, typography, spacing, components and taste guidance.
- Skill packaging concept: file-backed `SKILL.md` workflows with examples, templates, references and checklists.
- Prompt stack layering: official behavior + active design system + active skill + template/reference material.
- Preview sandbox concept for HTML artifacts, including explicit artifact linting before accepting output.
- Contract package boundary for typed API/SSE messages.
- Sidecar stamp concept: process identity is explicit rather than inferred from process title.
- Project workspace persistence concepts: projects, conversations, messages, tabs and user templates.

## ADAPT

- Design systems should become observed design corpus data first, then ADL/ATLAS/ATLASX design semantics. Atlas should ingest `design-systems/*/DESIGN.md`, not blindly adopt their taste.
- Skills should inform Atlas design-authoring workflows and artifact contracts, but Atlas skills must map to `apps/ui/src/design`, `apps/ui/src/editor/design`, and Rust validation APIs.
- OpenDesign's artifact model should become an Atlas design object model: view, component, layout, surface, region, token, typography, spacing, color, geometry, interaction, motion, responsive rule, accessibility rule and design objective.
- Sidecar/process-stamp ideas should influence Atlas Studio app process supervision only after Rust runtime boundaries exist.
- BYOK/provider proxy concepts should be represented as adapters if needed, but never as Atlas backend authority.

## REWRITE

- `apps/daemon/` backend authority must be rewritten into Rust owners before Atlas relies on it. Atlas backend/compiler/runtime remains Rust.
- Filesystem, process spawn, SQLite, provider proxy, deployment and media dispatch code belongs outside Atlas core. If retained, it is temporary frontend/adapter substrate only.
- JavaScript/TypeScript runtime orchestration must not become Atlas compiler, corpus or ATLASX authority.
- The OpenDesign project model must be refounded into Atlas semantic project/design objects connected to the same system graph, not a separate design database.
- Security checks such as SSRF guard, upload limits and path validation must be reimplemented through Atlas-native admission/security semantics before any corpus build uses them.

## REJECT

- Copying OpenDesign root topology into Atlas root.
- Treating OpenDesign generated artifacts as canonical design truth.
- Letting UI-local state become the authoritative Atlas design graph.
- Letting OpenDesign provider/backend code become a permanent Atlas backend.
- Running donor build scripts, package installs, postinstall hooks, tests or generators during ingestion.
- Treating bundled design systems as Atlas style defaults.

## Atlas Target Mapping

- `core/rule/design/*`: design ADL and validation primitives.
- `core/model/*`: typed design objects and provenance/evidence classifications.
- `runtime/compile/*`: ADL/ATLAS design normalization into ATLASX shards.
- `runtime/query/*`: design/system graph projection queries.
- `runtime/materialize/*`: bounded design-to-TypeScript/Rust materialization.
- `adapter/donor/*`: donor provenance/license capture and safe static observation.
- `adapter/security/*`: URL/path/process/admission concepts extracted from donor security patterns.
- `apps/ui/src/design/*`: Atlas-native design workspace, token explorer, component/composition views and preview surfaces.
- `apps/ui/src/editor/design/*`: visual editing projections over Atlas semantics.

## Atlas Design Primitives Informed

OpenDesign directly supports the need for:

- `view`
- `component`
- `layout`
- `surface`
- `region`
- `design_system`
- `theme`
- `token`
- `typography`
- `spacing`
- `color`
- `geometry`
- `interaction`
- `gesture`
- `shortcut`
- `motion`
- `responsive_rule`
- `accessibility_rule`
- `visual_constraint`
- `design_objective`

## Security Notes

OpenDesign is untrusted donor input. It includes local daemon code, process spawning, uploads, provider proxying, package scripts and media tooling. None of it may be executed during Atlas ingestion. Any reused idea must pass Atlas security admission and be rewritten into the proper Rust/TS boundary.

## Decision

OpenDesign is `CLONED` and `DEEP_CENSUSED` for initial design-lane architecture planning. It remains in `DEEP_FORK_TRANSITION_ALLOWED` only for UI/design substrate exploration and must converge into Atlas-owned `apps/ui` semantics.
