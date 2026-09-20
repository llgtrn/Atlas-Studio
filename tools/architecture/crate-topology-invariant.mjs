import { existsSync, readdirSync } from 'node:fs'
import { join } from 'node:path'

export const CANONICAL_CRATE_ROOTS = Object.freeze(['adapter', 'cap', 'core', 'runtime'])

// crates/infra is not a fifth position in the product stack -- it is developer/build tooling
// (INFRA.FABRIC) living outside the product dependency graph entirely, hard-firewalled from
// core/cap/adapter/runtime in both directions by
// tools/refoundation/validate-refoundation-layers.mjs. It is the one permanent NON-product
// exception this invariant tolerates; a domain-specific PRODUCT hierarchy (crates/fintech,
// crates/digital_twin, etc.) remains rejected exactly as before.
export const NON_PRODUCT_EXCEPTION_CRATE_ROOTS = Object.freeze(['infra'])

export function verifyCanonicalCrateTopology({ root = process.cwd() } = {}) {
  const cratesDir = join(root, 'crates')
  if (!existsSync(cratesDir)) {
    return { ok: false, errors: ['missing crates/ directory'], roots: [] }
  }

  const entries = readdirSync(cratesDir, { withFileTypes: true })
  const byName = new Map(entries.map((entry) => [entry.name, entry]))
  const errors = []

  for (const name of CANONICAL_CRATE_ROOTS) {
    const entry = byName.get(name)
    if (!entry) errors.push(`missing canonical crate root: crates/${name}`)
    else if (!entry.isDirectory()) errors.push(`canonical crate root is not a directory: crates/${name}`)
  }

  const allowed = new Set([...CANONICAL_CRATE_ROOTS, ...NON_PRODUCT_EXCEPTION_CRATE_ROOTS])
  const roots = entries.filter((entry) => entry.isDirectory()).map((entry) => entry.name).sort()
  for (const name of roots) {
    if (!allowed.has(name)) {
      errors.push(
        `non-canonical crate root: crates/${name}; domain/product code must remain under crates/{core,cap,adapter,runtime}`,
      )
    }
  }

  return { ok: errors.length === 0, errors, roots }
}
