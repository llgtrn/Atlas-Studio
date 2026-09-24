#!/usr/bin/env bash
# Verifies the invariant in .atlas/contracts/DONOR-WORKBENCH-ISOLATION.md: no path under
# .atlas/temporary/donors/** is live/discoverable as agent-tooling configuration (a `.claude`,
# `.codex` or `.cursor` directory; a bare CLAUDE.md/AGENTS.md/.mcp.json file; or a GitHub Copilot
# ambient-instruction file, copilot-instructions.md or .github/instructions/*.instructions.md).
# Donor content is census-visible but must never be repository or agent authority.
#
# Exit 0: clean. Exit 1: a live agent-tooling-shaped path was found under donors -- quarantine it
# (see the contract doc for the exact rename convention) before merging.
set -euo pipefail
cd "$(dirname "$0")/../.."

# Overridable so this script is directly testable against a scratch fixture (see
# runtime/src/lib.rs's own donor_quarantine_script_symlink_detection test module) without ever
# touching this repository's own real donor corpus during a test run.
donors_root="${1:-.atlas/temporary/donors}"

violations=()

# `find ... -type d`/`-type f` classify a symlink by its OWN type (`l`), never by what it resolves
# to -- so a path named exactly `.claude` that is a *symlink* to a real directory was previously
# invisible to this scan, even though it is exactly as live and agent-tooling-discoverable as a
# literal `.claude` directory (normal filesystem traversal follows it transparently). Real donor
# trees in this corpus already use this exact symlink-to-sibling-directory shape for unrelated
# purposes (e.g. capnproto's `c++/ekam-provider/c++header -> ../src`), so this is not a
# hypothetical construct. Fixed by matching candidates by NAME only (no `-type` filter, so a
# symlink is listed just like any other entry), then resolving each candidate's real type with a
# `[ -d ]`/`[ -f ]` test, which DOES follow symlinks -- catching a symlink-to-directory/file without
# ever making `find` itself follow symlinks during traversal (which would risk escaping the
# `donors_root` boundary through a symlink pointing outside it, e.g. to `/`).
while IFS= read -r -d '' path; do
  if [ -d "$path" ]; then
    violations+=("$path")
  fi
done < <(find "$donors_root" \( -iname ".claude" -o -iname ".codex" -o -iname ".cursor" \) -print0 2>/dev/null)

while IFS= read -r -d '' path; do
  if [ -f "$path" ]; then
    violations+=("$path")
  fi
done < <(find "$donors_root" \( -iname "CLAUDE.md" -o -iname "AGENTS.md" -o -iname ".mcp.json" -o -iname "copilot-instructions.md" \) -print0 2>/dev/null)

# Same symlink-aware leaf check applied here too, though this specific pattern additionally
# requires walking THROUGH `.github/instructions/` to find a nested match -- if `.github` or
# `instructions` itself is a symlinked directory (not just the leaf `*.instructions.md` file), this
# scan still cannot see through it without `find -L`'s full symlink-following traversal, which
# reintroduces the boundary-escape risk above. No donor in this corpus currently uses that shape
# (checked directly); broadening this specific case is deferred until real input demonstrates the
# need, matching this repository's own evidence-before-code discipline.
while IFS= read -r -d '' path; do
  if [ -f "$path" ]; then
    violations+=("$path")
  fi
done < <(find "$donors_root" -path "*/.github/instructions/*.instructions.md" -print0 2>/dev/null)

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
echo "  a CLAUDE.md/AGENTS.md/.mcp.json/copilot-instructions.md/*.instructions.md file -> append the suffix .donor-untrusted"
exit 1
