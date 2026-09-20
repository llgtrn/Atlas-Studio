---
id: donor-zed-census
type: census
status: initial_component_license_mapped
canonical: true
---
# Zed Donor Census

Zed is approved as Atlas donor lane 10: IDE, editor, workspace and agent interaction reference.

Pinned source:

```text
remote = https://github.com/zed-industries/zed
branch = main
commit = 0eda7703f6c88aa08a25c1d2105ff1ca46f775d4
tree = 29b1e214fd75837a66a07137eec7a41ca05ecf4c
clone = .atlas/temporary/donors/ide/zed
```

Atlas must not become a renamed Zed fork. Zed is donor/reference material only.

## License Map

The component map is machine-readable at:

```text
.atlas/provenance/donors/zed/license-map.json
```

Observed split:

```text
GPL-3.0-or-later:
  application/editor/workspace/agent crates such as editor, workspace, worktree,
  project, multi_buffer, buffer_diff, git, terminal, task, agent, agent_ui,
  agent_servers, acp_thread, acp_tools and the main zed app crate

Apache-2.0:
  GPUI and support crates such as gpui, gpui_platform, gpui_wgpu, gpui_web,
  collections, extension_api, http_client, reqwest_client, scheduler, sum_tree,
  util, watch, zlog and ztracing

Mixed/review-required:
  syntax_theme carries both Apache and GPL license files and requires
  component-level review before any reuse classification can be narrowed.
```

Reuse classification:

```text
GPL application code        -> PRINCIPLE_EXTRACTION_ONLY
Apache support/GPUI crates  -> DIRECT_REUSE_ALLOWED_WITH_NOTICE after explicit Atlas review
Mixed components            -> COMPONENT_REVIEW_REQUIRED
```

## Atlas Mapping

Study targets:

```text
apps/ui        world canvas/editor/workspace interaction patterns
runtime/work   task/thread/workflow interaction model
adapter/provider provider and collaboration boundaries
core/binding   editor action to semantic work binding concepts
```

Current status:

```text
ingestion = CLONED
census = INITIAL_COMPONENT_LICENSE_MAPPED
runtime_dependency = REFERENCE_ONLY
native_implementation = PENDING_ATLAS_NATIVE
```
