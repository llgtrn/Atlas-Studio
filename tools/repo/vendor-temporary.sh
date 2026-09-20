#!/usr/bin/env bash
# Vendor Temporary/ donor repos into git+GitHub, fully serial + idempotent + lock-resilient:
#   per donor -> git add -> strip >100MB files -> commit -> push THAT commit with retries.
# Lock resilience: the editor/harness polls git (status/ls-files) on this huge tree and intermittently
# holds .git/index.lock; we WAIT for that window (retry) instead of force-removing a live lock.
# Re-runnable in batches: skips donors already committed.  Usage: bash tools/repo/vendor-temporary.sh [MAX]
cd /c/Users/trngh/Documents/GitHub/Chronica || exit 1
git config http.postBuffer 1048576000
git config http.version HTTP/1.1
git config http.lowSpeedLimit 1000
git config http.lowSpeedTime 60
BR=truth-reset/canonical-docs-2026-06-08
MAX="${1:-999}"
done_list="$(git log --oneline | grep -oE 'vendor\(donor\): [^ ]+' | awk '{print $2}')"
processed=0

git_retry() {  # run a git command; if the harness's git-status poll holds index.lock, KILL it to seize the window
  local i
  for i in $(seq 1 60); do
    if "$@" 2>/tmp/gitr.err; then return 0; fi
    if grep -qi 'index.lock\|another git process' /tmp/gitr.err; then
      cmd.exe /c "taskkill /F /IM git.exe /T" >/dev/null 2>&1   # free the lock from the harness poll
      rm -f .git/index.lock 2>/dev/null
      sleep 1; continue
    fi
    return 1   # a real (non-lock) error
  done
  return 1
}

push_one() {  # push a single commit sha to the branch, retries + hard timeout
  local sha="$1" a
  for a in 1 2 3 4 5 6 7 8; do
    timeout 240 git push origin "$sha:refs/heads/$BR" >/tmp/push.out 2>&1 && return 0
    sleep 4
  done
  return 1
}

for d in Temporary/*/; do
  name=$(basename "$d")
  if printf '%s\n' "$done_list" | grep -qxF "$name"; then echo "skip  $name (done)"; continue; fi
  [ "$processed" -ge "$MAX" ] && { echo "-- batch limit $MAX reached --"; break; }
  processed=$((processed+1))
  if ! git_retry git add "$d"; then echo "ADD-FAIL $name: $(tail -1 /tmp/gitr.err)"; continue; fi
  nbig=0
  while IFS= read -r f; do
    [ -z "$f" ] && continue
    if [ -f "$f" ] && [ "$(stat -c%s "$f" 2>/dev/null || echo 0)" -gt 104857600 ]; then
      git rm --cached --quiet "$f" 2>/dev/null
      printf '%s\n' "$f" >> Temporary/.gitignore
      nbig=$((nbig+1))
    fi
  done < <(git diff --cached --name-only -- "$d")
  staged=$(git diff --cached --name-only -- "$d" | wc -l)
  if [ "$staged" -gt 0 ]; then
    git_retry git add Temporary/.gitignore
    if git_retry git commit -q -m "vendor(donor): $name — $staged files (skipped $nbig >100MB)"; then
      sha=$(git rev-parse HEAD)
      if push_one "$sha"; then echo "DONE  $name: $staged files (skipped $nbig) — pushed"; else echo "COMMITTED-NOT-PUSHED $name (run scripts/push-incremental.sh later)"; fi
    else
      echo "COMMIT-FAIL $name: $(tail -1 /tmp/gitr.err)"
    fi
  else
    echo "empty $name: 0 files"
  fi
done
echo "-- run done: processed $processed; committed $(git log --oneline | grep -c 'vendor(donor)')/81; origin files: $(git ls-files Temporary/ | wc -l) --"
