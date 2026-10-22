#!/usr/bin/env bash
# Create a lane worktree for a rust-ui build ticket (lane protocol, rust-ui-build/map.md step 1).
# Usage: scripts/new-lane.sh <NNN-slug> [--dry-run]
set -euo pipefail

die() { echo "new-lane: $*" >&2; exit 1; }
usage() { echo "usage: $0 <NNN-slug> [--dry-run]" >&2; exit 2; }

dry=0
slug=""
for arg in "$@"; do
  case "$arg" in
    --dry-run) dry=1 ;;
    -h|--help) usage ;;
    -*) die "unknown option: $arg" ;;
    *) [[ -z $slug ]] || die "only one ticket may be given"; slug=$arg ;;
  esac
done
[[ -n $slug ]] || usage
[[ $slug =~ ^[0-9]{3}-[a-z0-9][a-z0-9-]*$ ]] || die "ticket must look like NNN-slug, got: $slug"

base=feat/rust-ui-migration
ticket="docs/plans/rust-ui-build/issues/$slug.md"
lane_branch="lane/$slug"

# Integration checkout = parent of the shared git dir, so this works from any worktree.
common=$(git rev-parse --path-format=absolute --git-common-dir 2>/dev/null) || die "run this inside the BoardStudio repo"
[[ $(basename "$common") == .git ]] || die "not a standard checkout: $common"
integration=$(dirname "$common")
lane="$integration/.worktrees/lane-$slug"
cache="$HOME/.cache/boardstudio-target/$slug"

# Refusals come before any change.
git rev-parse --verify -q "refs/heads/$base" >/dev/null || die "branch $base not found"
git cat-file -e "$base:$ticket" 2>/dev/null || die "ticket $ticket not found on $base"
[[ ! -e $lane ]] || die "lane worktree already exists: $lane"
git rev-parse --verify -q "refs/heads/$lane_branch" >/dev/null && die "branch $lane_branch already exists"

# run: execute a change, or only print it under --dry-run.
run() {
  if (( dry )); then
    printf '[dry-run] %s\n' "$(printf '%q ' "$@")"
  else
    printf '+ %s\n' "$(printf '%q ' "$@")"
    "$@"
  fi
}

# copy_into_lane <relative-path>: real copy from the integration checkout, never overwrites.
copy_into_lane() {
  local rel=$1 src="$integration/$1" dest="$lane/$1"
  if [[ ! -e $src ]]; then
    echo "note: $rel missing in integration checkout, not copied"
  elif [[ -e $dest ]]; then
    echo "note: $rel already in lane, left alone"
  else
    run mkdir -p "$(dirname "$dest")"
    run cp -a "$src" "$dest"
    echo "note: copied $rel"
  fi
}

run git -C "$integration" worktree add "$lane" -b "$lane_branch" "$base"
run mkdir -p "$cache"
run ln -s "$cache" "$lane/target"

# Git-ignored files named in .worktreeinclude (blank lines and # comments skipped).
if [[ -f $integration/.worktreeinclude ]]; then
  while IFS= read -r entry || [[ -n $entry ]]; do
    entry=${entry#"${entry%%[![:space:]]*}"}
    entry=${entry%"${entry##*[![:space:]]}"}
    [[ -z $entry || $entry == \#* ]] && continue
    case "$entry" in /*|*..*) die ".worktreeinclude entry must be a relative path: $entry" ;; esac
    copy_into_lane "$entry"
  done < "$integration/.worktreeinclude"
fi

copy_into_lane cad/.cache
copy_into_lane cad/wasm/pkg

if (( dry )); then
  echo "dry run: nothing was created"
fi
echo "Lane worktree: $lane"
echo "1. cd $lane"
echo "2. claim: set Status: claimed in $ticket, commit on $lane_branch"
echo "3. check: python3 scripts/check.py quick"
echo "4. merge back (local fast-forward only, never origin): git push . HEAD:refs/heads/$base"
