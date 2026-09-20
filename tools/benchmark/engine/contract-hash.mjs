import { createHash } from 'node:crypto'

export function canonicalJson(value) {
  if (value === undefined) return 'null'
  if (Array.isArray(value)) return `[${value.map(canonicalJson).join(',')}]`
  if (value && typeof value === 'object') {
    const keys = Object.keys(value)
      .filter((key) => key !== 'generated_at' && key !== 'compiled_at' && key !== 'hash' && key !== 'packet_hash')
      .sort()
    return `{${keys.map((key) => `${JSON.stringify(key)}:${canonicalJson(value[key])}`).join(',')}}`
  }
  return JSON.stringify(value)
}

export function sha256Hex(value) {
  const payload = typeof value === 'string' ? value : canonicalJson(value)
  return createHash('sha256').update(payload).digest('hex')
}
