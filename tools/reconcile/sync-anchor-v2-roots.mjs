import { existsSync } from 'node:fs'
import { basename, join, relative, sep } from 'node:path'
import { listRustFiles } from './sync-anchor-v2-shard.mjs'
import { resolveWithinRoot } from './sync-anchor-v2-fs-safety.mjs'

const slash = (p) => p.split(sep).join('/')
const quoted = (body, key) => body.match(new RegExp(`^\\s*${key}\\s*=\\s*"([^"]+)"`, 'm'))?.[1]

/** Discover every Rust crate root without executing cargo or trusting paths outside the crate. */
export function discoverRustRoots(cratePath, manifestText = '') {
  const src = join(cratePath, 'src'); const roots = []; const duplicateRootIds = []
  const add = (kind, name, path) => {
    const safe = resolveWithinRoot(cratePath, path)
    if (!safe.ok || !existsSync(safe.realPath)) return
    const id = `${kind}:${name}`
    if (roots.some((r) => r.id === id)) { duplicateRootIds.push(id); return }
    if (!roots.some((r) => r.path === safe.realPath)) roots.push({ id, kind, name, path: safe.realPath })
  }
  const libBody = manifestText.match(/\[lib\]([\s\S]*?)(?=\n\s*\[|$)/)?.[1] ?? ''
  add('lib', quoted(libBody, 'name') ?? 'lib', join(cratePath, quoted(libBody, 'path') ?? 'src/lib.rs'))
  add('bin', 'main', join(src, 'main.rs'))
  const binDir = join(src, 'bin')
  const listing = listRustFiles(binDir)
  for (const path of listing.files) {
    const rel = slash(relative(binDir, path))
    if (!rel.includes('/') || rel.endsWith('/main.rs')) add('bin', rel.endsWith('/main.rs') ? rel.slice(0, -8) : basename(rel, '.rs'), path)
  }
  for (const match of manifestText.matchAll(/\[\[bin\]\]([\s\S]*?)(?=\n\s*\[|$)/g)) {
    const path = quoted(match[1], 'path'); const name = quoted(match[1], 'name')
    if (path) add('bin', name ?? basename(path, '.rs'), join(cratePath, path))
  }
  return {
    roots: duplicateRootIds.length ? [] : roots.sort((a, b) => a.id < b.id ? -1 : a.id > b.id ? 1 : 0),
    duplicateRootIds: [...new Set(duplicateRootIds)].sort(),
    truncated: listing.truncated,
    unsafeEntries: listing.unsafeEntries,
    unreadableDirs: listing.unreadableDirs,
  }
}
