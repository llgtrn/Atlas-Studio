#!/usr/bin/env node
import { basename, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import {
  CAPABILITY_CANONICAL_DIR,
  CAPABILITY_STATUSES,
  loadCapabilityShards,
  parseArgs,
  rel,
  sanitizeShardName,
  validateArrayOfStrings,
  validateBoolean,
  validateSorted,
  validateString,
} from '../canonical-shards/jsonl-lib.mjs'

const KEY_PATTERN = /^[a-z0-9][a-z0-9._-]*\.[a-z0-9][a-z0-9._-]*$/i

export function verifyCanonicalCapabilityShards({ root = process.cwd() } = {}) {
  const errors = []
  const warnings = []
  const { meta, metaPath, files, entries } = loadCapabilityShards(root)

  if (!meta) errors.push(`${rel(root, metaPath)} is required`)
  if (files.length === 0) errors.push(`${CAPABILITY_CANONICAL_DIR}/domains/*.jsonl is required`)

  const seen = new Map()
  const domains = new Set()
  let verified = 0
  let money = 0
  let implementedUnverified = 0

  for (const file of files) {
    const fileEntries = entries.filter((entry) => entry.file === file)
    validateSorted(errors, fileEntries, (record) => String(record.capability_key ?? ''), 'capability_key')
  }

  for (const entry of entries) {
    const record = entry.record
    const path = `${rel(root, entry.file)}:${entry.line}`
    if (!record || typeof record !== 'object' || Array.isArray(record)) {
      errors.push(`${path}: record must be an object`)
      continue
    }
    if (record.schema_version !== 1) errors.push(`${path}.schema_version must be 1`)
    const key = validateString(errors, record, 'capability_key', path)
    if (key && !KEY_PATTERN.test(key)) errors.push(`${path}.capability_key is invalid: ${key}`)
    if (key) {
      const previous = seen.get(key)
      if (previous) errors.push(`${path}: duplicate capability_key ${key}; first seen at ${previous}`)
      else seen.set(key, path)
    }

    validateString(errors, record, 'canonical_name', path)
    const domain = validateString(errors, record, 'domain', path)
    validateString(errors, record, 'side_effect_class', path)
    validateString(errors, record, 'acceptance_criteria', path)
    validateString(errors, record, 'target_crate', path, { nullable: true })
    validateString(errors, record, 'target_module', path, { nullable: true })
    validateString(errors, record, 'financial_control_test', path, { nullable: true })
    validateString(errors, record, 'acceptance_test', path, { nullable: true })
    validateString(errors, record, 'blocker', path, { nullable: true })
    validateBoolean(errors, record, 'moves_money', path)
    validateBoolean(errors, record, 'requires_approval', path)

    for (const field of [
      'required_tests',
      'docs_refs',
      'code_refs',
      'test_refs',
      'architecture_refs',
      'source_refs',
      'evidence_refs',
    ]) {
      validateArrayOfStrings(errors, record, field, path)
    }

    const status = validateString(errors, record, 'status', path)
    if (status && !CAPABILITY_STATUSES.has(status)) errors.push(`${path}.status is invalid: ${status}`)
    if (status === 'verified') verified += 1
    if (status === 'implemented_unverified') implementedUnverified += 1
    if (record.moves_money === true) money += 1
    if ((status === 'verified' || status === 'implemented_unverified') && record.target_crate == null) {
      errors.push(`${path}.target_crate is required for ${status}`)
    }

    if (status === 'verified') {
      for (const field of ['code_refs', 'test_refs', 'docs_refs', 'architecture_refs']) {
        if (!Array.isArray(record[field]) || record[field].length === 0) {
          errors.push(`${path}.${field} is required for verified capability ${key}`)
        }
      }
    }

    // Regression guard for the 2026-08 capability-verification-promotion sync gap: a promotion
    // that only writes the top-level `status` field (never touching this same record's own
    // `architecture_refs` array, root docs/architecture-canonical/links.jsonl, or the crate-local
    // shard) leaves those 3 surfaces reporting a stale, pre-promotion status. This check catches
    // the cheapest surface to check here -- the record's own embedded architecture_refs -- at
    // verify time, so a future promotion cannot land without updating it. Only refs that target
    // this capability's own current target_crate are checked: a ref for a different crate is
    // either the separate "mis-pointed architecture_refs" defect (caught by no rule here; it needs
    // a human re-point decision) or a legitimate historical/gap entry, neither of which this status
    // check should flag.
    if ((status === 'verified' || status === 'implemented_unverified' || status === 'implemented') && record.target_crate) {
      for (const ref of record.architecture_refs ?? []) {
        if (typeof ref !== 'string') continue
        // Only "targets:"/"implemented_by:" refs end in a status value; "caller:", "gap:", and
        // other relationship kinds end in a symbol/reason and must not be treated as a status.
        if (!ref.startsWith('targets:') && !ref.startsWith('implemented_by:')) continue
        const targetsOwnCrate = ref.includes(`:crate:${record.target_crate}:`) || ref.includes(`:module:${record.target_crate}:`)
        if (!targetsOwnCrate) continue
        const refStatus = ref.slice(ref.lastIndexOf(':') + 1)
        if (refStatus !== status) {
          errors.push(`${path}.architecture_refs entry "${ref}" is stale: capability status is ${status} but this ref for its own target_crate (${record.target_crate}) still says ${refStatus} -- promotion did not propagate`)
        }
      }
    }
    if (record.moves_money === true && status === 'verified') {
      if (record.requires_approval !== true) errors.push(`${path}.requires_approval must be true for verified money capability`)
      if (!record.financial_control_test) errors.push(`${path}.financial_control_test is required for verified money capability`)
      const joinedRefs = [...(record.architecture_refs ?? []), ...(record.test_refs ?? []), ...(record.docs_refs ?? [])].join(' ')
      if (!/(gate:money|gate:approval|gated_by|governed_by|approval|financial)/i.test(joinedRefs)) {
        errors.push(`${path}: verified money capability requires money/approval gate and financial-control refs`)
      }
    }

    if (domain) {
      domains.add(domain)
      const expectedFile = `${sanitizeShardName(domain)}.jsonl`
      if (basename(entry.file) !== expectedFile) {
        errors.push(`${path}: domain ${domain} belongs in ${expectedFile}`)
      }
    }
  }

  if (meta) {
    if (meta.schema_version !== 1) errors.push(`${rel(root, metaPath)}.schema_version must be 1`)
    if (meta.authority !== 'canonical_jsonl') errors.push(`${rel(root, metaPath)}.authority must be canonical_jsonl`)
    if (Number(meta.record_count) !== entries.length) {
      errors.push(`${rel(root, metaPath)}.record_count=${meta.record_count} but JSONL has ${entries.length}`)
    }
    if (Number(meta.domain_count) !== domains.size) {
      errors.push(`${rel(root, metaPath)}.domain_count=${meta.domain_count} but JSONL has ${domains.size}`)
    }
    if (Number(meta.verified_count) !== verified) {
      errors.push(`${rel(root, metaPath)}.verified_count=${meta.verified_count} but JSONL has ${verified}`)
    }
    if (Number(meta.implemented_unverified_count) !== implementedUnverified) {
      errors.push(`${rel(root, metaPath)}.implemented_unverified_count=${meta.implemented_unverified_count} but JSONL has ${implementedUnverified}`)
    }
    if (Number(meta.money_count) !== money) {
      errors.push(`${rel(root, metaPath)}.money_count=${meta.money_count} but JSONL has ${money}`)
    }
    const listed = new Set((meta.files ?? []).map(String))
    for (const file of files) {
      if (!listed.has(rel(root, file))) warnings.push(`${rel(root, file)} is not listed in capability meta files`)
    }
  }

  return {
    ok: errors.length === 0,
    errors,
    warnings,
    counts: {
      records: entries.length,
      files: files.length,
      domains: domains.size,
      verified,
      implemented_unverified: implementedUnverified,
      money,
      duplicate_keys: Math.max(0, entries.length - seen.size),
    },
  }
}

function main() {
  parseArgs(process.argv.slice(2))
  const result = verifyCanonicalCapabilityShards({ root: process.cwd() })
  console.log(JSON.stringify(result, null, 2))
  process.exitCode = result.ok ? 0 : 1
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  main()
}
