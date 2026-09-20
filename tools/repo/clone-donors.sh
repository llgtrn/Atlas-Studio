#!/usr/bin/env bash
# Fetch donor reference repos into Temporary/ at CLOUD bandwidth.
#
# TWO MODES:
#   bash tools/repo/clone-donors.sh <donor> [<donor2> ...]   # STRICT on-demand: clone ONLY the named donor(s)
#   bash tools/repo/clone-donors.sh --all                     # warehouse fill: clone EVERY donor in the manifest
#
# THE CLOUD HANDS MUST USE THE NAMED-DONOR FORM. Knowledge extraction is donor-grounded: a build spec
# that names a donor REQUIRES the real donor source on disk before any code is written. The Cloud clones
# JUST the named donor(s), reads the real files, builds, then `rm -rf Temporary/`. Cloning --all (81 repos,
# multi-GB) is the warehouse bloat that destroyed git before — the Cloud must NEVER use --all.
#
# STRICT: a named donor that is missing/UNRESOLVED in tools/repo/donors.manifest, or whose clone fails, is a
# HARD ERROR (exit != 0) — never a silent skip. A degraded build from a missing donor is exactly what this
# guards against. Source of truth = tools/repo/donors.manifest. Shallow clones (--depth 1) keep it fast.
#
# Why this exists: Temporary/ is gitignored + multi-GB and GitHub rejects files >100MB, so the donors are
# not in the repo. They are public OSS — cloning from source in the cloud is minutes.
set -u

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
DEST="$ROOT/Temporary"
MAN="$ROOT/tools/repo/donors.manifest"
PAR="${DONOR_CLONE_PARALLELISM:-8}"

mkdir -p "$DEST"

# Look up a donor's url from the manifest (col1=dir col2=url). Empty if not present.
url_for() {
  grep -vE '^\s*#' "$MAN" | awk -v d="$1" 'NF>=2 && $1==d {print $2; exit}'
}

# Clone one donor; STRICT=1 (named mode) makes any problem a hard failure (return 1).
clone_one() {
  local dir="$1" url="$2" strict="${3:-0}"
  if [ -z "$url" ] || [ "$url" = "UNRESOLVED" ]; then
    echo "MISSING $dir (no url in tools/repo/donors.manifest)"
    [ "$strict" = "1" ] && return 1 || return 0
  fi
  if [ -d "$DEST/$dir" ] && [ -n "$(ls -A "$DEST/$dir" 2>/dev/null)" ]; then
    echo "HAVE   $dir"; return 0
  fi
  if git clone --depth 1 --quiet "$url" "$DEST/$dir" 2>/dev/null; then
    local sha; sha="$(git -C "$DEST/$dir" rev-parse --short HEAD 2>/dev/null || echo '?')"
    echo "OK     $dir @ $sha  (record this SHA in your DONE reply)"
    return 0
  fi
  echo "FAIL   $dir <- $url (check the URL / branch)"
  [ "$strict" = "1" ] && return 1 || return 0
}

MODE="${1:-}"
if [ -z "$MODE" ]; then
  echo "ERROR: name the donor(s) to clone, or pass --all." >&2
  echo "  cloud on-demand (REQUIRED form):  bash tools/repo/clone-donors.sh <donor> [<donor2> ...]" >&2
  echo "  warehouse fill (NOT for the cloud): bash tools/repo/clone-donors.sh --all" >&2
  echo "  donors are listed in tools/repo/donors.manifest (col 1)." >&2
  exit 2
fi

if [ "$MODE" = "--all" ]; then
  echo "WAREHOUSE FILL: cloning EVERY donor from $MAN into $DEST (parallelism=$PAR)."
  echo "(The Cloud Hands must NOT use --all — clone only the donor a spec names.)"
  grep -vE '^\s*#' "$MAN" | awk 'NF>=2 {print $1, $2}' | while read -r dir url; do
    clone_one "$dir" "$url" 0 &
    while [ "$(jobs -r | wc -l)" -ge "$PAR" ]; do wait -n 2>/dev/null || break; done
  done
  wait
  echo ""
  echo "Done (warehouse). Resolved donors under $DEST/."
  exit 0
fi

# STRICT named-donor mode: every argument must resolve + clone, or the whole run fails.
rc=0
for donor in "$@"; do
  url="$(url_for "$donor")"
  if [ -z "$url" ]; then
    echo "ERROR: donor '$donor' is not in tools/repo/donors.manifest (col 1). Typo? Run with a valid name." >&2
    rc=1; continue
  fi
  clone_one "$donor" "$url" 1 || rc=1
done

if [ "$rc" != "0" ]; then
  echo "" >&2
  echo "STRICT FAIL: at least one named donor could not be cloned. Do NOT build from a summary —" >&2
  echo "reply BLOCKED with the donor name so the Brain resolves the manifest." >&2
  exit 1
fi
echo ""
echo "Donors ready under $DEST/. Read ONLY the spec's source_files, build, then rm -rf Temporary/."
