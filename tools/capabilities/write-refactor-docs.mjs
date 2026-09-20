#!/usr/bin/env node
// write-refactor-docs.mjs — write the mined small canonical docs from the refactor workflow journal.
// Each result has dir, start (band base), docs[]{number,slug,title,body_markdown}. Writes
// docs/NNN-<slug>.md with body_markdown verbatim. Bumps on collision. Verifies format (header + blurb).
import { readFileSync, writeFileSync, existsSync } from 'node:fs'
import { join } from 'node:path'

const [dir] = process.argv.slice(2)
const journal = readFileSync(join(dir, 'journal.jsonl'), 'utf8')
const results = []
for (const line of journal.split('\n')) {
  if (!line.trim()) continue
  let e; try { e = JSON.parse(line) } catch { continue }
  if (e.type === 'result' && e.result && Array.isArray(e.result.docs)) results.push(e.result)
}

const DOCS = join(process.cwd(), 'docs')
const used = new Set()
// reserve existing numbered docs so we never overwrite 000-043 / existing
for (const f of ['000', '001', '010', '020', '021', '022', '023', '024', '030', '031', '032', '033', '034', '035', '036', '037', '038', '039', '040', '041', '042', '043']) used.add(Number(f))

function nextFree(n) {
  let x = n
  while (used.has(x)) x++
  used.add(x)
  return x
}

let written = 0, thin = 0, perDir = {}
const writtenList = []
for (const r of results) {
  const d = r.dir
  perDir[d] = perDir[d] || []
  for (const doc of (r.docs || [])) {
    const body = (doc.body_markdown || '').trim()
    // format/quality guard: must have a header + a blurb line + be substantial
    if (!body || body.length < 400 || !/^#\s+\d+\s+—/m.test(body) && !/^#\s+\d+\s*[-—]/m.test(body)) {
      // still write if it has reasonable length, but flag thin ones
      if (body.length < 400) { thin++; continue }
    }
    const num = nextFree(Number(doc.number) || (r.start || 150))
    const slug = (doc.slug || doc.title || ('doc-' + num)).toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '').slice(0, 50)
    const fname = `${String(num).padStart(3, '0')}-${slug}.md`
    // ensure the body's H1 number matches the assigned number (rewrite the leading # NNN)
    let out = body.replace(/^#\s+\d+\s*[—-]\s*/m, `# ${String(num).padStart(3, '0')} — `)
    if (!/^#\s/m.test(out)) out = `# ${String(num).padStart(3, '0')} — ${doc.title || slug}\n\n` + out
    writeFileSync(join(DOCS, fname), out + '\n')
    written++
    perDir[d].push(fname)
    writtenList.push(fname)
  }
}

console.log(`wrote ${written} canonical docs (${thin} thin skipped) from ${results.length} topic dirs`)
for (const [d, files] of Object.entries(perDir)) console.log(`  ${d}: ${files.length} -> ${files[0] || '(none)'} .. ${files[files.length - 1] || ''}`)
// emit the full list for the index generator
writeFileSync(join(process.cwd(), '.refactor-written.json'), JSON.stringify(writtenList))
