#!/usr/bin/env node
// verify-pinned-actions.mjs — supply-chain gate for every remote `uses:` in
// GitHub workflows and local composite actions. Remote actions must be pinned to
// a full 40-character commit SHA. Local refs (`./`, `../`) and docker:// refs are
// intentionally exempt.

import { existsSync, readdirSync, readFileSync, statSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const SCRIPT_DIR = dirname(fileURLToPath(import.meta.url))
const REPO_ROOT = join(SCRIPT_DIR, '..', '..')
const GITHUB_DIR = join(REPO_ROOT, '.github')

const PINNED_RE = /^[\w.-]+(?:\/[\w.-]+)+@[0-9a-f]{40}$/
const FLOATING_IDENTIFIERS = new Set(['stable', 'beta', 'nightly', 'edge', 'latest', 'master', 'main', 'dev', 'trunk', 'head'])
const USES_LINE_RE = /^(?:-\s*)?uses:\s*(['"]?)([^\s'"#]+)\1/

export function checkPinnedActions(content) {
  const lines = content.split(/\r?\n/)
  const errors = []

  lines.forEach((line, i) => {
    const trimmed = line.trimStart()
    if (trimmed.startsWith('#')) return
    const match = trimmed.match(USES_LINE_RE)
    if (!match) return
    const ref = match[2]
    const lineNo = i + 1

    if (ref.startsWith('./') || ref.startsWith('../') || ref.startsWith('docker://')) return
    if (PINNED_RE.test(ref)) return

    const atIndex = ref.lastIndexOf('@')
    const refPart = atIndex === -1 ? '' : ref.slice(atIndex + 1)

    if (atIndex === -1) {
      errors.push(`line ${lineNo}: \`uses: ${ref}\` has no @ref -- pin remote actions to a full 40-character commit SHA`)
      return
    }
    if (FLOATING_IDENTIFIERS.has(refPart.toLowerCase())) {
      errors.push(`line ${lineNo}: \`uses: ${ref}\` uses floating ref @${refPart} -- pin to a full commit SHA`)
      return
    }
    if (/^[0-9a-f]{7,39}$/i.test(refPart)) {
      errors.push(`line ${lineNo}: \`uses: ${ref}\` uses a short SHA -- pin all 40 characters`)
      return
    }
    if (/^v?\d+(\.\d+){0,2}(-[\w.]+)?$/i.test(refPart)) {
      errors.push(`line ${lineNo}: \`uses: ${ref}\` uses a mutable tag -- pin the tag's full commit SHA`)
      return
    }
    errors.push(`line ${lineNo}: \`uses: ${ref}\` is not pinned to a full 40-character commit SHA`)
  })

  return { ok: errors.length === 0, errors }
}

function yamlFilesRecursively(dir) {
  if (!existsSync(dir)) return []
  const out = []
  for (const name of readdirSync(dir)) {
    const path = join(dir, name)
    const stat = statSync(path)
    if (stat.isDirectory()) out.push(...yamlFilesRecursively(path))
    else if (name.endsWith('.yml') || name.endsWith('.yaml')) out.push(path)
  }
  return out
}

function defaultActionFiles() {
  return [
    ...yamlFilesRecursively(join(GITHUB_DIR, 'workflows')),
    ...yamlFilesRecursively(join(GITHUB_DIR, 'actions')),
  ].sort()
}

function main() {
  const argFiles = process.argv.slice(2)
  const files = argFiles.length > 0 ? argFiles : defaultActionFiles()

  if (files.length === 0) {
    console.error('verify-pinned-actions: no workflow/composite-action YAML files found')
    process.exit(1)
  }

  let anyFailed = false
  for (const file of files) {
    const content = readFileSync(file, 'utf8')
    const { ok, errors } = checkPinnedActions(content)
    if (ok) {
      console.log(`OK    ${file}`)
    } else {
      anyFailed = true
      for (const error of errors) console.error(`FAIL  ${file}: ${error}`)
    }
  }

  process.exit(anyFailed ? 1 : 0)
}

if (import.meta.url === `file://${process.argv[1]}`) main()
