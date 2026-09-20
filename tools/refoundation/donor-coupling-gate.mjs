#!/usr/bin/env node
// Donor coupling gate (CHRONICA -- DISPATCH-FIRST MASS ABSORPTION ENGINE): a donor source file
// may only be deleted from temporary/<donor>/ once absorbed IF no other file remaining in that
// donor's own tree still depends on it. Chronica never builds the donor tree, but a dangling
// donor-internal reference still falsifies the "this file was safely deletable" judgment a
// Builder made -- it is a correctness bug in the drain, not a cosmetic one.
//
// DELETE donor source IFF no remaining donor source depends on that slice, OR the whole
// dependent slice is being drained atomically in the same candidate. "The donor tree is never
// built, so a dangling import elsewhere is fine" is not an acceptable justification -- every
// donor-source deletion this session that used that reasoning turned out to leave a real broken
// reference (cilium/pkg/idpool, vault/shamir, minio/cmd/httprange, containerd/pkg/identifiers,
// coredns/plugin/pkg/dnsutil/ttl.go, spicedb/pkg/zedtoken, opa/v1/util/backoff.go,
// nats/server/rate_counter.go, openfga/pkg/encoder/token_serializer.go,
// restate's invocation_task/retry_after.rs mod declaration, postgresql's mvcc.sgml entity) --
// all restored in the same change that added this gate.
//
// This gate diffs `--since <sha>` against the current working tree, finds every file deleted
// under temporary/<donor>/, extracts the symbols/module-name/filename it defined, and checks
// whether any file still present in that donor's tree still references them. It is a heuristic,
// language-general check (Go func/type, Rust pub items + mod declarations, and a generic
// same-filename reference check for every other language) that deliberately prefers a false
// positive (blocking a genuinely safe deletion, fixable by restoring the file or draining the
// whole dependent chain atomically) over a false negative (silently accepting a broken donor
// tree).
//
// Usage: node tools/refoundation/donor-coupling-gate.mjs --since <sha>

import { execFileSync } from 'node:child_process'
import { readFileSync, existsSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { dirname, join, basename, extname } from 'node:path'

function packageDirOf(path) {
  return dirname(path)
}

const HERE = dirname(fileURLToPath(import.meta.url))
const DEFAULT_ROOT = join(HERE, '..', '..')

function git(root, args) {
  return execFileSync('git', args, { cwd: root, encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 })
}

function deletedTemporaryFiles(root, sinceSha) {
  const out = git(root, ['diff', '--name-status', '--diff-filter=D', sinceSha, '--', 'temporary/'])
  return out
    .split('\n')
    .filter(Boolean)
    .map((line) => line.split('\t')[1])
    .filter(Boolean)
}

function donorOf(path) {
  const m = path.match(/^temporary\/([^/]+)\//)
  return m ? m[1] : null
}

// Go: only free functions and type declarations are extracted -- deliberately NOT receiver
// methods. A method name like `String`/`Error`/`Close` is reused across dozens of unrelated types
// satisfying the same common interface throughout a large donor package, so bare-matching a
// receiver-method name produces overwhelming false positives (confirmed: minio's HTTPRangeSpec
// .String()/.ToHeader() collided with ~120 unrelated files in cmd/ before this filter was added).
// Free-function and type names don't have that ambiguity: Go disallows two top-level
// functions/types with the same name in one package, so a bare match of one elsewhere in the same
// package is a reliable signal. This trades recall for precision -- a slice whose only remaining
// coupling is via a receiver method won't be caught by this gate and needs the Builder's own
// DELETION_SCOPE judgment, same as before this gate existed.
const GO_FUNC_RE = /^func\s+([A-Za-z_]\w*)\s*\(/gm
const GO_TYPE_RE = /^type\s+([A-Za-z_]\w*)\s/gm
const RUST_ITEM_RE = /^pub\s+(?:fn|struct|enum|trait|const|static)\s+([A-Za-z_]\w*)/gm

function extractSymbols(content, ext) {
  const symbols = new Set()
  if (ext === '.go') {
    for (const re of [GO_FUNC_RE, GO_TYPE_RE]) {
      let m
      while ((m = re.exec(content))) symbols.add(m[1])
    }
  } else if (ext === '.rs') {
    let m
    while ((m = RUST_ITEM_RE.exec(content))) symbols.add(m[1])
  }
  return symbols
}

/** Returns violations: [{ donor, deletedPath, referencingFile, reason }].
 * `root` defaults to the real Chronica checkout; tests pass an isolated synthetic git repo. */
export function findDonorCouplingViolations(sinceSha, root = DEFAULT_ROOT) {
  const violations = []
  const deleted = deletedTemporaryFiles(root, sinceSha)
  const deletedByDonor = new Map()
  for (const path of deleted) {
    const donor = donorOf(path)
    if (!donor) continue
    if (!deletedByDonor.has(donor)) deletedByDonor.set(donor, [])
    deletedByDonor.get(donor).push(path)
  }

  for (const [donor, paths] of deletedByDonor) {
    const donorRoot = join(root, 'temporary', donor)
    if (!existsSync(donorRoot)) continue // whole donor removed -- nothing left to dangle

    const remainingFiles = git(root, ['ls-files', `temporary/${donor}`])
      .split('\n')
      .filter(Boolean)

    for (const deletedPath of paths) {
      const ext = extname(deletedPath)
      const stem = basename(deletedPath, ext)
      const packageDir = packageDirOf(deletedPath)
      const packageName = basename(packageDir)
      let beforeContent
      try {
        beforeContent = git(root, ['show', `${sinceSha}:${deletedPath}`])
      } catch {
        continue // pre-deletion content unavailable -- skip rather than guess
      }
      const symbols = extractSymbols(beforeContent, ext)

      for (const remaining of remainingFiles) {
        if (remaining === deletedPath) continue
        let text
        try {
          text = readFileSync(join(root, remaining), 'utf8')
        } catch {
          continue
        }

        if (ext === '.rs') {
          const modRe = new RegExp(`\\bmod\\s+${stem}\\s*;`)
          if (modRe.test(text)) {
            violations.push({
              donor,
              deletedPath,
              referencingFile: remaining,
              reason: `dangling module declaration for deleted "${stem}"`,
            })
            continue
          }
        }

        // Go visibility: an unexported (lowercase) symbol is only visible to sibling files in the
        // SAME directory/package -- a same-named lowercase identifier anywhere else is a
        // different, unrelated thing by construction and must not be flagged. An exported
        // (capitalized) symbol is visible everywhere, but a file in a different directory must
        // reference it package-qualified (e.g. "shamir.Split("); requiring qualification for
        // cross-directory matches avoids false positives from common exported names (Split, New,
        // Parse, ...) that legitimately exist in many unrelated packages across a large donor tree.
        const sameDirectory = packageDirOf(remaining) === packageDir
        let hitSymbol = null
        for (const symbol of symbols) {
          const isExported = /^[A-Z]/.test(symbol)
          if (ext === '.go' && !isExported && !sameDirectory) continue
          const pattern =
            ext === '.go' && !sameDirectory
              ? `\\b${packageName}\\.${symbol}\\b`
              : `\\b${symbol}\\b`
          if (new RegExp(pattern).test(text)) {
            hitSymbol = symbol
            break
          }
        }
        if (hitSymbol) {
          violations.push({
            donor,
            deletedPath,
            referencingFile: remaining,
            reason: `references exported symbol "${hitSymbol}" defined only in the deleted file`,
          })
          continue
        }

        if (symbols.size === 0) {
          const nameRe = new RegExp(`\\b${stem}${ext.replace('.', '\\.')}\\b`)
          if (nameRe.test(text)) {
            violations.push({
              donor,
              deletedPath,
              referencingFile: remaining,
              reason: `references the deleted file's own name "${stem}${ext}"`,
            })
          }
        }
      }
    }
  }
  return violations
}

function main(argv = process.argv.slice(2)) {
  const sinceIdx = argv.indexOf('--since')
  const sinceSha = sinceIdx >= 0 ? argv[sinceIdx + 1] : 'HEAD'
  const violations = findDonorCouplingViolations(sinceSha, DEFAULT_ROOT)
  if (violations.length === 0) {
    console.log(
      `donor-coupling-gate: passed -- every donor-source deletion since ${sinceSha} left no dangling reference in that donor's retained tree.`,
    )
    return 0
  }
  console.error(
    `donor-coupling-gate: ${violations.length} violation(s) -- a deleted donor file is still referenced by other retained donor source.`,
  )
  for (const v of violations) {
    console.error(`  ${v.donor}: ${v.deletedPath} deleted, but ${v.referencingFile} ${v.reason}`)
  }
  console.error(
    '\nEither restore the deleted file (a native reimplementation stands on its own regardless of whether the donor file stays) or drain the whole dependent chain atomically in the same candidate.',
  )
  return 1
}

if (process.argv[1] === fileURLToPath(import.meta.url)) process.exit(main())
