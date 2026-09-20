import fs from 'node:fs'
import path from 'node:path'
import { execFileSync, spawnSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'

export const LOCAL_VERIFICATION_LEDGER = '.chronica/verification-evidence.json'
const MAX_RECORDS = 5000
const ALLOWED_STATUSES = new Set(['PASS', 'FAIL', 'SKIP', 'UNKNOWN'])

function git(root, args) {
  return execFileSync('git', args, { cwd: root, encoding: 'utf8' }).trim()
}

function gitState(root) {
  const sha = git(root, ['rev-parse', 'HEAD'])
  const statusText = execFileSync('git', ['status', '--porcelain=v1', '--untracked-files=all'], { cwd: root, encoding: 'utf8' })
  const lines = statusText.split('\n').filter(Boolean)
  return { sha, workingTreeDirty: lines.length > 0, workingTreeStatusCount: lines.length }
}

function now() {
  return new Date().toISOString()
}

function readLedger(root) {
  const ledgerPath = path.join(root, LOCAL_VERIFICATION_LEDGER)
  if (!fs.existsSync(ledgerPath)) return { schemaVersion: 1, source: 'local_execution', records: [] }
  const parsed = JSON.parse(fs.readFileSync(ledgerPath, 'utf8'))
  if (parsed.schemaVersion !== 1 || !Array.isArray(parsed.records)) {
    throw new Error(`Unsupported local verification ledger at ${LOCAL_VERIFICATION_LEDGER}`)
  }
  return parsed
}

function writeLedger(root, ledger) {
  const ledgerPath = path.join(root, LOCAL_VERIFICATION_LEDGER)
  fs.mkdirSync(path.dirname(ledgerPath), { recursive: true })
  const records = ledger.records.slice(-MAX_RECORDS)
  fs.writeFileSync(ledgerPath, `${JSON.stringify({ ...ledger, records }, null, 2)}\n`)
}

export function recordVerification(root, record) {
  if (!record.check || !record.command) throw new Error('Verification evidence requires check and command')
  const status = String(record.status ?? 'UNKNOWN').toUpperCase()
  if (!ALLOWED_STATUSES.has(status)) throw new Error(`Unsupported verification status: ${status}`)
  const ledger = readLedger(root)
  const state = record.gitState || gitState(root)
  const sha = record.sha || state.sha
  const executedAt = record.executedAt || now()
  const item = {
    id: record.id || `${sha}:${record.check}:${executedAt}`,
    sha,
    check: record.check,
    command: record.command,
    status,
    executor: record.executor || 'local',
    environment: record.environment || process.platform,
    executedAt,
    durationMs: record.durationMs ?? null,
    exitCode: record.exitCode ?? null,
    workingTreeDirty: record.workingTreeDirty ?? state.workingTreeDirty,
    workingTreeStatusCount: record.workingTreeStatusCount ?? state.workingTreeStatusCount,
    evidenceRefs: Array.isArray(record.evidenceRefs) ? record.evidenceRefs : [],
  }
  ledger.records.push(item)
  writeLedger(root, ledger)
  return item
}

export function runAndRecord(root, { check, command, args = [], executor = 'local' }) {
  const started = Date.now()
  const state = gitState(root)
  const result = spawnSync(command, args, { cwd: root, stdio: 'inherit', shell: false })
  const status = result.status === 0 ? 'PASS' : 'FAIL'
  const item = recordVerification(root, {
    check,
    command: [command, ...args].join(' '),
    status,
    executor,
    durationMs: Date.now() - started,
    exitCode: result.status,
    gitState: state,
  })
  return { item, result }
}

export function readLocalVerificationEvidence(root) {
  const ledger = readLedger(root)
  const currentState = gitState(root)
  const current = ledger.records.filter((record) => record.sha === currentState.sha)
  const latestByCheck = new Map()
  for (const record of current) {
    const prior = latestByCheck.get(record.check)
    if (!prior || String(record.executedAt).localeCompare(String(prior.executedAt)) > 0) latestByCheck.set(record.check, record)
  }
  const latestCurrentByCheck = [...latestByCheck.values()].sort((a, b) => a.check.localeCompare(b.check))
  const cleanCurrent = latestCurrentByCheck.filter((record) => record.workingTreeDirty === false)
  return {
    source: LOCAL_VERIFICATION_LEDGER,
    currentSha: currentState.sha,
    currentWorkingTreeDirty: currentState.workingTreeDirty,
    currentWorkingTreeStatusCount: currentState.workingTreeStatusCount,
    records: ledger.records,
    currentRecords: current,
    latestCurrentByCheck,
    cleanCurrentByCheck: cleanCurrent,
    stats: {
      totalRecords: ledger.records.length,
      currentShaRecords: current.length,
      currentChecks: latestCurrentByCheck.length,
      currentPass: latestCurrentByCheck.filter((record) => record.status === 'PASS').length,
      currentFail: latestCurrentByCheck.filter((record) => record.status === 'FAIL').length,
      currentSkip: latestCurrentByCheck.filter((record) => record.status === 'SKIP').length,
      cleanCurrentChecks: cleanCurrent.length,
      dirtyCurrentChecks: latestCurrentByCheck.length - cleanCurrent.length,
      staleRecords: ledger.records.length - current.length,
    },
  }
}

function parseArgs(argv) {
  const out = { _: [] }
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i]
    if (!arg.startsWith('--')) out._.push(arg)
    else {
      const key = arg.slice(2)
      const value = argv[i + 1] && !argv[i + 1].startsWith('--') ? argv[++i] : true
      out[key] = value
    }
  }
  return out
}

const invokedDirectly = process.argv[1] && fileURLToPath(import.meta.url) === path.resolve(process.argv[1])
if (invokedDirectly) {
  const root = process.cwd()
  const [mode] = process.argv.slice(2)
  const args = parseArgs(process.argv.slice(3))
  if (mode === 'record') {
    if (!args.check || !args.command || !args.status) throw new Error('record requires --check --command --status')
    const item = recordVerification(root, {
      check: String(args.check),
      command: String(args.command),
      status: String(args.status).toUpperCase(),
      executor: args.executor ? String(args.executor) : 'local',
      durationMs: args.duration ? Number(args.duration) : null,
      exitCode: args.exit ? Number(args.exit) : null,
    })
    console.log(JSON.stringify(item, null, 2))
  } else if (mode === 'run') {
    if (!args.check || args._.length === 0) throw new Error('run requires --check followed by a command')
    const [command, ...commandArgs] = args._
    const { result } = runAndRecord(root, { check: String(args.check), command, args: commandArgs })
    process.exitCode = result.status ?? 1
  } else if (mode === 'show') {
    console.log(JSON.stringify(readLocalVerificationEvidence(root), null, 2))
  } else {
    console.error('Usage: local-verification.mjs <run|record|show> ...')
    process.exitCode = 2
  }
}
