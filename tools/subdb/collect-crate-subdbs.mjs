#!/usr/bin/env node
import { collectCrateSubdbs, parseCli } from './subdb-lib.mjs'

const options = parseCli(process.argv.slice(2))
console.log(JSON.stringify(collectCrateSubdbs({ root: process.cwd(), crates: options.crates, write: options.write }), null, 2))
