import { existsSync, readFileSync, readdirSync } from 'node:fs'
import { join, relative, sep } from 'node:path'
import {
  BROWSER_COMPUTER_DATA_FLOWS,
  BROWSER_COMPUTER_EDGES,
  BROWSER_COMPUTER_INVARIANTS,
  BROWSER_COMPUTER_NODES,
} from './browser-computer-model.mjs'
import { discoverReachableModules } from './cfg-test-module-classifier.mjs'

const slash = (p) => p.split(sep).join('/')

function readText(path) {
  return readFileSync(path, 'utf8')
}

function sqlString(value) {
  return value == null ? null : String(value)
}

function parsePackageName(toml) {
  const m = toml.match(/^\s*name\s*=\s*"([^"]+)"/m)
  return m?.[1] ?? null
}

function parseWorkspaceMembers(toml) {
  const m = toml.match(/members\s*=\s*\[([\s\S]*?)\]/m)
  if (!m) return []
  return [...m[1].matchAll(/"([^"]+)"/g)].map((x) => x[1])
}

function parseChronicaDependencies(toml) {
  const deps = new Set()
  for (const m of toml.matchAll(/^\s*(chronica-[a-z0-9-]+)\s*=/gm)) {
    deps.add(m[1])
  }
  return [...deps].sort()
}

export function listRustFiles(dir) {
  if (!existsSync(dir)) return []
  const out = []
  const walk = (p) => {
    for (const ent of readdirSync(p, { withFileTypes: true })) {
      const full = join(p, ent.name)
      if (ent.isDirectory()) {
        if (['target', '.git', 'node_modules'].includes(ent.name)) continue
        walk(full)
      } else if (ent.isFile() && ent.name.endsWith('.rs')) {
        out.push(full)
      }
    }
  }
  walk(dir)
  return out
}

export function countPattern(files, pattern) {
  let count = 0
  const hits = []
  for (const file of files) {
    const raw = readText(file)
    if (pattern.test(raw)) {
      count += 1
      hits.push(file)
    }
  }
  return { count, hits }
}

function makeNode(id, kind, name, fields = {}) {
  return {
    id,
    kind,
    name,
    crate: fields.crate ?? null,
    modulePath: fields.modulePath ?? null,
    filePath: fields.filePath ?? null,
    status: fields.status ?? 'generated',
    evidenceRef: fields.evidenceRef ?? fields.filePath ?? null,
  }
}

function makeEdge(fromNode, toNode, kind, evidenceRef = null) {
  return { fromNode, toNode, kind, evidenceRef }
}

const HOLDING_RUNTIME_NODES = [
  makeNode('runtime_entity:Group', 'runtime_entity', 'Group', {
    crate: 'chronica-core',
    evidenceRef: 'crates/chronica-core',
  }),
  makeNode('runtime_entity:CouncilPresident', 'authority_role', 'CouncilPresident', {
    crate: 'chronica-company',
    evidenceRef: 'crates/chronica-company',
  }),
  makeNode('runtime_entity:Council', 'authority_role', 'Council', {
    crate: 'chronica-company',
    evidenceRef: 'crates/chronica-company',
  }),
  makeNode('runtime_entity:Company', 'runtime_entity', 'Company', {
    crate: 'chronica-core',
    evidenceRef: 'crates/chronica-core',
  }),
  makeNode('runtime_entity:CompanyCeo', 'authority_role', 'CompanyCeo', {
    crate: 'chronica-company',
    evidenceRef: 'docs/design/profit-os-architecture.md',
  }),
  makeNode('runtime_entity:Project', 'runtime_entity', 'Project', {
    crate: 'chronica-core',
    evidenceRef: 'crates/chronica-core',
  }),
  makeNode('runtime_entity:ProjectOwner', 'authority_role', 'ProjectOwner', {
    crate: 'chronica-company',
    evidenceRef: 'crates/chronica-company/src/authority.rs',
  }),
  makeNode('runtime_entity:Agent', 'runtime_entity', 'Agent', {
    crate: 'chronica-ai-workforce',
    evidenceRef: 'crates/chronica-ai-workforce',
  }),
  makeNode('runtime_entity:Tool', 'runtime_entity', 'Tool', {
    crate: 'chronica-runtime',
    evidenceRef: 'crates/chronica-runtime',
  }),
  makeNode('template:GroupTemplate', 'template', 'GroupTemplate', {
    crate: 'chronica-template',
    evidenceRef: 'crates/chronica-template',
  }),
  makeNode('template:CompanyTemplate', 'template', 'CompanyTemplate', {
    crate: 'chronica-template',
    evidenceRef: 'crates/chronica-template',
  }),
  makeNode('template:ProjectTemplate', 'template', 'ProjectTemplate', {
    crate: 'chronica-template',
    evidenceRef: 'crates/chronica-template',
  }),
  makeNode('gate:money', 'gate', 'One Money Gate', {
    crate: 'chronica-policy',
    evidenceRef: 'docs/010-invariant-one-money-gate.md',
  }),
  makeNode('audit:merkle', 'audit', 'Merkle Audit Chain', {
    crate: 'chronica-core',
    evidenceRef: 'crates/chronica-core',
  }),
  makeNode('scope:ScopePath', 'scope', 'ScopePath Isolation', {
    crate: 'chronica-core',
    evidenceRef: 'crates/chronica-core',
  }),
  makeNode('gate:approval', 'gate', 'Approval Governance Gate', {
    crate: 'chronica-approvals',
    evidenceRef: 'crates/chronica-approvals/src/request.rs',
  }),
]

const HOLDING_EDGES = [
  makeEdge('template:GroupTemplate', 'runtime_entity:Group', 'materializes', 'docs/design/profit-os-architecture.md'),
  makeEdge('template:CompanyTemplate', 'runtime_entity:Company', 'materializes', 'docs/design/profit-os-architecture.md'),
  makeEdge('template:ProjectTemplate', 'runtime_entity:Project', 'materializes', 'docs/design/profit-os-architecture.md'),
  makeEdge('runtime_entity:Group', 'runtime_entity:Company', 'owns', 'docs/design/profit-os-architecture.md'),
  makeEdge('runtime_entity:Company', 'runtime_entity:Project', 'owns', 'docs/design/profit-os-architecture.md'),
  makeEdge('runtime_entity:CompanyCeo', 'runtime_entity:Company', 'operates', 'docs/design/profit-os-architecture.md'),
  makeEdge('runtime_entity:ProjectOwner', 'runtime_entity:Project', 'operates', 'docs/design/holding-os-architecture.md'),
  makeEdge('runtime_entity:CouncilPresident', 'gate:money', 'approves', 'docs/design/profit-os-architecture.md'),
  makeEdge('runtime_entity:Council', 'gate:money', 'approves', 'docs/design/profit-os-architecture.md'),
  makeEdge('runtime_entity:Agent', 'runtime_entity:Tool', 'invokes', 'docs/design/profit-os-architecture.md'),
  makeEdge('runtime_entity:Tool', 'gate:money', 'routes_to', 'docs/010-invariant-one-money-gate.md'),
  makeEdge('gate:money', 'audit:merkle', 'writes', 'docs/010-invariant-one-money-gate.md'),
  makeEdge('runtime_entity:Agent', 'gate:approval', 'requests', 'docs/design/profit-os-architecture.md'),
  makeEdge('runtime_entity:Council', 'gate:approval', 'approves', 'docs/design/profit-os-architecture.md'),
  makeEdge('gate:approval', 'audit:merkle', 'writes', 'docs/design/profit-os-architecture.md'),
]

const INVARIANTS = [
  {
    key: 'one_money_gate',
    name: 'One money gate',
    description: 'Money-touching actions route through the canonical policy/approval gate; no parallel money gate.',
    enforcingNode: 'gate:money',
    verificationCommand: 'cargo test -p chronica-policy gate',
    status: 'implemented',
  },
  {
    key: 'one_merkle_audit',
    name: 'One Merkle audit chain',
    description: 'Consequential money/evidence paths use the single SHA-256 Merkle audit chain.',
    enforcingNode: 'audit:merkle',
    verificationCommand: 'cargo test -p chronica-core audit_chain',
    status: 'implemented',
  },
  {
    key: 'scopepath_isolation',
    name: 'ScopePath isolation',
    description: 'Company/project/group visibility is scoped; parent rollups use containment and peer raw data stays isolated.',
    enforcingNode: 'scope:ScopePath',
    verificationCommand: 'cargo test -p chronica-core scope',
    status: 'implemented',
  },
  {
    key: 'screening_tighten_only',
    name: 'Profit screening is tighten-only',
    description: 'Profit/Holding verdicts can block or escalate but cannot authorize money.',
    enforcingNode: 'crate:chronica-policy',
    verificationCommand: 'cargo test -p chronica-strategy screening_never_authorizes_money_for_any_outcome',
    status: 'implemented',
  },
  {
    key: 'cognition_tighten_only',
    name: 'Cognition is advisory only',
    description: 'Game-aware cognition can classify, block, downgrade, recommend, or escalate; it cannot authorize money, place trades, execute tools, or bypass approvals.',
    enforcingNode: 'module:chronica-strategy:cognition',
    verificationCommand: 'cargo test -p chronica-strategy',
    status: 'generated',
  },
  {
    key: 'council_money_authority',
    name: 'Council/President money authority',
    description: 'Company CEOs may propose; Council/President approval path owns group capital and irreversible decisions.',
    enforcingNode: 'runtime_entity:Council',
    verificationCommand: 'cargo test -p chronica-company --test authority',
    status: 'implemented',
  },
]

const AUTHORITY_RULES = [
  {
    actor: 'CouncilPresident',
    scope: 'Group',
    maySee: 'group rollups and governed evidence',
    mayPropose: 'constitution, strategy, irreversible actions',
    mayApprove: 'constitution, group capital, irreversible exits, rule changes',
    mayExecute: 'approval decisions only; execution remains gated runtime',
    mayAudit: 'all governed group/company rollups',
    forbiddenActions: 'silent autonomy; raw peer-data bypass; direct tool bypass',
    enforcingNode: 'runtime_entity:CouncilPresident',
  },
  {
    actor: 'Council',
    scope: 'Group',
    maySee: 'portfolio rollups via ScopePath containment',
    mayPropose: 'capital allocation, harvest, turnaround, exit',
    mayApprove: 'money, policy, irreversible actions',
    mayExecute: 'approval decisions only',
    mayAudit: 'portfolio and company evidence within governance',
    forbiddenActions: 'second money gate; unaudited rule edits',
    enforcingNode: 'runtime_entity:Council',
  },
  {
    actor: 'CompanyCeo',
    scope: 'Company',
    maySee: 'own company subtree',
    mayPropose: 'budgets, projects, operating actions',
    mayApprove: 'company-local non-money actions inside approved envelope',
    mayExecute: 'company operations after policy clears',
    mayAudit: 'own company evidence',
    forbiddenActions: 'group capital commit; irreversible exit; group rule change; peer raw data',
    enforcingNode: 'runtime_entity:CompanyCeo',
  },
  {
    actor: 'ProjectOwner',
    scope: 'Project',
    maySee: 'own project subtree only',
    mayPropose: 'project workflow, budget requests, task plans',
    mayApprove: 'project-local non-money workflow actions inside approved envelope',
    mayExecute: 'project operations through governed runtime paths',
    mayAudit: 'own project evidence',
    forbiddenActions: 'peer project raw data; peer company raw data; group capital commit; direct tool bypass',
    enforcingNode: 'runtime_entity:ProjectOwner',
  },
  {
    actor: 'Agent',
    scope: 'Assigned scope',
    maySee: 'granted scoped context',
    mayPropose: 'tool/workflow/project actions with evidence',
    mayApprove: 'never money; only bounded non-money automation if calibrated',
    mayExecute: 'through runtime invoke_tool only',
    mayAudit: 'own run evidence',
    forbiddenActions: 'self-approval; raw AI-to-money; direct adapter bypass; self-report-only success',
    enforcingNode: 'runtime_entity:Agent',
  },
]

const DATA_FLOWS = [
  {
    key: 'runtime_tool_invocation',
    sourceNode: 'runtime_entity:Agent',
    targetNode: 'runtime_entity:Tool',
    dataKind: 'ToolInvocation',
    scopeRule: 'must carry ScopePath',
    gateRequired: 1,
    auditRequired: 1,
  },
  {
    key: 'company_rollup_to_group',
    sourceNode: 'runtime_entity:Company',
    targetNode: 'runtime_entity:Group',
    dataKind: 'PortfolioRollup',
    scopeRule: 'parent reads child rollups via ScopePath containment only',
    gateRequired: 0,
    auditRequired: 1,
  },
  {
    key: 'template_to_runtime_entity',
    sourceNode: 'template:CompanyTemplate',
    targetNode: 'runtime_entity:Company',
    dataKind: 'TemplateApplication',
    scopeRule: 'template materializes entity but grants no authority bypass',
    gateRequired: 1,
    auditRequired: 1,
  },
]

export function discoverWorkspaceArchitecture(root = process.cwd()) {
  const workspaceTomlPath = join(root, 'Cargo.toml')
  const workspaceToml = readText(workspaceTomlPath)
  const members = parseWorkspaceMembers(workspaceToml)
  const nodes = []
  const edges = []

  for (const member of members) {
    const cargoPath = join(root, member, 'Cargo.toml')
    if (!existsSync(cargoPath)) continue
    const toml = readText(cargoPath)
    const name = parsePackageName(toml)
    if (!name) continue
    const relCargo = slash(relative(root, cargoPath))
    nodes.push(makeNode(`crate:${name}`, 'crate', name, {
      crate: name,
      filePath: relCargo,
      evidenceRef: relCargo,
    }))
    const srcDir = join(root, member, 'src')
    if (existsSync(srcDir)) {
      const rustFiles = listRustFiles(srcDir)
      const { production } = discoverReachableModules(rustFiles)
      const canonicalCounts = new Map()
      for (const [modulePath] of production) canonicalCounts.set(modulePath, (canonicalCounts.get(modulePath) ?? 0) + 1)
      for (const [modulePath, file] of production) {
        const rel = slash(relative(root, file))
        const canonicalId = `module:${name}:${modulePath || 'lib'}`
        const id = canonicalCounts.get(modulePath) > 1 ? `${canonicalId}@${rel}` : canonicalId
        nodes.push(makeNode(id, 'module', modulePath || 'lib', {
          crate: name,
          modulePath,
          filePath: rel,
          evidenceRef: rel,
        }))
        edges.push(makeEdge(`crate:${name}`, id, 'owns', rel))
      }
    }
    for (const dep of parseChronicaDependencies(toml)) {
      if (dep === name) continue
      edges.push(makeEdge(`crate:${name}`, `crate:${dep}`, 'depends_on', relCargo))
    }
  }

  nodes.push(...HOLDING_RUNTIME_NODES, ...BROWSER_COMPUTER_NODES)
  edges.push(...HOLDING_EDGES, ...BROWSER_COMPUTER_EDGES)

  return {
    nodes: dedupeRows(nodes, 'id'),
    edges,
    invariants: [...INVARIANTS, ...BROWSER_COMPUTER_INVARIANTS],
    authorityRules: AUTHORITY_RULES,
    dataFlows: [...DATA_FLOWS, ...BROWSER_COMPUTER_DATA_FLOWS],
  }
}

function dedupeRows(rows, key) {
  const out = new Map()
  for (const row of rows) if (!out.has(row[key])) out.set(row[key], row)
  return [...out.values()]
}
