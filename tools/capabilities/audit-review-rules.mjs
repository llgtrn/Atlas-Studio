#!/usr/bin/env node
// audit-review-rules.mjs — integrator-side honesty audit of an expanded review manifest.
// Consumes the manifest emitted by gen-file-review-from-rules.mjs on stdin and flags the
// classification calls most likely to be dishonest:
//   - real code files parked in non_behavioral_support / generated_vendor_build_artifact
//     (behavior hidden in a "no behavior" bucket)
//   - files in test_fixture that don't look like tests/fixtures/mocks/testdata/examples
//   - per-classification totals + mapped/dup id coverage
// Read-only; prints a report to stdout. Exit 0 always (advisory) unless --strict (exit 1 on flags).
//
// Usage:  node tools/capabilities/gen-file-review-from-rules.mjs --rules X.json 2>/dev/null \
//           | node tools/capabilities/audit-review-rules.mjs [--strict] [--show N]
import process from 'node:process'

const strict = process.argv.includes('--strict')
const showIdx = process.argv.indexOf('--show')
const SHOW = showIdx >= 0 ? Number(process.argv[showIdx + 1]) : 25

const CODE = /\.(ts|tsx|js|jsx|mjs|cjs|go|py|rs|java|kt|kts|rb|php|ex|exs|scala|c|cc|cpp|cxx|h|hpp|cs|swift|sql|graphql|proto|vue|svelte|sh|bash|ps1)$/i
// things that are legitimately non-behavioral even though they have a code-ish extension
const GENERATED_OK = /(\.pb\.go|_pb2\.py|\.pb\.gw\.go|_grpc\.pb\.go|\.generated\.|\.gen\.go|\.min\.js|\.d\.ts)$/i
const TESTISH = /(^|\/)(tests?|__tests__|testdata|test_data|fixtures?|mocks?|__mocks__|e2e|spec|specs|examples?|demo|samples?|stub|stubs|seeders?|factories|cassettes|snapshots?|__snapshots__)(\/|$)|(_test\.go$)|(\.(test|spec)\.[a-z]+$)|(test_[^/]*\.py$)|(_testcase\.go$)|(\.snap$)|(\.golden$)/i

let data = ''
process.stdin.setEncoding('utf8')
process.stdin.on('data', c => (data += c))
process.stdin.on('end', () => {
  const m = JSON.parse(data)
  const byCls = {}
  const flags = { code_in_nonbehavioral: [], code_in_vendor: [], nontest_in_fixture: [] }
  let mappedPaths = 0, dupPaths = 0
  for (const r of m.reviews) {
    byCls[r.classification] = (byCls[r.classification] || 0) + r.paths.length
    if (r.classification === 'mapped') mappedPaths += r.paths.length
    if (r.classification === 'duplicate_variant_mapped') dupPaths += r.paths.length
    for (const p of r.paths) {
      if (r.classification === 'non_behavioral_support' && CODE.test(p) && !GENERATED_OK.test(p)) flags.code_in_nonbehavioral.push(p)
      if (r.classification === 'generated_vendor_build_artifact' && CODE.test(p) && !GENERATED_OK.test(p) && !/(vendor|third_party|node_modules)\//i.test(p)) flags.code_in_vendor.push(p)
      if (r.classification === 'test_fixture' && !TESTISH.test(p)) flags.nontest_in_fixture.push(p)
    }
  }
  const total = Object.values(byCls).reduce((a, b) => a + b, 0)
  console.log(`donor=${m.donor} reviewed_by=${m.reviewed_by}`)
  console.log(`total reviewed paths: ${total}`)
  for (const [c, n] of Object.entries(byCls).sort((a, b) => b[1] - a[1])) console.log(`  ${String(n).padStart(7)}  ${c}`)
  console.log(`mapped paths: ${mappedPaths} · duplicate_variant paths: ${dupPaths} · new sources: ${(m.sources || []).length}`)
  const report = (label, arr) => {
    console.log(`\n[${arr.length ? 'FLAG' : 'ok'}] ${label}: ${arr.length}`)
    arr.slice(0, SHOW).forEach(p => console.log(`    ${p}`))
    if (arr.length > SHOW) console.log(`    …and ${arr.length - SHOW} more`)
  }
  report('code files in non_behavioral_support (possible hidden behavior)', flags.code_in_nonbehavioral)
  report('code files in generated_vendor_build_artifact (not under vendor/)', flags.code_in_vendor)
  report('non-test-looking files in test_fixture', flags.nontest_in_fixture)
  const flagged = flags.code_in_nonbehavioral.length + flags.code_in_vendor.length + flags.nontest_in_fixture.length
  console.log(`\n${flagged === 0 ? '✓ no honesty flags' : '⚠ ' + flagged + ' flag(s) to review'}`)
  if (strict && flagged) process.exit(1)
})
