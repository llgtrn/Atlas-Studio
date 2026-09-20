import Database from 'better-sqlite3'
import { copyFileSync, existsSync, mkdtempSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { basename, join } from 'node:path'

export function openReadOnlyDatabase(dbPath, options = {}) {
  const testQuery = options.testQuery ?? "SELECT count(*) n FROM sqlite_master WHERE type='table'"
  if (!existsSync(dbPath)) {
    const error = new Error(`SQLite DB not found: ${dbPath}`)
    error.code = 'SQLITE_DB_MISSING'
    throw error
  }

  const openAndProbe = (path, cleanup = null) => {
    const db = new Database(path, { readonly: true, fileMustExist: true })
    try {
      db.prepare(testQuery).get()
    } catch (error) {
      db.close()
      cleanup?.()
      throw error
    }
    if (cleanup) {
      const close = db.close.bind(db)
      db.close = () => {
        try {
          return close()
        } finally {
          cleanup()
        }
      }
    }
    return db
  }

  try {
    return openAndProbe(dbPath)
  } catch (error) {
    if (!String(error.code ?? '').startsWith('SQLITE_IOERR')) throw error

    const tempDir = mkdtempSync(join(tmpdir(), 'chronica-sqlite-ro-'))
    const copyPath = join(tempDir, basename(dbPath))
    copyFileSync(dbPath, copyPath)
    try {
      return openAndProbe(copyPath, () => rmSync(tempDir, { recursive: true, force: true }))
    } catch (fallbackError) {
      fallbackError.message = `${fallbackError.message}; fallback copy after ${error.code} also failed`
      throw fallbackError
    }
  }
}
