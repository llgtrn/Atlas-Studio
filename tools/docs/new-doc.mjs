#!/usr/bin/env node

import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { basename, dirname, join } from 'node:path'
import {
  expectedGeneratedDocumentTarget,
  generatedDocumentKinds,
  isCanonicalArchitectureOwner,
  isValidGeneratedDocumentTarget,
} from './architecture-registry.mjs'

const [, , type, target] = process.argv
const allowed = new Set(generatedDocumentKinds)

if (!allowed.has(type) || !target) {
  console.error(`usage: node tools/docs/new-doc.mjs <${generatedDocumentKinds.join('|')}> <docs/...md>`)
  process.exit(2)
}

if (!target.startsWith('docs/') || !target.endsWith('.md')) {
  console.error('target must be a Markdown path under docs/')
  process.exit(2)
}

const reserved = new Set([
  'docs/INDEX.md',
  'docs/TEMPLATE.md',
  'docs/README.md',
  'docs/architecture/README.md',
  'docs/blueprints/README.md',
  'docs/decisions/README.md',
])

if (reserved.has(target) || basename(target) === 'README.md') {
  console.error('governance/router README files are deliberate hand-maintained entrypoints and are not generator targets')
  process.exit(2)
}

if (!isValidGeneratedDocumentTarget(type, target)) {
  console.error(`${type} target violates documentation taxonomy; expected ${expectedGeneratedDocumentTarget(type)}`)
  process.exit(2)
}

if (existsSync(target)) {
  console.error(`refusing to overwrite existing file: ${target}`)
  process.exit(1)
}

const templatePath = join('tools', 'docs', 'templates', `${type}.md`)
const template = readFileSync(templatePath, 'utf8')
const slug = basename(target).replace(/\.md$/i, '').replace(/^\d{4}-/, '')
const title = slug.split('-').map((part) => part ? part[0].toUpperCase() + part.slice(1) : part).join(' ')
const content = template
  .replaceAll('{{TITLE}}', title)
  .replaceAll('{{SLUG}}', slug)
  .replaceAll('{{PATH}}', target)

mkdirSync(dirname(target), { recursive: true })
writeFileSync(target, content, 'utf8')
console.log(`created ${target} from ${templatePath}`)

if (type === 'architecture-owner' && !isCanonicalArchitectureOwner(target)) {
  console.log('governance: this new architecture owner is intentionally unregistered; add the exact path to canonicalArchitectureOwners in tools/docs/architecture-registry.mjs after architectural review')
}
console.log(`next: add one annotated \`${target.slice('docs/'.length)}\` entry to docs/INDEX.md, then run pnpm docs:check`)
