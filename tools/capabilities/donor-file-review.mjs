#!/usr/bin/env node
// donor-file-review.mjs — apply a reviewed donor file manifest to docs/capabilities.db.
// Reviewers inspect source files elsewhere; this command serializes the DB update.
import Database from 'better-sqlite3'
import { readFileSync } from 'node:fs'
import { CAPABILITIES_DB } from '../_paths.mjs'
import { applyReviewManifest, validateReviewManifest } from './donor-file-review-lib.mjs'

const manifestPath = argValue('--manifest')
const reviewer = argValue('--reviewed-by') || 'donor-file-review'
const checkOnly = process.argv.includes('--check')

function argValue(name) {
  const i = process.argv.indexOf(name)
  return i >= 0 ? process.argv[i + 1] : null
}

if (!manifestPath) {
  console.error('usage: node tools/capabilities/donor-file-review.mjs --manifest <review.json> [--check] [--reviewed-by name]')
  process.exit(2)
}

const manifest = JSON.parse(readFileSync(manifestPath, 'utf8'))
if (checkOnly) {
  const normalized = validateReviewManifest(manifest)
  console.log(JSON.stringify({
    donor: normalized.donor,
    file_reviews: normalized.file_reviews.length,
    links: normalized.links.length,
    new_source_capabilities_needed: normalized.new_source_capabilities_needed.length,
    blocked: normalized.blocked.length,
  }, null, 2))
  process.exit(0)
}

const db = new Database(CAPABILITIES_DB)
db.pragma('journal_mode = WAL')
const result = applyReviewManifest(db, manifest, { reviewedBy: reviewer })
db.close()
console.log(JSON.stringify(result, null, 2))
