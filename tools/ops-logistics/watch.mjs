#!/usr/bin/env node
import { watch } from 'node:fs'
import { mkdirSync, writeFileSync } from 'node:fs'
import { dirname, resolve, sep } from 'node:path'
import { collectOpsReality, parseCli } from './lib.mjs'

const args = parseCli(process.argv.slice(2))
const opsRoot = resolve(String(args.ops ?? process.cwd()))
const output = resolve(opsRoot, String(args.output ?? '.chronica/ops-reality.json'))
const debounceMs = Number(args.debounce ?? 500)
let timer = null
let building = false
let pending = false

function slash(value) {
  return String(value).split(sep).join('/')
}

function ignored(filename) {
  const value = slash(filename ?? '')
  return !value || value.startsWith('.git/') || value.startsWith('.chronica/') || value.includes('/node_modules/') || value.includes('/target/')
}

function build() {
  if (building) {
    pending = true
    return
  }
  building = true
  try {
    const reality = collectOpsReality(opsRoot)
    mkdirSync(dirname(output), { recursive: true })
    writeFileSync(output, `${JSON.stringify(reality, null, 2)}\n`, 'utf8')
    console.log(`[ops-reality] ${reality.headSha.slice(0, 12)} source=${reality.stats.sourceFiles} semantics=${reality.stats.semanticMappings} legacy=${reality.stats.legacyCounterTotal} drift=${reality.stats.driftFindings}`)
  } catch (error) {
    console.error(`[ops-reality] build failed: ${error instanceof Error ? error.message : String(error)}`)
  } finally {
    building = false
    if (pending) {
      pending = false
      build()
    }
  }
}

function schedule(filename) {
  if (ignored(filename)) return
  if (timer) clearTimeout(timer)
  timer = setTimeout(build, debounceMs)
}

build()
console.log(`[ops-reality] watching ${opsRoot}; output=${output}`)
const watcher = watch(opsRoot, { recursive: true }, (_eventType, filename) => schedule(filename))

for (const signal of ['SIGINT', 'SIGTERM']) {
  process.on(signal, () => {
    watcher.close()
    process.exit(0)
  })
}
