#!/usr/bin/env node

import { execFileSync } from 'node:child_process'
import { existsSync, readFileSync } from 'node:fs'
import { basename, dirname, isAbsolute, join, normalize, resolve, sep } from 'node:path'
import {
  architectureResponsibilityDirs,
  canonicalArchitectureOwners,
  documentationRouterPaths,
  documentationTopLevelEntries,
  forbiddenDocumentationRoots,
  isCanonicalArchitectureOwner,
} from './architecture-registry.mjs'

const ROOT = process.cwd()
const errors = []

const approvedTopLevel = new Set(documentationTopLevelEntries)
const approvedArchitectureDirs = new Set(architectureResponsibilityDirs)
const routerAndConstitutionPaths = new Set(documentationRouterPaths)
const forbiddenRoots = forbiddenDocumentationRoots

const staleCanonicalPaths = new Map([
  ['docs/architecture/NORTH-STAR.md', 'docs/architecture/constitution/NORTH-STAR.md'],
  ['docs/architecture/ENGINEERING-CONTRACT.md', 'docs/architecture/constitution/ENGINEERING-CONTRACT.md'],
  ['docs/architecture/NORMATIVE-WORLD.md', 'docs/architecture/governance/normative.md'],
  ['docs/architecture/normative.md', 'docs/architecture/governance/normative.md'],
  ['docs/architecture/EXECUTION.md', 'docs/architecture/governance/execution.md'],
  ['docs/architecture/execution.md', 'docs/architecture/governance/execution.md'],
  ['docs/architecture/evidence.md', 'docs/architecture/governance/evidence.md'],
  ['docs/architecture/FABRIC.md', 'docs/architecture/operations/fabric.md'],
  ['docs/architecture/fabric.md', 'docs/architecture/operations/fabric.md'],
  ['docs/architecture/MEMORY-CONTEXT.md', 'docs/architecture/intelligence/memory.md + docs/architecture/intelligence/context.md'],
  ['docs/architecture/DIGITAL-ORGANISM-ENGINEERING.md', 'docs/architecture/organism/organism.md + docs/blueprints/digital-organism.md'],
  ['docs/decisions/only-current-non-obsolete-ADRs.md', 'docs/decisions/README.md'],
])

// Every canonical architecture owner also implicitly retires the historical flat
// docs/architecture/<basename> location. Deriving these aliases from the shared
// registry prevents the stale-path checker from lagging behind future owner moves.
for (const owner of canonicalArchitectureOwners) {
  const rel = owner.slice('docs/architecture/'.length)
  const parts = rel.split('/')
  if (parts.length !== 2) continue
  const flat = `docs/architecture/${parts[1]}`
  if (flat !== owner && !staleCanonicalPaths.has(flat)) staleCanonicalPaths.set(flat, owner)
}

function slash(value) { return String(value).split(sep).join('/') }
function trackedFiles(prefix = 'docs') {
  return execFileSync('git', ['ls-files', prefix], { cwd: ROOT, encoding: 'utf8' })
    .split('\n').map((line) => slash(line.trim())).filter(Boolean)
}
function requireFile(path) {
  if (!existsSync(join(ROOT, path))) errors.push(`missing required documentation entry: ${path}`)
}
function read(path) { return readFileSync(join(ROOT, path), 'utf8') }

const paths = trackedFiles('docs')
const markdownPaths = paths.filter((p) => p.endsWith('.md'))
const markdownPathSet = new Set(markdownPaths)
const prosePaths = ['AGENTS.md', 'README.md', ...markdownPaths].filter((p) => existsSync(join(ROOT, p)))
const ciReferencePaths = [
  ...trackedFiles('.github/workflows'),
  ...trackedFiles('.github/actions'),
].filter((p) => /\.ya?ml$/i.test(p) && existsSync(join(ROOT, p)))
const referenceCarrierPaths = [...new Set([...prosePaths, ...ciReferencePaths])]

for (const path of paths) {
  const rel = path.slice('docs/'.length)
  const top = rel.split('/')[0]
  if (!approvedTopLevel.has(top)) errors.push(`unapproved docs top-level entry: docs/${top}`)
  for (const forbidden of forbiddenRoots) {
    if (path === forbidden.slice(0, -1) || path.startsWith(forbidden)) errors.push(`forbidden documentation silo/path: ${path}`)
  }
}

const seenCase = new Map()
for (const path of paths) {
  const key = path.toLowerCase()
  const prior = seenCase.get(key)
  if (prior && prior !== path) errors.push(`case-colliding documentation paths: ${prior} <-> ${path}`)
  else seenCase.set(key, path)
}

// Architecture is closed-world: README is the router, and every other Markdown
// file must be an explicitly registered canonical owner. Merely placing a file in
// an approved responsibility directory does not make it architecture authority.
for (const path of markdownPaths.filter((p) => p.startsWith('docs/architecture/'))) {
  const rel = path.slice('docs/architecture/'.length)
  if (!rel.includes('/')) {
    if (rel !== 'README.md') errors.push(`architecture root is a router only; move/remove flat owner: ${path}`)
    continue
  }
  const dir = rel.split('/')[0]
  if (!approvedArchitectureDirs.has(dir)) {
    errors.push(`unapproved architecture responsibility directory: docs/architecture/${dir}/`)
    continue
  }
  if (!isCanonicalArchitectureOwner(path)) {
    errors.push(`unregistered architecture document: ${path}; architecture authority requires explicit canonicalArchitectureOwners registration`)
  }
}

for (const owner of canonicalArchitectureOwners) requireFile(owner)
for (const router of documentationRouterPaths) requireFile(router)

if (existsSync(join(ROOT, 'docs/INDEX.md'))) {
  const index = read('docs/INDEX.md')
  const indexLines = index.split('\n')

  for (const owner of canonicalArchitectureOwners) {
    const rel = owner.slice('docs/'.length)
    if (!index.includes(`\`${rel}\``)) errors.push(`docs/INDEX.md missing canonical owner: ${rel}`)
  }

  // INDEX is the maintained-doc registry. Every tracked Markdown document except
  // INDEX itself must have exactly one annotated registry entry. Appearances in task
  // tables or prose do not count; duplicate owner descriptions are also rejected.
  for (const path of markdownPaths) {
    if (path === 'docs/INDEX.md') continue
    const rel = path.slice('docs/'.length)
    const needle = `\`${rel}\``
    const annotatedLines = indexLines.filter((line) => {
      const normalizedLine = line.trim().replace(/^[-*]\s+/, '')
      if (!normalizedLine.startsWith(needle)) return false
      const after = normalizedLine.slice(needle.length).trim()
      return after.length >= 12
    })
    if (annotatedLines.length === 0) errors.push(`docs/INDEX.md missing annotated maintained-document entry: ${rel}`)
    if (annotatedLines.length > 1) errors.push(`docs/INDEX.md has duplicate annotated maintained-document entries: ${rel}`)
  }

  // Fail on stale registry bullets as well. An INDEX entry must not outlive the
  // maintained file it claims to register. This parser intentionally recognizes
  // only bullet entries whose first token is a Markdown path.
  const registryBullet = /^\s*[-*]\s+`([^`]+\.md)`\s+(.+)$/
  for (const line of indexLines) {
    const match = line.match(registryBullet)
    if (!match) continue
    const rel = match[1]
    if (rel === 'INDEX.md') continue
    const full = rel.startsWith('docs/') ? rel : `docs/${rel}`
    if (!markdownPathSet.has(full)) errors.push(`docs/INDEX.md registers missing maintained document: ${rel}`)
  }
}

// Reject known moved canonical paths anywhere humans/agents/CI are likely to read them.
// CI YAML participates only as a reference carrier; it is not treated as Markdown.
for (const path of referenceCarrierPaths) {
  const content = read(path)
  for (const [oldPath, replacement] of staleCanonicalPaths) {
    if (content.includes(oldPath)) errors.push(`stale documentation path in ${path}: ${oldPath} -> ${replacement}`)
  }
}

// Validate explicit repository-root docs/*.md references in maintained prose and CI
// configuration. This catches workflow comments/run steps that still point at retired docs.
const rootDocRef = /docs\/[A-Za-z0-9_.\/-]+\.md/g
for (const path of referenceCarrierPaths) {
  const content = read(path)
  for (const match of content.matchAll(rootDocRef)) {
    const target = normalize(match[0])
    if (!existsSync(join(ROOT, target))) errors.push(`broken documentation reference in ${path}: ${match[0]}`)
  }
}

// Validate ordinary local Markdown links. External URLs, mailto and anchors are ignored.
const markdownLink = /\]\(([^)]+)\)/g
for (const path of markdownPaths) {
  const content = read(path)
  for (const match of content.matchAll(markdownLink)) {
    let target = match[1].trim().replace(/^<|>$/g, '')
    if (!target || target.startsWith('#') || /^(https?:|mailto:|tel:)/i.test(target)) continue
    target = target.split('#')[0].split('?')[0]
    if (!target) continue

    const absolute = target.startsWith('docs/')
      ? join(ROOT, target)
      : isAbsolute(target)
        ? join(ROOT, target.replace(/^\/+/, ''))
        : resolve(ROOT, dirname(path), target)

    if (!existsSync(absolute)) errors.push(`broken Markdown link in ${path}: ${match[1]}`)
  }
}

// Validate Markdown paths written as inline code. Explicit relative paths are checked
// everywhere. Bare basenames are checked strictly only in routers/constitution, where
// a moved owner reference is especially dangerous; other prose may use *.md as an
// illustrative filename and should not be guessed into a link by CI.
const inlineCodeDocRef = /`([^`\n]+\.md)`/g
const byBasename = new Map()
for (const doc of markdownPaths) {
  const name = basename(doc)
  const list = byBasename.get(name) ?? []
  list.push(doc)
  byBasename.set(name, list)
}

for (const path of markdownPaths) {
  const content = read(path)
  for (const match of content.matchAll(inlineCodeDocRef)) {
    const raw = match[1].trim()
    if (!raw || raw.startsWith('http://') || raw.startsWith('https://')) continue
    if (raw.startsWith('docs/')) continue // already checked above

    if (raw.startsWith('./') || raw.startsWith('../') || raw.includes('/')) {
      const absolute = resolve(ROOT, dirname(path), raw)
      if (!existsSync(absolute)) errors.push(`broken relative inline documentation reference in ${path}: ${raw}`)
      continue
    }

    const local = resolve(ROOT, dirname(path), raw)
    if (existsSync(local)) continue
    if (!routerAndConstitutionPaths.has(path)) continue

    const matches = byBasename.get(raw) ?? []
    if (matches.length === 1) {
      errors.push(`ambiguous/moved bare documentation reference in ${path}: ${raw}; use ${matches[0]}`)
    }
  }
}

// Guard a small set of constitutional semantic boundaries. These are meaning checks,
// not prose-shape checks: they prevent reintroducing vocabulary that previously caused
// architecture drift while leaving authors free to structure documents naturally.
const semanticBoundaryPaths = [
  'README.md',
  'AGENTS.md',
  'docs/INDEX.md',
  'docs/TEMPLATE.md',
  'docs/architecture/README.md',
  'docs/architecture/constitution/NORTH-STAR.md',
  'docs/architecture/foundation/system-model.md',
  'docs/architecture/governance/execution.md',
]

const forbiddenSemanticPatterns = [
  { pattern: /\bAdmit\(\.\.\.\)/, label: 'use ExecutionAdmission(...) instead of Admit(...) shorthand' },
  { pattern: /\bCanonical Admission\b/i, label: 'use Canonicalization; canonical admission is ambiguous' },
]

for (const path of semanticBoundaryPaths) {
  if (!existsSync(join(ROOT, path))) {
    errors.push(`missing semantic-boundary document: ${path}`)
    continue
  }
  const content = read(path)
  for (const { pattern, label } of forbiddenSemanticPatterns) {
    if (pattern.test(content)) errors.push(`semantic vocabulary regression in ${path}: ${label}`)
  }
}

const requiredSemanticMarkers = new Map([
  ['README.md', ['ExecutionAdmission', 'Canonicalization', 'Selected Binding']],
  ['AGENTS.md', ['ExecutionAdmission != Canonicalization', 'Can(...) != Authorized(...) != May(...) != Must(...)']],
  ['docs/TEMPLATE.md', ['ExecutionAdmission', 'Canonicalization']],
  ['docs/architecture/README.md', ['ExecutionAdmission != Canonicalization', 'Can != Authorized != May != Must']],
  ['docs/architecture/constitution/NORTH-STAR.md', ['ExecutionAdmission', 'Canonicalization', 'Selected Binding']],
  ['docs/architecture/foundation/system-model.md', ['ExecutionAdmission', 'Canonicalization', 'Selected Binding']],
  ['docs/architecture/governance/execution.md', ['ExecutionAdmission != Canonicalization', 'candidate binding', 'selected binding']],
])

for (const [path, markers] of requiredSemanticMarkers) {
  if (!existsSync(join(ROOT, path))) continue
  const content = read(path)
  for (const marker of markers) {
    if (!content.toLowerCase().includes(marker.toLowerCase())) {
      errors.push(`missing canonical semantic marker in ${path}: ${marker}`)
    }
  }
}

// CI deliberately does NOT enforce prose length, heading count, Mermaid presence,
// equation count, frontmatter shape, or one universal section skeleton.
// Those rules made agents optimize documentation for the checker instead of humans.

if (errors.length) {
  console.error('Chronica documentation layout FAILED.')
  for (const error of [...new Set(errors)]) console.error(`\n- ${error}`)
  process.exit(1)
}

console.log('Chronica documentation layout OK.')
console.log(`Tracked docs: ${paths.length}`)
console.log(`Maintained Markdown docs: ${markdownPaths.length}`)
console.log(`Canonical architecture owners: ${canonicalArchitectureOwners.length}`)
console.log(`CI YAML reference carriers: ${ciReferencePaths.length}`)
