#!/usr/bin/env node
import { writeFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { collectOpsReality, parseCli } from './lib.mjs'

const args = parseCli(process.argv.slice(2))
const opsRoot = resolve(String(args.ops ?? process.cwd()))
const report = collectOpsReality(opsRoot)
const json = `${JSON.stringify(report, null, 2)}\n`
if (args.output) writeFileSync(resolve(String(args.output)), json, 'utf8')
else process.stdout.write(json)

if (report.drift.some((entry) => entry.severity === 'error') && args.strict) process.exit(1)
