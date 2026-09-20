---
id: atlas.reports.tools-census-2026-09-20
type: report
status: active
canonical: true
---
# Tools Census

`tools/` is transitional repository engineering machinery. Product/compiler/runtime semantics must move to Rust owners under `core/`, `runtime/`, `adapter/`, or thin apps.

| Owner | Classification | Target / Reason |
| --- | --- | --- |
| `agent` | KEEP_AS_TOOL | Local agent and skill smoke machinery; repository engineering only. |
| `architecture` | KEEP_AS_TOOL | Architecture verification and query scripts until Rust checks cover them. |
| `benchmark` | KEEP_AS_TOOL | Benchmark harness and fixtures; can remain as tool-owned evidence machinery. |
| `build` | KEEP_AS_TOOL | Build/bootstrap/audit helpers; not product runtime authority. |
| `canonical-shards` | MIGRATE_TO_RUNTIME | JSONL shard handling is semantic corpus/ATLASX debt; replace with Rust runtime corpus/shard code. |
| `capabilities` | MIGRATE_TO_CORE | Capability vocabulary and authority concepts belong in typed core/rule/model structures; scripts are transitional. |
| `ci` | KEEP_AS_TOOL | CI policy verification helper scripts. |
| `cloud-dispatch` | KEEP_AS_TOOL | Development dispatch helper, not Atlas runtime. |
| `db` | OBSOLETE_DELETE | Chronica-era database helpers; no Atlas database root authority. Preserve only if a concrete adapter need is proven. |
| `dev` | KEEP_AS_TOOL | Developer process helpers. |
| `docs` | KEEP_AS_TOOL | Documentation generation/lint helpers. |
| `docs-atlas` | MIGRATE_TO_RUNTIME | Documentation graph construction should converge into Rust source/docs ingestion. |
| `first-l4` | OBSOLETE_DELETE | Chronica-era slice material; no current Atlas owner. |
| `git-hooks` | KEEP_AS_TOOL | Repository engineering hooks. |
| `ops-logistics` | MIGRATE_TO_RUNTIME | Operational semantics should not remain JavaScript backend logic; extract only Atlas-relevant workflow concepts. |
| `packaging` | KEEP_AS_TOOL | Release/package scaffolding helpers. |
| `parity` | KEEP_AS_TOOL | Transitional parity reports until native Rust checks replace them. |
| `platform-plane` | MIGRATE_TO_CORE | Trust/platform claims should become typed Atlas security/rule semantics. |
| `reality-atlas` | MIGRATE_TO_RUNTIME | Repository reality checks belong in Rust ingestion/verification runtime. |
| `reconcile` | MIGRATE_TO_RUNTIME | Reconciliation logic belongs in Rust compile/link/materialize owners. |
| `refoundation` | KEEP_AS_TOOL | Migration controller and donor transition machinery; bounded and eventually removable. |
| `release` | KEEP_AS_TOOL | Release engineering. |
| `repo` | KEEP_AS_TOOL | Repository bootstrap and donor clone helpers, with donor intake moving to `adapter/donor`. |
| `subdb` | MIGRATE_TO_ADAPTER | External storage/subdatabase mechanics belong under adapters if still needed. |
| `system-atlas` | MIGRATE_TO_RUNTIME | Current JavaScript system graph logic must be replaced by Rust runtime/core graph and ATLASX logic. |
| `testing` | KEEP_AS_TOOL | Smoke harnesses only. |
| `tracking` | OBSOLETE_DELETE | Chronica-era progress database helpers unless a current Atlas reporting need is proven. |
| `universal-graph` | MIGRATE_TO_CORE | Generic graph schemas should become typed `core/model` and `core/rule` concepts. |

No new Atlas engine behavior should be added to `.mjs` tooling after this census.
