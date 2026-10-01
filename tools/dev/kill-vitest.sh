#!/usr/bin/env bash
#
# Kill all running vitest processes.
#
# Usage:
#   tools/dev/kill-vitest.sh        # kill all
#   tools/dev/kill-vitest.sh --dry   # preview what would be killed
#

set -euo pipefail

DRY_RUN=false
if [[ "${1:-}" == "--dry" || "${1:-}" == "--dry-run" || "${1:-}" == "-n" ]]; then
  DRY_RUN=true
fi

pids=()
lines=()

while IFS= read -r line; do
  [[ -z "$line" ]] && continue
  read -r _ pid _ <<<"$line"
  pids+=("$pid")
  lines+=("$line")
done < <(ps aux | grep -E '(^|/)(vitest|node .*/vitest)( |$)|/\.bin/vitest|vitest/dist|vitest\.mjs' | grep -v grep || true)

if [[ ${#pids[@]} -eq 0 ]]; then
  echo "No vitest processes found."
  exit 0
fi

echo "Found ${#pids[@]} vitest process(es):"
echo ""

for i in "${!pids[@]}"; do
  line="${lines[$i]}"
  read -r -a fields <<<"$line"
  pid="${fields[1]:-}"
  start="${fields[8]:-}"
  cmd=""
  for word in "${fields[@]:10}"; do cmd+="$word "; done
  cmd=$(echo "$cmd" | sed "s|$HOME/||g")
  printf "  PID %-7s  started %-10s  %s\n" "$pid" "$start" "$cmd"
done

echo ""

if [[ "$DRY_RUN" == true ]]; then
  echo "Dry run — re-run without --dry to kill these processes."
  exit 0
fi

echo "Sending SIGTERM..."
for pid in "${pids[@]}"; do
  kill -TERM "$pid" 2>/dev/null && echo "  signaled $pid" || echo "  $pid already gone"
done

sleep 2

for pid in "${pids[@]}"; do
  if kill -0 "$pid" 2>/dev/null; then
    echo "  $pid still alive, sending SIGKILL..."
    kill -KILL "$pid" 2>/dev/null || true
  fi
done

echo "Done."
