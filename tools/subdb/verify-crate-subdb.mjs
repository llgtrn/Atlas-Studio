#!/usr/bin/env node
import { parseCli, printResult, verifyCrateSubdbs } from './subdb-lib.mjs'

const options = parseCli(process.argv.slice(2))
const result = verifyCrateSubdbs({ root: process.cwd(), crates: options.crates })
printResult(result, options)
process.exitCode = result.ok ? 0 : 1
