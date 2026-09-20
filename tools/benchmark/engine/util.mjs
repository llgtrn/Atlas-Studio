// Shared primitives for the benchmark truth engine. Extends existing controller libs; does not replace them.

export const SHA40 = /^[0-9a-f]{40}$/i

const FILLER = new Set(['', 'n/a', 'na', 'none', 'unknown', 'tbd', 'todo', 'not sure', '-', 'null'])

export function text(value) {
  return typeof value === 'string' ? value.trim() : ''
}

export function isObject(value) {
  return Boolean(value) && typeof value === 'object' && !Array.isArray(value)
}

export function isFiller(value) {
  return FILLER.has(text(value).toLowerCase())
}

export function hasText(value) {
  return text(value).length > 0 && !isFiller(value)
}

export function asArray(value) {
  return Array.isArray(value) ? value : []
}

export function unique(values) {
  return [...new Set(values.map(text).filter(Boolean))]
}
