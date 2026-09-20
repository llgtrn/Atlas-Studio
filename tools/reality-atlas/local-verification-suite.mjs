import { runAndRecord } from './local-verification.mjs'
import { defaultLocalVerificationChecks } from './local-verification-plan.mjs'

function parse(argv) {
  return {
    full: argv.includes('--full'),
    keepGoing: argv.includes('--keep-going'),
  }
}

const options = parse(process.argv.slice(2))
const checks = defaultLocalVerificationChecks({ full: options.full })
let failed = false

for (const spec of checks) {
  console.log(`\n[local-verification] ${spec.check}`)
  const { item } = runAndRecord(process.cwd(), spec)
  console.log(`[local-verification] ${item.status} ${item.check} (${item.durationMs ?? '?'}ms)`)
  if (item.status !== 'PASS') {
    failed = true
    if (!options.keepGoing) break
  }
}

if (failed) process.exitCode = 1
