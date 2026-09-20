#!/usr/bin/env node
// census-schema.mjs — create the LITERAL capability census tables in docs/capabilities.db.
// This is the source-backed, named, deduped inventory (NO stub/placeholder rows allowed).
// The old `capability` (slice + donor_capability stub) table is kept for parity continuity but
// the census lives in source_capability + canonical_capability + provenance.
import Database from 'better-sqlite3'
import { join } from 'node:path'
import { CAPABILITIES_DB } from '../_paths.mjs'

const db = new Database(CAPABILITIES_DB)
db.pragma('journal_mode = WAL')

db.exec(`
-- One row per NAMED capability extracted from a donor's real source. Every row MUST cite source files.
CREATE TABLE IF NOT EXISTS source_capability (
  id              INTEGER PRIMARY KEY,
  donor           TEXT NOT NULL,
  module          TEXT,                       -- the donor module/shard this came from
  canonical_name  TEXT NOT NULL,              -- verb-noun, e.g. "post-purchase-invoice"
  source_files    TEXT NOT NULL,              -- comma/space list of real files (REQUIRED, non-empty)
  source_symbols  TEXT,                       -- routes/models/jobs/UI/CLI symbols
  business_behavior TEXT,
  technical_behavior TEXT,
  inputs          TEXT,
  outputs         TEXT,
  persistence     TEXT,                       -- DB objects touched
  surface         TEXT,                       -- api|ui|cli|worker|model|automation|governance
  side_effect_class TEXT,                     -- none|internal_write|artifact_write|external_read|external_write|money|...
  moves_money     INTEGER DEFAULT 0,
  requires_approval INTEGER DEFAULT 0,
  external_services TEXT,
  target_crate    TEXT,                       -- Chronica target
  target_module   TEXT,
  canonical_id    INTEGER,                    -- FK -> canonical_capability (dedupe link), NULL until mapped
  created_pass    TEXT
);
CREATE INDEX IF NOT EXISTS idx_sc_donor ON source_capability(donor);
CREATE INDEX IF NOT EXISTS idx_sc_canon ON source_capability(canonical_id);
CREATE INDEX IF NOT EXISTS idx_sc_money ON source_capability(moves_money);

-- The deduped canonical Chronica capabilities (many source rows -> one canonical).
CREATE TABLE IF NOT EXISTS canonical_capability (
  id              INTEGER PRIMARY KEY,
  key             TEXT UNIQUE NOT NULL,       -- e.g. "erp.purchase_invoice.post"
  canonical_name  TEXT NOT NULL,
  domain          TEXT,                       -- erp|commerce|runtime|workflow|osint|observability|...
  target_crate    TEXT,
  target_module   TEXT,
  side_effect_class TEXT,
  moves_money     INTEGER DEFAULT 0,
  requires_approval INTEGER DEFAULT 0,
  acceptance_criteria TEXT,                   -- REQUIRED for completeness
  required_tests  TEXT,
  financial_control_test TEXT,                -- REQUIRED if moves_money or external_write
  acceptance_test TEXT,
  status          TEXT DEFAULT 'unimplemented', -- verified|implemented_unverified|unimplemented|excluded|blocked
  exclusion_note  TEXT,                        -- REQUIRED if status=excluded
  blocker         TEXT,                        -- REQUIRED if status=blocked
  slice           INTEGER,                     -- linked registry slice if any
  donor_count     INTEGER DEFAULT 0            -- how many donors contribute (provenance breadth)
);
CREATE INDEX IF NOT EXISTS idx_cc_status ON canonical_capability(status);
CREATE INDEX IF NOT EXISTS idx_cc_domain ON canonical_capability(domain);
CREATE INDEX IF NOT EXISTS idx_cc_money ON canonical_capability(moves_money);

-- Many-to-one provenance: which donor source capabilities map to which canonical.
CREATE TABLE IF NOT EXISTS provenance (
  source_id     INTEGER NOT NULL,
  canonical_id  INTEGER NOT NULL,
  PRIMARY KEY (source_id, canonical_id)
);

-- Track which donor modules have been extracted (so we can prove exhaustiveness / resume sharding).
CREATE TABLE IF NOT EXISTS census_coverage (
  donor          TEXT NOT NULL,
  module         TEXT NOT NULL,
  status         TEXT NOT NULL,              -- extracted | blocked
  capabilities   INTEGER DEFAULT 0,
  coverage_note  TEXT,
  blocker        TEXT,
  PRIMARY KEY (donor, module)
);

-- File-level donor census: the hard proof layer below source_capability.
-- One row records either a concrete donor file or a pruned artifact directory.
-- The final 100% gate is NOT satisfied until every non-artifact source/behavior row
-- is reviewed/mapped/excluded/blocked instead of left as *_review_pending.
CREATE TABLE IF NOT EXISTS donor_file_census (
  donor            TEXT NOT NULL,
  path             TEXT NOT NULL,            -- donor-relative slash path; directories end with '/'
  is_directory     INTEGER DEFAULT 0,
  kind             TEXT NOT NULL,            -- source|documentation|config_schema|vendor_dir|...
  classification   TEXT NOT NULL,            -- capability_review_pending|mapped|non_behavioral_support|...
  read_status      TEXT NOT NULL,            -- unread_pending|classified_not_read|reviewed|blocked
  mapped_source_ids TEXT,                    -- comma-separated source_capability.id values once mapped
  exclusion_reason TEXT,
  size_bytes       INTEGER,
  mtime_ms         INTEGER,
  sha256           TEXT,
  reviewed_by      TEXT,
  reviewed_at      TEXT,
  PRIMARY KEY (donor, path)
);
CREATE INDEX IF NOT EXISTS idx_dfc_donor ON donor_file_census(donor);
CREATE INDEX IF NOT EXISTS idx_dfc_class ON donor_file_census(classification);
CREATE INDEX IF NOT EXISTS idx_dfc_read ON donor_file_census(read_status);
CREATE INDEX IF NOT EXISTS idx_dfc_kind ON donor_file_census(kind);

CREATE TABLE IF NOT EXISTS source_file_capability_link (
  source_id       INTEGER NOT NULL,
  donor           TEXT NOT NULL,
  path            TEXT NOT NULL,
  symbol          TEXT,
  evidence_note   TEXT,
  PRIMARY KEY (source_id, donor, path, symbol)
);
CREATE INDEX IF NOT EXISTS idx_sfcl_file ON source_file_capability_link(donor, path);
CREATE INDEX IF NOT EXISTS idx_sfcl_source ON source_file_capability_link(source_id);

-- ════════════════════════════════════════════════════════════════════════════
-- SURVIVABLE SIDE-TABLES — keyed on the STABLE canonical_capability.key (TEXT),
-- NOT the id (which apply-clusters reassigns on every rebuild). These persist
-- across census wipe-and-rebuild; reapply-overrides.mjs re-projects them onto the
-- freshly rebuilt canonical_capability rows. This is what makes the numbered docs
-- and the DB the TWO synchronized tracking surfaces — doc-authored status survives.
-- ════════════════════════════════════════════════════════════════════════════

-- The slice <-> canonical BRIDGE (the "both sides" link). Survives canonical rebuild
-- because it joins on canonical_key. One canonical may link to several slices.
CREATE TABLE IF NOT EXISTS slice_canonical (
  slice         INTEGER NOT NULL,            -- FK -> capability.slice (the 195 roadmap slices)
  canonical_key TEXT    NOT NULL,            -- FK -> canonical_capability.key (stable text)
  match_method  TEXT    NOT NULL,            -- manifest | domain | crate | keyprefix | title
  confidence    TEXT    NOT NULL,            -- high | medium | low
  reviewed      INTEGER DEFAULT 0,           -- 1 once a human/agent confirms the edge
  note          TEXT,
  PRIMARY KEY (slice, canonical_key)
);
CREATE INDEX IF NOT EXISTS idx_slcan_key  ON slice_canonical(canonical_key);
CREATE INDEX IF NOT EXISTS idx_slcan_conf ON slice_canonical(confidence);

-- Doc-authored canonical status (the docs->DB write-back target). An agent flips a
-- checkbox in a numbered doc, sync-docs.mjs writes HERE, reapply-overrides.mjs pushes
-- it onto canonical_capability after a rebuild. The DB stays system-of-record; this is
-- the durable journal of progress that the wipe cannot erase.
CREATE TABLE IF NOT EXISTS canonical_status_override (
  canonical_key   TEXT PRIMARY KEY,          -- FK -> canonical_capability.key
  status          TEXT,                      -- verified | implemented_unverified | unimplemented | blocked | excluded
  acceptance_test TEXT,
  financial_control_test TEXT,
  moves_money     INTEGER,                   -- survivable money correction: when file-level donor proof contradicts the census moves_money (e.g. a JE is minted), pin the corrected value here so a census re-extraction cannot silently revert it. NULL = no override.
  requires_approval INTEGER,                 -- survivable approval-flag correction (FAIL-SAFE direction only: a moves_money cap should require approval). Census derives this from source rows; when a money cap lands with requires_approval=0, pin the corrected 1 here so a rebuild cannot revert it. NULL = no override.
  blocker         TEXT,
  set_at          TEXT,
  set_by          TEXT                        -- 'docs:sync' | 'parity:set' | agent id
);

-- CHRONICA-SIDE implementation evidence: when a capability is verified, record WHAT code proves it,
-- WHERE it lives, WHEN (real timestamp), and which DONOR source it was modeled against. This closes
-- the gap where 'verified' only pointed at a test NAME — now the DB tracks the actual code. Survives
-- rebuild (keyed on canonical_key). record-verified.mjs refuses to write a row whose test symbol is
-- not actually present in the impl file, so 'verified' can never dangle.
CREATE TABLE IF NOT EXISTS impl_evidence (
  canonical_key   TEXT PRIMARY KEY,          -- FK -> canonical_capability.key
  impl_file       TEXT,                      -- the Chronica source file, e.g. crates/chronica-erp/src/accounting.rs
  impl_symbols    TEXT,                      -- comma-list of fns/structs added (the native code), e.g. "invoice_action,PurchaseInvoice"
  test_file       TEXT,                      -- file holding the proving test (same as impl_file when co-located)
  test_symbol     TEXT,                      -- the test fn name, e.g. create_purchase_invoice_money_chain_full
  donor_source    TEXT,                      -- the donor file(s) read to model it, e.g. Temporary/erpnext-main/.../purchase_invoice.py
  verified_at     TEXT,                      -- ISO-8601 real timestamp of verification
  verified_by     TEXT,                      -- e.g. 'loop:verify'
  notes           TEXT
);
CREATE INDEX IF NOT EXISTS idx_impl_file ON impl_evidence(impl_file);

-- The donor-absorption ledger folded onto the DB surface (was _machine/donor-absorption/
-- absorption-map.generated.json). status lifecycle: spec -> red -> green -> verified.
CREATE TABLE IF NOT EXISTS donor_absorption (
  donor             TEXT PRIMARY KEY,
  disposition       TEXT,                     -- native_absorption | native_ui_absorption | discard_or_reference_only
  target_module     TEXT,
  capability_absorbed TEXT,
  optional_bridge   INTEGER DEFAULT 0,
  priority          TEXT,
  status            TEXT DEFAULT 'spec',
  test_ids          TEXT,                     -- JSON array
  note              TEXT
);

-- The package-retirement (strangler) ledger folded onto the DB surface (was
-- _machine/architecture/package-retirement-map.generated.json). active -> strangling -> retired.
CREATE TABLE IF NOT EXISTS package_retirement (
  package           TEXT PRIMARY KEY,
  replacement_crate TEXT,
  covered_by_slices TEXT,                     -- JSON array of slice numbers
  status            TEXT DEFAULT 'active',    -- active | strangling | retired | transitional
  rust_route_available INTEGER,
  test_ids          TEXT,                     -- JSON array
  note              TEXT
);

-- The continuous-execution state folded onto the DB surface (was
-- _machine/parity/continuous-execution-state.json). One row per tracked phase/flag.
CREATE TABLE IF NOT EXISTS execution_state (
  k                 TEXT PRIMARY KEY,         -- e.g. 'phase0.kernel-skeleton' | 'native_runtime_execution_complete'
  v                 TEXT,                     -- value / status / boolean-as-text
  detail            TEXT,
  set_at            TEXT
);

-- The SHARED CLAUDE<->CODEX coordination channel: an append-only note log so the two agents working
-- this repo from opposite ends (Codex discovers donor capabilities; Claude implements + verifies them)
-- can hand off explicitly instead of only through capability rows. Survivable side-table (keyed on a
-- stable id); mirrored to docs/_machine/agent-notes.md for human + git visibility. Written via tools/capabilities/
-- agent-note.mjs; both agents READ open notes at the start of each iteration. See AGENTS.md.
CREATE TABLE IF NOT EXISTS agent_note (
  id          INTEGER PRIMARY KEY AUTOINCREMENT,
  author      TEXT NOT NULL,                  -- 'claude' | 'codex'
  kind        TEXT NOT NULL,                  -- census_correction | donor_warning | request | handoff
  status      TEXT NOT NULL DEFAULT 'open',   -- open | acked | resolved   (handoff/correction/warning auto-resolve; request stays open until the other acks/resolves)
  capability_key TEXT,                        -- optional FK-ish -> canonical_capability.key the note is about
  donor       TEXT,                           -- optional donor / source_files the note concerns
  body        TEXT NOT NULL,                  -- the message (what + why + what to do)
  created_at  TEXT NOT NULL,                  -- ISO-8601
  resolved_by TEXT,                           -- who acked/resolved (the OTHER agent)
  resolved_at TEXT
);
CREATE INDEX IF NOT EXISTS idx_agent_note_status ON agent_note(status);
CREATE INDEX IF NOT EXISTS idx_agent_note_kind ON agent_note(kind);

-- The STANDARD ISSUE / FINDING tracker: ONE uniform place for audit findings, reconciliation gaps,
-- money-flag questions, blockers, and product/doc TODOs — instead of scattering them across agent_note,
-- docs/design/audit-*.md, architecture_target_gap, blocker fields, and chat. Survivable side-table: the
-- committed SOURCE is tools/capabilities/tracking-issues.json, re-applied by reconcile-tracking-issues.mjs
-- on every rebuild (caps.db is gitignored, so the JSON is the durable record). The combined state +
-- progress + issues snapshot projects to docs/_generated/026-status.md. Written via track.mjs (pnpm caps:track).
CREATE TABLE IF NOT EXISTS tracking_issue (
  id              TEXT PRIMARY KEY,            -- stable slug, e.g. 'money-get-details-overlabel'
  severity        TEXT NOT NULL,               -- P0 | P1 | P2 | P3 | P4
  area            TEXT NOT NULL,               -- money | architecture | evidence | product | tenancy | security | docs | meta | tests | process
  title           TEXT NOT NULL,
  detail          TEXT,
  evidence_ref    TEXT,                        -- cap-key / file:line / arch-node / commit
  status          TEXT NOT NULL DEFAULT 'open',-- open | in_progress | fixed | wontfix | deferred
  source          TEXT,                        -- db-truth-audit-2026-06 | audit-2026-06 | db-reconcile | manual | ...
  autonomous_safe INTEGER DEFAULT 0,           -- 1 = fixable without a human decision; 0 = needs human / STOP-gate
  created_at      TEXT,
  resolved_at     TEXT,
  resolved_by     TEXT,
  fix_ref         TEXT                          -- commit hash / file that resolved it
);
CREATE INDEX IF NOT EXISTS idx_tracking_issue_status ON tracking_issue(status);
CREATE INDEX IF NOT EXISTS idx_tracking_issue_sev ON tracking_issue(severity);
CREATE INDEX IF NOT EXISTS idx_tracking_issue_area ON tracking_issue(area);

-- The 4th tracking dimension: OPPORTUNITIES — what SHOULD exist, what could be improved/integrated,
-- what creates leverage. Each is a first-class strategic object: not "what's broken" (tracking_issue)
-- but "what's worth building". The Architecture Intelligence backlog. Ranked by Expected Value
-- (ev = impact*reach*leverage) and priority (ev*confidence/effort). 'leverage' is tracked EXPLICITLY
-- (foundational work that unlocks other work scores high). Durable SOURCE = the committed
-- tools/capabilities/opportunities.json; reconcile-opportunities.mjs re-applies it + recomputes ev/priority
-- on every caps:rebuild. Surfaced via pnpm caps:track opp + docs/_generated/027-opportunities.md. Donor-grounded:
-- source_repo names the Temporary/ donor(s) (or cross-repo:a+b, or native) that inspired it.
CREATE TABLE IF NOT EXISTS opportunity (
  id              TEXT PRIMARY KEY,            -- stable slug, e.g. 'synergy-oasis-sim-into-feed'
  title           TEXT NOT NULL,
  description     TEXT,
  category        TEXT NOT NULL,               -- capability|workflow|integration|dedup|architecture|product|revenue|automation|ux|observability|security|testing|synergy
  impact          INTEGER,                     -- 1-5 value if realized
  reach           INTEGER,                     -- 1-5 how many flows/companies/users/domains it touches
  leverage        INTEGER,                     -- 1-5 ARCHITECTURAL LEVERAGE: unlocks other work / is foundational
  effort          INTEGER,                     -- 1-5 build cost
  confidence      INTEGER,                     -- 1-5 certainty it is real + will pay off
  ev              INTEGER,                     -- computed: impact * reach * leverage  (Expected Value)
  priority        REAL,                        -- computed: ev * confidence / effort  (ranks the backlog)
  source_repo     TEXT,                        -- donor(s) e.g. 'erpnext-main' | 'cross-repo:posthog+temporal' | 'native'
  related_capabilities TEXT,                   -- comma list of canonical_capability.key
  related_architecture_nodes TEXT,             -- comma list of architecture_node id
  status          TEXT NOT NULL DEFAULT 'discovered', -- discovered|validated|planned|executing|implemented|rejected
  created_at      TEXT,
  updated_at      TEXT,
  notes           TEXT
);
CREATE INDEX IF NOT EXISTS idx_opportunity_status ON opportunity(status);
CREATE INDEX IF NOT EXISTS idx_opportunity_category ON opportunity(category);
CREATE INDEX IF NOT EXISTS idx_opportunity_priority ON opportunity(priority);

-- DIMENSION 5: STRATEGY GRAPH — the tracker stops being a list and becomes a graph. Directed edges
-- between any two tracked objects (opportunity|capability|issue|donor|pillar). relation: 'unlocks'
-- (completing FROM raises/enables TO), 'depends_on' (FROM needs TO first), 'blocks' (open FROM stops
-- TO), 'obsoletes' (FROM makes TO unnecessary), 'contributes_to' (FROM advances a TO pillar),
-- 'relates'. This is what lets the system answer: which opportunity unlocks the most others (leverage
-- path), which is a dead-end, what an open issue is blocking, and what a finished node changes.
-- Durable SOURCE = committed strategy-edges.json; reconcile-strategy-edges.mjs re-applies + recomputes
-- the graph leverage on every caps:rebuild.
CREATE TABLE IF NOT EXISTS strategy_edge (
  id          TEXT PRIMARY KEY,                -- stable: '<from>--<relation>-->.<to>' slug
  from_kind   TEXT NOT NULL,                   -- opportunity|capability|issue|donor|pillar
  from_id     TEXT NOT NULL,                   -- opportunity.id | canonical_capability.key | tracking_issue.id | donor | end_state_pillar.id
  relation    TEXT NOT NULL,                   -- unlocks|depends_on|blocks|obsoletes|contributes_to|relates
  to_kind     TEXT NOT NULL,
  to_id       TEXT NOT NULL,
  weight      INTEGER DEFAULT 3,               -- 1-5 strength of the relation
  rationale   TEXT,
  source      TEXT,
  created_at  TEXT
);
CREATE INDEX IF NOT EXISTS idx_strategy_edge_from ON strategy_edge(from_id);
CREATE INDEX IF NOT EXISTS idx_strategy_edge_to ON strategy_edge(to_id);
CREATE INDEX IF NOT EXISTS idx_strategy_edge_rel ON strategy_edge(relation);

-- DIMENSION 7: END-STATE — the formalized Company-OS target. Each pillar = a major end-state
-- subsystem (Agent System, Workflow Engine, Revenue Engine, Observability, Security, ...) with the
-- domains/crates that realize it and a target weight. distance_to_end_state is COMPUTED (verified caps
-- in the pillar's domains / target) so the system can report "Company OS Completion: Architecture 42%,
-- Autonomy 9%". Durable SOURCE = committed end-state.json; reconcile-end-state.mjs re-applies it.
CREATE TABLE IF NOT EXISTS end_state_pillar (
  id              TEXT PRIMARY KEY,            -- slug, e.g. 'agent-system'
  name            TEXT NOT NULL,
  description     TEXT,
  domains         TEXT,                        -- comma list of canonical_capability.domain that count toward this pillar
  crates          TEXT,                        -- comma list of chronica-* crates
  target_caps     INTEGER,                     -- target number of verified caps for "done" (denominator for distance)
  weight          INTEGER DEFAULT 3,           -- 1-5 importance of this pillar to the end-state
  ordinal         INTEGER,                     -- display order
  notes           TEXT
);

-- ════════════════════════════════════════════════════════════════════════════
-- STRATEGIC COMPANY BRAIN — DIM 10-15. These optimize the CORRECTNESS and PURPOSE
-- of the system, not just what to build next: are the bets still true (10), why did
-- we choose X (11), does a capability produce value (12), how do we compare outside
-- (13), what works-but-is-expensive (14), and what maximizes the org's own evolution
-- (15). All are survivable side-tables: the committed JSON seed is the durable record
-- (caps.db is gitignored), re-applied by reconcile-<dim>.mjs on every caps:rebuild.
-- ════════════════════════════════════════════════════════════════════════════

-- DIM 10: ASSUMPTION TRACKING. Every large architecture rests on a hypothesis treated
-- as fact. Track the bet + the strongest honest counter so we never build 6 months on a
-- wrong premise. fragility is COMPUTED (risk_if_wrong x (6 - confidence)); a fragile +
-- unvalidated assumption casts a warning onto everything it underpins.
CREATE TABLE IF NOT EXISTS assumption (
  id                TEXT PRIMARY KEY,
  statement         TEXT NOT NULL,
  underpins         TEXT,                       -- opportunity/decision/pillar/capability id(s) or area this bet supports
  evidence_for      TEXT,
  evidence_against  TEXT,
  confidence        INTEGER,                    -- 1-5 how sure we are it is TRUE
  risk_if_wrong     INTEGER,                    -- 1-5 blast radius if it is false
  validation_status TEXT DEFAULT 'unvalidated', -- unvalidated|validating|validated|invalidated
  fragility         INTEGER,                    -- computed: risk_if_wrong * (6 - confidence)  (1-25, higher = more dangerous)
  source            TEXT,
  created_at        TEXT,
  updated_at        TEXT,
  notes             TEXT
);
CREATE INDEX IF NOT EXISTS idx_assumption_status ON assumption(validation_status);
CREATE INDEX IF NOT EXISTS idx_assumption_fragility ON assumption(fragility);

-- DIM 11: DECISION MEMORY (ADR). Why did we choose X? Recoverable in 2 years. alternatives
-- is a JSON array of {option, rejected_because}. Linked into the planning graph by the
-- related_* columns (decision rests on assumptions, implements opportunities, realizes caps).
CREATE TABLE IF NOT EXISTS architecture_decision (
  id                   TEXT PRIMARY KEY,
  title                TEXT NOT NULL,
  decision             TEXT,
  status               TEXT DEFAULT 'accepted', -- proposed|accepted|superseded|rejected
  context              TEXT,                    -- what forced the choice
  rationale            TEXT,
  alternatives         TEXT,                    -- JSON array of {option, rejected_because}
  consequences         TEXT,
  related_opportunities TEXT,                   -- comma list of opportunity.id
  related_assumptions  TEXT,                    -- comma list of assumption.id
  related_capabilities TEXT,                    -- comma list of canonical_capability.key
  supersedes           TEXT,                    -- decision id this replaces
  decided_at           TEXT,
  source               TEXT
);
CREATE INDEX IF NOT EXISTS idx_decision_status ON architecture_decision(status);

-- DIM 12: VALUE FLOW GRAPH. A capability is only worth building if it flows to value.
-- value_node kinds: capability|user_value|business_value|revenue. value_edge wires them
-- into a DAG terminating in revenue. A capability node with NO path to revenue = an ORPHAN
-- (computed in tracking-lib) — the warning this dimension exists to raise.
CREATE TABLE IF NOT EXISTS value_node (
  id     TEXT PRIMARY KEY,                      -- kind:slug
  kind   TEXT NOT NULL,                         -- capability|user_value|business_value|revenue
  label  TEXT NOT NULL,
  notes  TEXT
);
CREATE INDEX IF NOT EXISTS idx_value_node_kind ON value_node(kind);
CREATE TABLE IF NOT EXISTS value_edge (
  id        TEXT PRIMARY KEY,                   -- from|to
  from_id   TEXT NOT NULL,
  to_id     TEXT NOT NULL,
  weight    INTEGER DEFAULT 3,
  rationale TEXT
);
CREATE INDEX IF NOT EXISTS idx_value_edge_from ON value_edge(from_id);
CREATE INDEX IF NOT EXISTS idx_value_edge_to ON value_edge(to_id);

-- DIM 13: COMPETITIVE INTELLIGENCE. Chronica does not exist in a vacuum. Map competitor
-- features to our capabilities so we see gap/parity/advantage/differentiation honestly.
CREATE TABLE IF NOT EXISTS competitor (
  id     TEXT PRIMARY KEY,
  name   TEXT NOT NULL,
  market TEXT,                                  -- browser|ai_ide|agent_platform|company_os|erp
  notes  TEXT
);
CREATE TABLE IF NOT EXISTS competitor_feature (
  id                 TEXT PRIMARY KEY,
  competitor_id      TEXT NOT NULL,
  feature            TEXT NOT NULL,
  our_capability_ref TEXT,                      -- canonical_capability.key / area, or NULL if we lack it
  relation           TEXT,                      -- gap|parity|advantage|differentiation
  notes              TEXT
);
CREATE INDEX IF NOT EXISTS idx_compfeat_comp ON competitor_feature(competitor_id);
CREATE INDEX IF NOT EXISTS idx_compfeat_rel ON competitor_feature(relation);

-- DIM 14: ARCHITECTURAL DEBT. NOT a bug (those are tracking_issue). Debt = works today
-- but is expensive to keep or will cost to migrate. Tracked separately so it cannot hide
-- behind green tests. cost_load is COMPUTED (maintenance_cost + migration_cost) for ranking.
CREATE TABLE IF NOT EXISTS arch_debt (
  id                   TEXT PRIMARY KEY,
  title                TEXT NOT NULL,
  location             TEXT,                    -- crate/file/area
  kind                 TEXT,                    -- coupling|complexity|temporary|duplication|migration|operational
  description          TEXT,
  maintenance_cost     INTEGER,                 -- 1-5 ongoing carrying cost
  migration_cost       INTEGER,                 -- 1-5 cost to pay it down later
  status               TEXT DEFAULT 'active',   -- active|accepted|scheduled|paid
  related_capabilities TEXT,
  created_at           TEXT,
  notes                TEXT
);
CREATE INDEX IF NOT EXISTS idx_arch_debt_status ON arch_debt(status);
CREATE INDEX IF NOT EXISTS idx_arch_debt_kind ON arch_debt(kind);

-- DIM 15: ORGANISM-MODE EVOLUTION SCORING. Per opportunity, score future-evolution
-- potential (NOT near-term ROI): learning_value, optionality (future directions opened),
-- adaptability, knowledge_gain — each 1-5. evolution_score is COMPUTED (the product).
-- organism ranking (in tracking-lib) also folds in graph transitive-unlocks, surfacing
-- low-ROI-today / high-optionality-tomorrow plays the priority sort buries.
CREATE TABLE IF NOT EXISTS evolution_score (
  opportunity_id  TEXT PRIMARY KEY,            -- FK -> opportunity.id
  learning_value  INTEGER,                     -- 1-5 how much the org learns by building it
  optionality     INTEGER,                     -- 1-5 how many future directions it opens
  adaptability    INTEGER,                     -- 1-5 does it make the system more able to change
  knowledge_gain  INTEGER,                     -- 1-5 reusable knowledge/infra vs a point feature
  evolution_score INTEGER,                     -- computed product (clamped)
  rationale       TEXT
);

-- ════════════════════════════════════════════════════════════════════════════
-- THE TRUTH LAYER — sits UNDER all 15 dimensions, not beside them. It answers the
-- question the planning brain cannot answer for itself: "do I KNOW this, or do I
-- just THINK it?" Without it, a self-authored opinion and a runtime-proven fact sit
-- in the same graph at the same confidence — the failure mode where a planning brain
-- starts trusting its own inference as if it were evidence.
-- ════════════════════════════════════════════════════════════════════════════

-- EVIDENCE QUALITY: every meta-node (assumption/decision/opportunity/value_edge/
-- arch_debt/competitor_feature/pillar/external_signal) gets an evidence_level on a
-- 5-rung ladder and a provenance. The cardinal rule: EVIDENCE CAPS CONFIDENCE — an
-- L0 opinion can never be high-confidence (ceiling 0.40), only L4 production reality
-- reaches 1.0. intrinsic_confidence = min(stated, ceiling(level)); effective_confidence
-- is intrinsic propagated weakest-link down the support graph (reconcile-evidence).
--   L0 opinion · L1 inferred (read code/git/web, not verified) · L2 code-verified
--   (real code/test ref) · L3 runtime-verified (passing live/smoke) · L4 production-verified
-- Durable SOURCE = committed evidence.json; reconcile-evidence.mjs derives + propagates.
CREATE TABLE IF NOT EXISTS node_evidence (
  node_kind            TEXT NOT NULL,          -- assumption|decision|opportunity|value_edge|arch_debt|competitor_feature|pillar|external_signal
  node_id              TEXT NOT NULL,
  evidence_level       TEXT NOT NULL,          -- L0|L1|L2|L3|L4
  provenance           TEXT,                   -- claude-authored|discovery-workflow|user-stated|code-derived|test-derived|runtime-smoke|production|web-scan
  evidence_ref         TEXT,                   -- file:line|commit|test symbol|url|memory-note backing the level
  intrinsic_confidence REAL,                   -- 0-1, = min(stated_confidence, ceiling(evidence_level))
  effective_confidence REAL,                   -- 0-1, intrinsic propagated weakest-link down the support graph
  rationale            TEXT,
  assessed_at          TEXT,
  assessed_by          TEXT,
  PRIMARY KEY (node_kind, node_id)
);
CREATE INDEX IF NOT EXISTS idx_node_evidence_level ON node_evidence(evidence_level);
CREATE INDEX IF NOT EXISTS idx_node_evidence_kind ON node_evidence(node_kind);
CREATE INDEX IF NOT EXISTS idx_node_evidence_prov ON node_evidence(provenance);

-- EXTERNAL REALITY: the system otherwise only looks INWARD (caps.db/donors/its own
-- graph). This is the outward loop — dated developments in the markets + OSS/AI/
-- browser/agent ecosystems that should re-score the roadmap (the best roadmap for
-- 2025 is not the best for 2026). Each signal names what it AFFECTS + a recommended
-- action. Itself evidence-leveled (web-scan signals are L1). Durable SOURCE =
-- committed external-signals.json; refreshed by the scan-external-reality workflow.
CREATE TABLE IF NOT EXISTS external_signal (
  id                 TEXT PRIMARY KEY,
  observed_at        TEXT,                     -- when the development happened / was scanned
  source             TEXT,
  category           TEXT,                     -- competitor|company_os|agent_platform|ai_ide|browser|oss_ecosystem|ai_model|standards|market
  headline           TEXT NOT NULL,
  implication        TEXT,                     -- what it means for Chronica specifically
  affects_kind       TEXT,                     -- opportunity|assumption|pillar|area
  affects_id         TEXT,                     -- the roadmap node it re-scores (or an area name)
  recommended_action TEXT,
  url                TEXT,
  confidence         INTEGER                   -- 1-5 confidence in the signal itself
);
CREATE INDEX IF NOT EXISTS idx_external_signal_cat ON external_signal(category);
CREATE INDEX IF NOT EXISTS idx_external_signal_affects ON external_signal(affects_id);

-- ════════════════════════════════════════════════════════════════════════════
-- THE REALITY ENGINE (DIM 16) — the layer the whole stack ultimately answers to.
-- Everything above asks "is it implemented?"; this asks "did it actually HAPPEN?".
-- L3 = code exists + tests pass + verified. L4 = a real request flowed through it,
-- real (non-synthetic) data passed, a real user touched it, the expected outcome
-- occurred. L4 is NON-HAND-ASSIGNABLE — it is earned only by a recorded reality_event
-- with user_real + data_real, never claimed in evidence.json. That is the structural
-- answer to "who evidences the evidencer?": the top rung cannot be authored, only lived.
-- ════════════════════════════════════════════════════════════════════════════

-- REALITY EVENTS: the ledger of things that actually occurred in runtime/production. A smoke/live test
-- (real runtime, synthetic user+data) is L3_synthetic; only a real-user + real-data occurrence is
-- L4_production. The Reality Ratio = L4 nodes / total strategic nodes (computed in tracking-lib) — the
-- single most honest metric in the system. Durable SOURCE = committed reality-events.json.
CREATE TABLE IF NOT EXISTS reality_event (
  id               TEXT PRIMARY KEY,
  node_kind        TEXT,                       -- capability|assumption|decision|opportunity (what it is evidence FOR)
  node_id          TEXT,
  reality_level    TEXT NOT NULL,              -- L3_synthetic | L4_production
  event_type       TEXT,                       -- smoke_test|live_test|integration_run|real_user_action|real_money_flow|production_request
  occurred_at      TEXT,
  description      TEXT,                        -- what actually happened
  data_real        INTEGER DEFAULT 0,          -- did real (non-synthetic) data pass through?
  user_real        INTEGER DEFAULT 0,          -- did a real (non-test) user touch it?
  outcome_observed TEXT,                        -- the actual observed outcome
  evidence_ref     TEXT                         -- commit / log / url proving it occurred
);
CREATE INDEX IF NOT EXISTS idx_reality_event_level ON reality_event(reality_level);
CREATE INDEX IF NOT EXISTS idx_reality_event_node ON reality_event(node_kind, node_id);

-- OUTCOME GRAPH: results, not builds. "Built CQRS backbone" is not the point — "agent latency -35%,
-- throughput +120%" is. status: predicted (a target) -> measuring -> measured (real number observed).
-- Until something runs in reality, actual stays NULL and the system stays honest that it has shipped
-- capability, not outcome. Durable SOURCE = committed outcomes.json.
CREATE TABLE IF NOT EXISTS outcome (
  id           TEXT PRIMARY KEY,
  subject_kind TEXT,                            -- opportunity|capability
  subject_id   TEXT,
  metric       TEXT NOT NULL,                   -- e.g. 'agent decision latency'
  baseline     TEXT,
  target       TEXT,
  actual       TEXT,                            -- NULL until measured in reality
  unit         TEXT,
  status       TEXT DEFAULT 'predicted',        -- predicted | measuring | measured
  observed_at  TEXT,
  evidence_ref TEXT,
  notes        TEXT
);
CREATE INDEX IF NOT EXISTS idx_outcome_status ON outcome(status);

-- PREDICTION MARKET: every assumption/bet gets a FALSIFIABLE prediction + an expiry + a confidence, so
-- that later it resolves true/false/partial and the brain learns its OWN forecasting accuracy (Brier
-- score) instead of merely storing assumptions. This is how the planning brain starts grading itself
-- against reality. Durable SOURCE = committed predictions.json.
CREATE TABLE IF NOT EXISTS prediction (
  id             TEXT PRIMARY KEY,
  subject_kind   TEXT,                          -- assumption|opportunity|decision
  subject_id     TEXT,
  prediction     TEXT NOT NULL,                 -- a falsifiable, checkable claim
  predicted_at   TEXT,
  expiry         TEXT,                          -- when reality should be checked
  confidence     REAL,                          -- 0-1 at prediction time
  resolution     TEXT DEFAULT 'pending',        -- pending | true | false | partial
  resolved_at    TEXT,
  actual_outcome TEXT,
  brier          REAL,                           -- computed on resolution: (confidence - outcome_value)^2
  notes          TEXT
);
CREATE INDEX IF NOT EXISTS idx_prediction_resolution ON prediction(resolution);
CREATE INDEX IF NOT EXISTS idx_prediction_expiry ON prediction(expiry);

-- REALITY LEVERAGE — the OBJECTIVE-FUNCTION FLIP. Once the Reality Engine exists, "build capability ->
-- gain progress" is the wrong objective; "create L4 evidence -> gain progress" is right. reality_milestone
-- is the L4 DEPENDENCY GRAPH: the ordered critical path to the FIRST real-world L4 event (real user ->
-- auth -> tenant -> real action -> money gate -> approval -> audit). The question stops being "what
-- unlocks the workflow engine?" and becomes "what unlocks the first L4?". Reality Leverage =
-- expected_new_L4 / effort re-ranks the whole roadmap so the minimal path to reality beats the biggest
-- internal build. Durable SOURCE = committed reality-path.json.
CREATE TABLE IF NOT EXISTS reality_milestone (
  id               TEXT PRIMARY KEY,
  ordinal          INTEGER,                     -- position on the critical path to the first L4
  label            TEXT NOT NULL,
  description      TEXT,
  status           TEXT DEFAULT 'todo',         -- done | in_progress | todo | blocked
  effort           INTEGER,                     -- 1-5 remaining effort to complete it
  produces_l4      INTEGER DEFAULT 0,           -- does completing it CREATE an L4 reality event?
  delivered_by     TEXT,                        -- opportunity id / capability / 'existing' that delivers it
  blocked_by       TEXT,                        -- milestone id / external blocker
  would_create_event TEXT,                      -- the reality_event this milestone would record when done
  notes            TEXT
);
CREATE INDEX IF NOT EXISTS idx_reality_milestone_status ON reality_milestone(status);
CREATE INDEX IF NOT EXISTS idx_reality_milestone_ord ON reality_milestone(ordinal);
`)

// Idempotent additive-column migrations for tables that pre-date a column (CREATE TABLE IF NOT
// EXISTS will not add columns to an already-existing table). Each is a no-op once applied.
function addColumnIfMissing(table, column, decl) {
  const cols = db.prepare(`SELECT name FROM pragma_table_info('${table}')`).all().map(r => r.name)
  if (!cols.includes(column)) db.exec(`ALTER TABLE ${table} ADD COLUMN ${column} ${decl}`)
}
addColumnIfMissing('canonical_status_override', 'moves_money', 'INTEGER')
addColumnIfMissing('opportunity', 'expected_l4', 'INTEGER DEFAULT 0')  // Reality Leverage: expected NEW L4 evidence this opportunity creates (set by reconcile-reality-path)

// ── guard: never let canonical_capability sit silently empty when this SAME db file clearly
// carries real prior census work in its survivable side tables. The `CREATE TABLE IF NOT EXISTS`
// above is a no-op the moment ANY canonical_capability table exists — including an empty shell —
// and every other tool in this family (canonical-fallback.mjs, query.mjs, verify-docs.mjs,
// verify-impl-evidence.mjs, record-verified.mjs) treats "table exists" as "trust its row count".
// An empty table therefore silently DISABLES the docs/capabilities-cloud/cap-core.db fallback and
// reports zero canonical capabilities as if that were real — exactly the failure mode issue #713
// describes (source_capability/side tables survive, canonical_capability does not). Refuse to
// leave that state unannounced; point the operator at the dedicated recovery command instead.
const canonicalRows = db.prepare('SELECT count(*) n FROM canonical_capability').get().n
if (canonicalRows === 0 && !process.argv.includes('--allow-empty-canonical')) {
  const overrideRows = db.prepare('SELECT count(*) n FROM canonical_status_override').get().n
  const evidenceRows = db.prepare('SELECT count(*) n FROM impl_evidence').get().n
  const bridgeRows = db.prepare('SELECT count(*) n FROM slice_canonical').get().n
  if (overrideRows > 0 || evidenceRows > 0 || bridgeRows > 0) {
    console.error(JSON.stringify({
      error: 'CANONICAL_CAPABILITY_EMPTY_WITH_SURVIVING_SIDE_TABLES',
      message: 'canonical_capability has 0 rows in this db file, but canonical_status_override/' +
        'impl_evidence/slice_canonical carry real prior work. Leaving canonical_capability empty ' +
        'here would silently disable the cloud/side-table fallback used elsewhere in this tool ' +
        'family and report zero canonical capabilities as if that were true. Run ' +
        '`node tools/capabilities/recover-canonical.mjs` (dry-run, then --apply) to restore ' +
        'canonical rows from docs/capabilities-cloud/cap-core.db + docs/architecture.db before ' +
        're-running caps:schema, or pass --allow-empty-canonical if this really is a fresh, ' +
        'intentional bootstrap.',
      canonical_status_override_rows: overrideRows,
      impl_evidence_rows: evidenceRows,
      slice_canonical_rows: bridgeRows,
    }, null, 2))
    db.close()
    process.exit(1)
  }
}

const counts = {
  source_capability: db.prepare('SELECT count(*) n FROM source_capability').get().n,
  canonical_capability: db.prepare('SELECT count(*) n FROM canonical_capability').get().n,
  census_coverage: db.prepare('SELECT count(*) n FROM census_coverage').get().n,
  donor_file_census: db.prepare('SELECT count(*) n FROM donor_file_census').get().n,
  source_file_capability_link: db.prepare('SELECT count(*) n FROM source_file_capability_link').get().n,
  slice_canonical: db.prepare('SELECT count(*) n FROM slice_canonical').get().n,
  canonical_status_override: db.prepare('SELECT count(*) n FROM canonical_status_override').get().n,
  impl_evidence: db.prepare('SELECT count(*) n FROM impl_evidence').get().n,
  donor_absorption: db.prepare('SELECT count(*) n FROM donor_absorption').get().n,
  package_retirement: db.prepare('SELECT count(*) n FROM package_retirement').get().n,
  execution_state: db.prepare('SELECT count(*) n FROM execution_state').get().n,
  agent_note: db.prepare('SELECT count(*) n FROM agent_note').get().n,
  tracking_issue: db.prepare('SELECT count(*) n FROM tracking_issue').get().n,
  opportunity: db.prepare('SELECT count(*) n FROM opportunity').get().n,
  strategy_edge: db.prepare('SELECT count(*) n FROM strategy_edge').get().n,
  end_state_pillar: db.prepare('SELECT count(*) n FROM end_state_pillar').get().n,
  assumption: db.prepare('SELECT count(*) n FROM assumption').get().n,
  architecture_decision: db.prepare('SELECT count(*) n FROM architecture_decision').get().n,
  value_node: db.prepare('SELECT count(*) n FROM value_node').get().n,
  value_edge: db.prepare('SELECT count(*) n FROM value_edge').get().n,
  competitor: db.prepare('SELECT count(*) n FROM competitor').get().n,
  competitor_feature: db.prepare('SELECT count(*) n FROM competitor_feature').get().n,
  arch_debt: db.prepare('SELECT count(*) n FROM arch_debt').get().n,
  evolution_score: db.prepare('SELECT count(*) n FROM evolution_score').get().n,
  node_evidence: db.prepare('SELECT count(*) n FROM node_evidence').get().n,
  external_signal: db.prepare('SELECT count(*) n FROM external_signal').get().n,
  reality_event: db.prepare('SELECT count(*) n FROM reality_event').get().n,
  outcome: db.prepare('SELECT count(*) n FROM outcome').get().n,
  prediction: db.prepare('SELECT count(*) n FROM prediction').get().n,
  reality_milestone: db.prepare('SELECT count(*) n FROM reality_milestone').get().n,
}
console.log('census schema ready:', JSON.stringify(counts))
db.close()
