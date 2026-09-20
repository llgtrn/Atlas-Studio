import {
  BLACKBOX_ANTI_PATTERNS,
  HARD_FAILURE_TAGS as BLACKBOX_HARD,
} from './blackbox-vocab.mjs'
import { EMPLOYMENT_ANTI_PATTERNS, HARD_FAILURE_TAGS as EMPLOYMENT_HARD } from './employment-vocab.mjs'
import { STORE_ANTI_PATTERNS, HARD_FAILURE_TAGS as STORE_HARD } from './store-vocab.mjs'
import { WEB2APP_ANTI_PATTERNS } from './web2app-vocab.mjs'
import { ANTI_PATTERNS as ENGINE_ANTI } from './vocab.mjs'
import { CONTRACT_ANTI_PATTERNS } from './contract-vocab.mjs'

export function unifiedHardGates() {
  return new Set([
    ...ENGINE_ANTI,
    ...WEB2APP_ANTI_PATTERNS,
    ...EMPLOYMENT_ANTI_PATTERNS,
    ...STORE_ANTI_PATTERNS,
    ...BLACKBOX_ANTI_PATTERNS,
    ...CONTRACT_ANTI_PATTERNS,
    ...EMPLOYMENT_HARD,
    ...STORE_HARD,
    ...BLACKBOX_HARD,
  ])
}

export function overlayHardGates() {
  return new Set([
    ...WEB2APP_ANTI_PATTERNS,
    ...EMPLOYMENT_ANTI_PATTERNS,
    ...STORE_ANTI_PATTERNS,
    ...BLACKBOX_ANTI_PATTERNS,
    ...CONTRACT_ANTI_PATTERNS,
    ...EMPLOYMENT_HARD,
    ...STORE_HARD,
    ...BLACKBOX_HARD,
  ])
}

export function unifiedAntiPatternIds() {
  return [...unifiedHardGates()]
}
