export function createSchema(db) {
  db.exec(`
    PRAGMA journal_mode = WAL;
    CREATE TABLE meta (
      k TEXT PRIMARY KEY,
      v TEXT NOT NULL
    );
    CREATE TABLE architecture_node (
      id TEXT PRIMARY KEY,
      kind TEXT NOT NULL,
      name TEXT NOT NULL,
      crate TEXT,
      module_path TEXT,
      file_path TEXT,
      status TEXT NOT NULL,
      evidence_ref TEXT
    );
    CREATE TABLE architecture_edge (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      from_node TEXT NOT NULL,
      to_node TEXT NOT NULL,
      kind TEXT NOT NULL,
      evidence_ref TEXT
    );
    CREATE TABLE architecture_invariant (
      key TEXT PRIMARY KEY,
      name TEXT NOT NULL,
      description TEXT NOT NULL,
      enforcing_node TEXT NOT NULL,
      verification_command TEXT NOT NULL,
      status TEXT NOT NULL
    );
    CREATE TABLE authority_rule (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      actor TEXT NOT NULL,
      scope TEXT NOT NULL,
      may_see TEXT NOT NULL,
      may_propose TEXT NOT NULL,
      may_approve TEXT NOT NULL,
      may_execute TEXT NOT NULL,
      may_audit TEXT NOT NULL,
      forbidden_actions TEXT NOT NULL,
      enforcing_node TEXT NOT NULL
    );
    CREATE TABLE data_flow (
      key TEXT PRIMARY KEY,
      source_node TEXT NOT NULL,
      target_node TEXT NOT NULL,
      data_kind TEXT NOT NULL,
      scope_rule TEXT NOT NULL,
      gate_required INTEGER NOT NULL,
      audit_required INTEGER NOT NULL
    );
    CREATE TABLE capability_architecture_link (
      capability_key TEXT NOT NULL,
      architecture_node TEXT NOT NULL,
      relationship TEXT NOT NULL,
      status TEXT,
      target_crate TEXT,
      target_module TEXT,
      PRIMARY KEY (capability_key, architecture_node, relationship)
    );
    CREATE TABLE architecture_evidence (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      architecture_node TEXT NOT NULL,
      status TEXT NOT NULL,
      evidence_kind TEXT NOT NULL,
      evidence_ref TEXT NOT NULL,
      test_command TEXT,
      test_result TEXT,
      verified_at TEXT,
      verified_by TEXT,
      blocker_reason TEXT
    );
    CREATE TABLE architecture_status_override (
      architecture_node TEXT PRIMARY KEY,
      status TEXT NOT NULL,
      reason TEXT NOT NULL,
      updated_at TEXT NOT NULL,
      updated_by TEXT NOT NULL
    );
    CREATE TABLE architecture_target_gap (
      capability_key TEXT NOT NULL,
      target_crate TEXT,
      target_module TEXT,
      gap_kind TEXT NOT NULL,
      suggested_architecture_node TEXT,
      reason TEXT NOT NULL,
      status TEXT NOT NULL,
      evidence_ref TEXT,
      PRIMARY KEY (capability_key, gap_kind)
    );
    CREATE TABLE planned_architecture_node (
      id TEXT PRIMARY KEY,
      kind TEXT NOT NULL,
      name TEXT NOT NULL,
      logical_domain TEXT,
      intended_owner_crate TEXT,
      status TEXT NOT NULL,
      reason TEXT NOT NULL,
      evidence_ref TEXT
    );
    CREATE INDEX idx_arch_gap_kind ON architecture_target_gap(gap_kind);
    CREATE INDEX idx_arch_gap_target ON architecture_target_gap(target_crate);
    CREATE INDEX idx_arch_node_kind ON architecture_node(kind);
    CREATE INDEX idx_arch_edge_from ON architecture_edge(from_node);
    CREATE INDEX idx_arch_edge_to ON architecture_edge(to_node);
    CREATE INDEX idx_cap_arch_key ON capability_architecture_link(capability_key);
    CREATE INDEX idx_arch_evidence_node ON architecture_evidence(architecture_node);
    CREATE INDEX idx_arch_evidence_status ON architecture_evidence(status);
  `)
}

const GENERATED_TABLES = [
  'meta',
  'architecture_node',
  'architecture_edge',
  'architecture_invariant',
  'authority_rule',
  'data_flow',
  'capability_architecture_link',
  'architecture_evidence',
  'architecture_status_override',
  'architecture_target_gap',
  'planned_architecture_node',
]

export function resetGeneratedSchema(db) {
  db.exec('PRAGMA foreign_keys = OFF')
  for (const table of GENERATED_TABLES) {
    db.exec(`DROP TABLE IF EXISTS ${table}`)
  }
}