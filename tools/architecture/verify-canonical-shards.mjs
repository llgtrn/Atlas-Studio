#!/usr/bin/env node
import { existsSync } from 'node:fs'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'
import {
  ARCHITECTURE_CANONICAL_DIR,
  loadArchitectureShards,
  parseArgs,
  rel,
  validateArrayOfStrings,
  validateBoolean,
  validateSorted,
  validateString,
} from '../canonical-shards/jsonl-lib.mjs'

const REQUIRED_FILES = ['nodes.jsonl', 'links.jsonl', 'gaps.jsonl', 'evidence.jsonl']
const GENERATED_DB_REF_PATTERN = /\b[a-z0-9._-]+\.db:/i

function sortKey(record) {
  return [
    record.record_type ?? '',
    record.id ?? record.key ?? record.capability_key ?? record.architecture_node ?? record.actor ?? '',
    record.relationship ?? record.kind ?? record.from_node ?? '',
    record.to_node ?? record.target_crate ?? '',
  ].join('|')
}

function countByType(entries) {
  const counts = {}
  for (const entry of entries) {
    counts[entry.record.record_type] = (counts[entry.record.record_type] ?? 0) + 1
  }
  return counts
}

export function verifyCanonicalArchitectureShards({ root = process.cwd() } = {}) {
  const errors = []
  const warnings = []
  const { meta, metaPath, files, entries } = loadArchitectureShards(root)
  const shardRoot = join(root, ARCHITECTURE_CANONICAL_DIR)

  if (!meta) errors.push(`${rel(root, metaPath)} is required`)
  for (const fileName of REQUIRED_FILES) {
    const path = join(shardRoot, fileName)
    if (!existsSync(path)) errors.push(`${rel(root, path)} is required`)
  }
  for (const file of files) {
    validateSorted(errors, entries.filter((entry) => entry.file === file), sortKey, 'architecture record')
  }

  const nodes = new Set()
  const capabilityLinks = new Set()
  const capabilityGaps = new Set()

  for (const entry of entries) {
    const record = entry.record
    const path = `${rel(root, entry.file)}:${entry.line}`
    if (!record || typeof record !== 'object' || Array.isArray(record)) {
      errors.push(`${path}: record must be an object`)
      continue
    }
    if (record.schema_version !== 1) errors.push(`${path}.schema_version must be 1`)
    const recordType = validateString(errors, record, 'record_type', path)

    if (recordType === 'architecture_node') {
      const id = validateString(errors, record, 'id', path)
      validateString(errors, record, 'kind', path)
      validateString(errors, record, 'name', path)
      validateString(errors, record, 'status', path)
      validateString(errors, record, 'crate', path, { nullable: true })
      validateString(errors, record, 'module_path', path, { nullable: true })
      validateString(errors, record, 'file_path', path, { nullable: true })
      validateString(errors, record, 'evidence_ref', path, { nullable: true })
      if (id) {
        if (nodes.has(id)) errors.push(`${path}: duplicate architecture_node id ${id}`)
        nodes.add(id)
      }
    } else if (recordType === 'architecture_edge') {
      validateString(errors, record, 'from_node', path)
      validateString(errors, record, 'to_node', path)
      validateString(errors, record, 'relationship', path)
      validateString(errors, record, 'evidence_ref', path, { nullable: true })
    } else if (recordType === 'capability_architecture_link') {
      const key = validateString(errors, record, 'capability_key', path)
      validateString(errors, record, 'architecture_node', path)
      validateString(errors, record, 'relationship', path)
      validateString(errors, record, 'status', path, { nullable: true })
      validateString(errors, record, 'target_crate', path, { nullable: true })
      validateString(errors, record, 'target_module', path, { nullable: true })
      if (key) capabilityLinks.add(key)
    } else if (recordType === 'architecture_target_gap') {
      const key = validateString(errors, record, 'capability_key', path)
      validateString(errors, record, 'gap_kind', path)
      validateString(errors, record, 'reason', path)
      const status = validateString(errors, record, 'status', path)
      validateString(errors, record, 'target_crate', path, { nullable: true })
      validateString(errors, record, 'target_module', path, { nullable: true })
      validateString(errors, record, 'suggested_architecture_node', path, { nullable: true })
      validateString(errors, record, 'evidence_ref', path, { nullable: true })
      if (status === 'implemented' || status === 'verified') errors.push(`${path}: architecture_target_gap must not claim ${status}`)
      if (key) capabilityGaps.add(key)
    } else if (recordType === 'planned_architecture_node') {
      validateString(errors, record, 'id', path)
      validateString(errors, record, 'kind', path)
      validateString(errors, record, 'name', path)
      validateString(errors, record, 'status', path)
      validateString(errors, record, 'reason', path)
      validateString(errors, record, 'logical_domain', path, { nullable: true })
      validateString(errors, record, 'intended_owner_crate', path, { nullable: true })
      validateString(errors, record, 'evidence_ref', path, { nullable: true })
    } else if (recordType === 'architecture_evidence') {
      validateString(errors, record, 'architecture_node', path)
      validateString(errors, record, 'status', path)
      validateString(errors, record, 'evidence_kind', path)
      validateString(errors, record, 'evidence_ref', path)
      if (GENERATED_DB_REF_PATTERN.test(record.evidence_ref ?? '')) {
        errors.push(`${path}.evidence_ref must not cite generated DB authority: ${record.evidence_ref}`)
      }
      validateString(errors, record, 'test_command', path, { nullable: true })
      validateString(errors, record, 'test_result', path, { nullable: true })
      validateString(errors, record, 'verified_at', path, { nullable: true })
      validateString(errors, record, 'verified_by', path, { nullable: true })
      validateString(errors, record, 'blocker_reason', path, { nullable: true })
    } else if (recordType === 'architecture_invariant') {
      validateString(errors, record, 'key', path)
      validateString(errors, record, 'name', path)
      validateString(errors, record, 'description', path)
      validateString(errors, record, 'enforcing_node', path)
      validateString(errors, record, 'verification_command', path)
      validateString(errors, record, 'status', path)
    } else if (recordType === 'authority_rule') {
      for (const field of ['actor', 'scope', 'may_see', 'may_propose', 'may_approve', 'may_execute', 'may_audit', 'forbidden_actions', 'enforcing_node']) {
        validateString(errors, record, field, path)
      }
    } else if (recordType === 'data_flow') {
      for (const field of ['key', 'source_node', 'target_node', 'data_kind', 'scope_rule']) validateString(errors, record, field, path)
      validateBoolean(errors, record, 'gate_required', path)
      validateBoolean(errors, record, 'audit_required', path)
    } else if (recordType === 'architecture_status_override') {
      for (const field of ['architecture_node', 'status', 'reason', 'updated_at', 'updated_by']) validateString(errors, record, field, path)
    } else if (recordType) {
      errors.push(`${path}.record_type is invalid: ${recordType}`)
    }
  }

  for (const entry of entries) {
    const record = entry.record
    const path = `${rel(root, entry.file)}:${entry.line}`
    if (record.record_type === 'architecture_edge') {
      if (!nodes.has(record.from_node)) errors.push(`${path}.from_node references missing node ${record.from_node}`)
      if (!nodes.has(record.to_node)) errors.push(`${path}.to_node references missing node ${record.to_node}`)
    }
    if (record.record_type === 'capability_architecture_link' && !nodes.has(record.architecture_node)) {
      errors.push(`${path}.architecture_node references missing node ${record.architecture_node}`)
    }
    if (record.record_type === 'architecture_evidence' && !nodes.has(record.architecture_node)) {
      errors.push(`${path}.architecture_node references missing node ${record.architecture_node}`)
    }
    if (record.record_type === 'architecture_invariant' && !nodes.has(record.enforcing_node)) {
      errors.push(`${path}.enforcing_node references missing node ${record.enforcing_node}`)
    }
    if (record.record_type === 'authority_rule' && !nodes.has(record.enforcing_node)) {
      errors.push(`${path}.enforcing_node references missing node ${record.enforcing_node}`)
    }
    if (record.record_type === 'data_flow') {
      if (!nodes.has(record.source_node)) errors.push(`${path}.source_node references missing node ${record.source_node}`)
      if (!nodes.has(record.target_node)) errors.push(`${path}.target_node references missing node ${record.target_node}`)
    }
    if (record.record_type === 'architecture_target_gap' && record.suggested_architecture_node && !nodes.has(record.suggested_architecture_node)) {
      errors.push(`${path}.suggested_architecture_node references missing node ${record.suggested_architecture_node}`)
    }
  }

  for (const key of capabilityLinks) {
    if (capabilityGaps.has(key)) errors.push(`capability ${key} is both linked and gapped`)
  }

  const counts = countByType(entries)
  if (meta) {
    if (meta.schema_version !== 1) errors.push(`${rel(root, metaPath)}.schema_version must be 1`)
    if (meta.authority !== 'canonical_jsonl') errors.push(`${rel(root, metaPath)}.authority must be canonical_jsonl`)
    for (const [table, count] of Object.entries(meta.counts ?? {})) {
      if (Number(count) !== Number(counts[table] ?? 0)) {
        errors.push(`${rel(root, metaPath)}.counts.${table}=${count} but JSONL has ${counts[table] ?? 0}`)
      }
    }
  }

  return {
    ok: errors.length === 0,
    errors,
    warnings,
    counts: {
      ...counts,
      nodes: nodes.size,
      linked_capabilities: capabilityLinks.size,
      gapped_capabilities: capabilityGaps.size,
    },
  }
}

function main() {
  parseArgs(process.argv.slice(2))
  const result = verifyCanonicalArchitectureShards({ root: process.cwd() })
  console.log(JSON.stringify(result, null, 2))
  process.exitCode = result.ok ? 0 : 1
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  main()
}
