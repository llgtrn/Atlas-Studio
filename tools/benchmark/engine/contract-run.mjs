#!/usr/bin/env node
// Benchmark contract compiler CLI. Does not mutate product crates or P0 CI.

import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { execFileSync } from 'node:child_process'
import { compileContracts } from './contract-compile.mjs'
import { diffReleases } from './contract-diff.mjs'
import { nextActionable } from './contract-next.mjs'
import { buildCiMatrix } from './contract-ci-matrix.mjs'
import { engineRoot } from './contract-load.mjs'

const ROOT = engineRoot()

function arg(args, name) {
  const idx = args.indexOf(name)
  return idx >= 0 ? args[idx + 1] : undefined
}

function gitSha() {
  try {
    return execFileSync('git', ['rev-parse', 'HEAD'], { cwd: ROOT, encoding: 'utf8' }).trim()
  } catch {
    return 'UNBOUND'
  }
}

function compile(write = true) {
  return compileContracts({
    root: ROOT,
    write,
    repositorySha: gitSha(),
    generatedAt: process.env.BENCHMARK_RELEASE_TIME || '2026-08-21T00:00:00Z',
    releaseId: 'BR-2026.08.21.1',
  })
}

function main() {
  const args = process.argv.slice(2)
  const cmd = args[0] || 'compile'
  if (cmd === 'compile' || cmd === 'release') {
    const built = compile(true)
    if (!built.ok) {
      console.error(built.errors.slice(0, 40).join('\n'))
      process.exitCode = 1
    }
    console.log(
      JSON.stringify(
        {
          ok: built.ok,
          release: built.release.id,
          hash: built.release.hash,
          source_hash: built.ir.source_hash,
          flows: built.ir.flows.length,
          packets: built.packets.length,
          errors: built.errors.length,
        },
        null,
        2,
      ),
    )
    return
  }
  if (cmd === 'verify') {
    const built = compile(false)
    const again = compileContracts({
      root: ROOT,
      write: false,
      repositorySha: built.release.benchmark_sha,
      generatedAt: '2026-08-21T00:00:00Z',
      releaseId: 'BR-2026.08.21.1',
    })
    const drift = built.release.hash !== again.release.hash || built.ir.source_hash !== again.ir.source_hash
    const packetDrift = built.packets.some((packet) => {
      const disk = JSON.parse(readFileSync(resolve(ROOT, 'tools/benchmark/contracts/generated/packets', `${packet.cap}.json`), 'utf8'))
      return disk.packet_hash !== packet.packet_hash
    })
    const ok = built.ok && again.ok && !drift && !packetDrift
    if (!ok) process.exitCode = 1
    console.log(JSON.stringify({ ok, drift, packetDrift, errors: [...built.errors, ...again.errors].slice(0, 20) }, null, 2))
    return
  }
  if (cmd === 'flows') {
    const built = compile(false)
    console.log(JSON.stringify({ ok: built.ok, count: built.ir.flows.length, flows: built.ir.flows.map((f) => ({ id: f.id, owner: f.owner, status: f.derived_status, verdict: f.verdict })) }, null, 2))
    return
  }
  if (cmd === 'packet' || cmd === 'cap-pack') {
    const cap = arg(args, '--cap')
    const built = compile(false)
    const packet = built.packets.find((row) => row.cap === cap) || built.packets[0]
    console.log(JSON.stringify(packet, null, 2))
    return
  }
  if (cmd === 'diff') {
    const built = compile(false)
    const synthetic = JSON.parse(JSON.stringify(built.ir))
    synthetic.flows = synthetic.flows.map((flow) =>
      flow.owner === 'CAP18' ? { ...flow, hard_gates: [...new Set([...(flow.hard_gates || []), 'OUTPUT_DLP_REQUIRED'])] } : flow,
    )
    const diff = diffReleases(built.ir, synthetic)
    console.log(JSON.stringify({ ok: true, diff }, null, 2))
    return
  }
  if (cmd === 'next') {
    const built = compile(false)
    console.log(JSON.stringify(nextActionable(built.ir, arg(args, '--cap'), built.release.id), null, 2))
    return
  }
  if (cmd === 'ci-matrix') {
    const built = compile(false)
    const changed = arg(args, '--changed')
    const paths = changed ? changed.split(',') : asArrayFromRest(args)
    console.log(JSON.stringify(buildCiMatrix(built.ir, paths), null, 2))
    return
  }
  console.error(`unknown command ${cmd}`)
  process.exitCode = 1
}

function asArrayFromRest(args) {
  const idx = args.indexOf('--changed-list')
  return idx >= 0 ? args.slice(idx + 1) : []
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main()
}
