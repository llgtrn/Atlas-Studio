#!/usr/bin/env bash
# Verifies the invariant in .atlas/contracts/DONOR-WORKBENCH-ISOLATION.md: no path under
# .atlas/temporary/donors/** is live/discoverable as agent-tooling configuration (a `.claude`,
# `.codex` or `.cursor` directory, or a bare CLAUDE.md/AGENTS.md/.mcp.json file). Donor content is
# census-visible but must never be repository or agent authority.
#
# Exit 0: clean. Exit 1: a live agent-tooling-shaped path was found under donors -- quarantine it
# (see the contract doc for the exact rename convention) before merging.
set -euo pipefail
cd "$(dirname "$0")/../.."

violations=()

while IFS= read -r -d '' path; do
  violations+=("$path")
done < <(find .atlas/temporary/donors -type d \( -iname ".claude" -o -iname ".codex" -o -iname ".cursor" \) -print0 2>/dev/null)

while IFS= read -r -d '' path; do
  violations+=("$path")
done < <(find .atlas/temporary/donors -type f \( -iname "CLAUDE.md" -o -iname "AGENTS.md" -o -iname ".mcp.json" \) -print0 2>/dev/null)

if [ "${#violations[@]}" -eq 0 ]; then
  echo "verify-donor-quarantine: clean -- no live agent-tooling-shaped paths under .atlas/temporary/donors/"
  exit 0
fi

echo "verify-donor-quarantine: FAILED -- found ${#violations[@]} live agent-tooling-shaped path(s):"
for v in "${violations[@]}"; do
  echo "  $v"
done
echo
echo "Quarantine each one per .atlas/contracts/DONOR-WORKBENCH-ISOLATION.md before merging:"
echo "  a .claude/.codex/.cursor directory -> rename to _donor-quarantine.<name-without-leading-dot>"
echo "  a CLAUDE.md/AGENTS.md/.mcp.json file -> append the suffix .donor-untrusted"
exit 1
