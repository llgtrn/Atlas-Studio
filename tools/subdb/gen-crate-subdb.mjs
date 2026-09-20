#!/usr/bin/env node
import { generateCrateSubdbs, parseCli, printResult } from './subdb-lib.mjs'

const options = parseCli(process.argv.slice(2))
printResult(generateCrateSubdbs({ root: process.cwd(), crates: options.crates }), options)
