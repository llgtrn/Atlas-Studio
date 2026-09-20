// Bounded lexical masking for the small Rust grammar used by sync-anchor-v2.  This is not an
// AST: it only makes comments and literals incapable of manufacturing mod/cfg/call/test tokens.
// UTF-16 code units are replaced in place (newlines are retained), so offsets used by String
// APIs remain stable even when astral characters precede a token.
export function maskRustNonCode(source) {
  const out = source.split('')
  const blank = (from, to) => { for (let k = from; k < to; k += 1) if (out[k] !== '\n' && out[k] !== '\r') out[k] = ' ' }
  const rawStart = (i) => {
    let p = i
    if ((source[p] === 'b' || source[p] === 'c') && source[p + 1] === 'r') p += 1
    if (source[p] !== 'r') return null
    let q = p + 1
    while (source[q] === '#') q += 1
    return source[q] === '"' ? { hashes: q - p - 1, content: q + 1 } : null
  }
  let i = 0
  while (i < source.length) {
    if (source.startsWith('//', i)) {
      const end = source.indexOf('\n', i + 2); const stop = end < 0 ? source.length : end
      blank(i, stop); i = stop; continue
    }
    if (source.startsWith('/*', i)) {
      let depth = 1; let j = i + 2
      while (j < source.length && depth) {
        if (source.startsWith('/*', j)) { depth += 1; j += 2 } else if (source.startsWith('*/', j)) { depth -= 1; j += 2 } else j += 1
      }
      blank(i, j); i = j; continue
    }
    const raw = rawStart(i)
    if (raw) {
      const close = `"${'#'.repeat(raw.hashes)}`
      const found = source.indexOf(close, raw.content); const end = found < 0 ? source.length : found + close.length
      blank(i, end); i = end; continue
    }
    let quoteAt = i
    if ((source[i] === 'b' || source[i] === 'c') && source[i + 1] === '"') quoteAt = i + 1
    if (source[quoteAt] === '"') {
      let j = quoteAt + 1
      while (j < source.length) { if (source[j] === '\\') j += 2; else if (source[j++] === '"') break }
      blank(i, j); i = j; continue
    }
    // A lifetime is apostrophe + identifier without a closing apostrophe.  A char/byte-char has
    // a closing apostrophe after one scalar or escape; only the latter is masked.
    quoteAt = i
    if (source[i] === 'b' && source[i + 1] === "'") quoteAt = i + 1
    if (source[quoteAt] === "'") {
      let j = quoteAt + 1
      if (source[j] === '\\') j += 2; else if (j < source.length) j += 1
      if (source[j] === "'") { blank(i, j + 1); i = j + 1; continue }
    }
    i += 1
  }
  return out.join('')
}
