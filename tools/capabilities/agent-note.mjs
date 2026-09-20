#!/usr/bin/env node
// agent-note.mjs — the SHARED Claude<->Codex coordination channel.
//
// Two agents work this repo from opposite ends: Codex discovers donor capabilities (the file census),
// Claude implements + verifies them. They coordinate implicitly through capability rows, but some
// things need an EXPLICIT hand-off: "I corrected this census moves_money tag from donor proof",
// "this donor source is misleading", "please re-census X", "I just verified cap K". This is that
// channel — an append-only note log in capabilities.db (survives census rebuilds) mirrored to
// docs/_machine/agent-notes.md for human + git visibility.
//
// BOTH AGENTS: read open notes at the START of each iteration (`list`), and post a note whenever you
// have one of the four kinds below. Requests stay `open` until the other agent acks/resolves them;
// the other kinds are informational and auto-resolve.
//
// KINDS:
//   census_correction  — "I changed moves_money for <cap> from X to Y because the donor source shows …"
//                        (so the other agent stops re-emitting the wrong tag / learns the real behavior)
//   donor_warning      — "<donor/source_files> is MISLEADING: <why>" (e.g. an _auto=False VIEW that looks
//                        like it posts money but doesn't) so nobody repeats the misread
//   request            — an explicit ask: "please re-census <cap>", "source_files for <cap> are wrong",
//                        "I'm refactoring <file>, don't touch it" — stays open until acked/resolved
//   handoff            — lightweight progress: "verified <cap> in <file>", "discovered N caps in <domain>"
//
// USAGE:
//   node tools/capabilities/agent-note.mjs post --author <claude|codex> --kind <kind> \
//        --body "<message>" [--cap <capability_key>] [--donor <donor/source>] [--request]
//   node tools/capabilities/agent-note.mjs list [--author <a>] [--kind <k>] [--open] [--for <me>] [--all]
//   node tools/capabilities/agent-note.mjs ack    --id <n> --by <claude|codex> [--note "<reply>"]
//   node tools/capabilities/agent-note.mjs resolve --id <n> --by <claude|codex> [--note "<reply>"]
//   node tools/capabilities/agent-note.mjs render   # just regenerate docs/_machine/agent-notes.md
//
// Notes: `post --kind request` (or `--request`) keeps status 'open'; other kinds post as 'resolved'
// (informational). `--now <iso>` overrides the timestamp (else real wall clock — fine in a CLI).
import Database from 'better-sqlite3'
import { writeFileSync } from 'node:fs'
import { CAPABILITIES_DB, AGENT_NOTES_MD } from '../_paths.mjs'

const NOTES_MD = AGENT_NOTES_MD
const KINDS = new Set(['census_correction', 'donor_warning', 'request', 'handoff'])
const AGENTS = new Set(['claude', 'codex'])

function arg(flag) { const i = process.argv.indexOf(flag); return i >= 0 ? process.argv[i + 1] : undefined }
const has = (flag) => process.argv.includes(flag)
const cmd = process.argv[2]
const nowIso = () => new Date().toISOString()

const db = new Database(CAPABILITIES_DB)
ensureSchema()

function ensureSchema() {
  db.exec(`CREATE TABLE IF NOT EXISTS agent_note (
    id INTEGER PRIMARY KEY AUTOINCREMENT, author TEXT NOT NULL, kind TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'open', capability_key TEXT, donor TEXT, body TEXT NOT NULL,
    created_at TEXT NOT NULL, resolved_by TEXT, resolved_at TEXT);
    CREATE INDEX IF NOT EXISTS idx_agent_note_status ON agent_note(status);
    CREATE INDEX IF NOT EXISTS idx_agent_note_kind ON agent_note(kind);`)
}

function render() {
  const rows = db.prepare('SELECT * FROM agent_note ORDER BY id DESC').all()
  const open = rows.filter(r => r.status !== 'resolved')
  const lines = []
  lines.push('# Agent Notes — shared Claude ↔ Codex coordination channel')
  lines.push('')
  lines.push('> Auto-generated mirror of the `agent_note` table in `docs/capabilities.db`. DO NOT hand-edit —')
  lines.push('> write via `node tools/capabilities/agent-note.mjs post …` (see AGENTS.md → Agent coordination).')
  lines.push('> Both agents read the **OPEN** section at the start of each iteration.')
  lines.push('')
  lines.push(`_${rows.length} notes total · ${open.length} open/unacked · generated ${rows.length ? rows[0].created_at.slice(0, 10) : 'n/a'}_`)
  lines.push('')
  const fmt = (r) => {
    const head = `- **#${r.id}** \`${r.kind}\` · by **${r.author}**` +
      (r.capability_key ? ` · cap \`${r.capability_key}\`` : '') +
      (r.donor ? ` · donor \`${r.donor}\`` : '') +
      ` · ${r.created_at.slice(0, 19).replace('T', ' ')}` +
      (r.status === 'open' ? '  ⏳ OPEN' : r.status === 'acked' ? '  👍 acked' : '  ✅ resolved') +
      (r.resolved_by ? ` by ${r.resolved_by}` : '')
    const body = String(r.body)
      .split('\n')
      .map(line => (line ? `  ${line}` : ''))
      .join('\n')
    return head + `\n${body}`
  }
  lines.push('## Open / unresolved')
  lines.push(open.length ? open.map(fmt).join('\n') : '_(none — all clear)_')
  lines.push('')
  lines.push('## Resolved (recent 40)')
  const resolved = rows.filter(r => r.status === 'resolved').slice(0, 40)
  lines.push(resolved.length ? resolved.map(fmt).join('\n') : '_(none yet)_')
  lines.push('')
  writeFileSync(NOTES_MD, lines.join('\n'))
}

if (cmd === 'post') {
  const author = arg('--author'), kind = arg('--kind'), body = arg('--body')
  if (!AGENTS.has(author)) { console.error('--author must be claude | codex'); process.exit(2) }
  if (!KINDS.has(kind)) { console.error('--kind must be one of: ' + [...KINDS].join(', ')); process.exit(2) }
  if (!body) { console.error('--body is required'); process.exit(2) }
  // requests stay OPEN (need the other agent to act); other kinds are informational -> resolved.
  const status = (kind === 'request' || has('--request')) ? 'open' : 'resolved'
  const info = db.prepare(`INSERT INTO agent_note (author,kind,status,capability_key,donor,body,created_at)
    VALUES (?,?,?,?,?,?,?)`).run(author, kind, status, arg('--cap') || null, arg('--donor') || null, body, arg('--now') || nowIso())
  render()
  console.log(`posted note #${info.lastInsertRowid} [${kind}, ${status}] by ${author}`)
} else if (cmd === 'list') {
  let rows = db.prepare('SELECT * FROM agent_note ORDER BY id DESC').all()
  if (arg('--author')) rows = rows.filter(r => r.author === arg('--author'))
  if (arg('--kind')) rows = rows.filter(r => r.kind === arg('--kind'))
  if (has('--open')) rows = rows.filter(r => r.status !== 'resolved')
  // --for <me>: notes the OTHER agent posted that are still open (things I should act on).
  if (arg('--for')) { const me = arg('--for'); rows = rows.filter(r => r.status !== 'resolved' && r.author !== me) }
  if (!has('--all') && !arg('--for') && !has('--open')) rows = rows.slice(0, 25)
  if (!rows.length) { console.log('(no matching notes)'); }
  for (const r of rows) {
    console.log(`#${r.id} [${r.kind}/${r.status}] by ${r.author}` +
      (r.capability_key ? ` cap=${r.capability_key}` : '') + (r.donor ? ` donor=${r.donor}` : '') +
      ` @${r.created_at.slice(0, 19)}` + (r.resolved_by ? ` (by ${r.resolved_by})` : ''))
    console.log('   ' + r.body.replace(/\n/g, '\n   '))
  }
} else if (cmd === 'ack' || cmd === 'resolve') {
  const id = arg('--id'), by = arg('--by')
  if (!id || !AGENTS.has(by)) { console.error('usage: ' + cmd + ' --id <n> --by <claude|codex> [--note "…"]'); process.exit(2) }
  const status = cmd === 'ack' ? 'acked' : 'resolved'
  const reply = arg('--note')
  const row = db.prepare('SELECT * FROM agent_note WHERE id=?').get(id)
  if (!row) { console.error('no note #' + id); process.exit(1) }
  const newBody = reply ? `${row.body}\n  ↳ ${by}: ${reply}` : row.body
  db.prepare('UPDATE agent_note SET status=?, resolved_by=?, resolved_at=?, body=? WHERE id=?')
    .run(status, by, arg('--now') || nowIso(), newBody, id)
  render()
  console.log(`note #${id} -> ${status} by ${by}`)
} else if (cmd === 'render') {
  render()
  console.log('rendered docs/_machine/agent-notes.md')
} else {
  console.log('usage: agent-note.mjs <post|list|ack|resolve|render> … (see file header)')
  process.exit(cmd ? 2 : 0)
}
db.close()
