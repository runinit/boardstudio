# Keeping the GitNexus index current

GitNexus answers "what calls this" and "what breaks if I change it". Its answers are only as
good as the index, and on this branch the index was stale for days. This page records why,
and the rule that keeps it current without a pile of agents re-indexing at once.

## Why it was stale (investigated 2026-10-10)

- **Nothing re-indexes automatically.** The user-level hook
  (`~/.claude/hooks/gitnexus/gitnexus-hook.cjs`) only *notifies*. After a successful
  `git commit`, `merge`, `rebase`, `cherry-pick` or `pull` it compares `git rev-parse HEAD`
  with the registered index's `lastCommit` and prints "index is stale, run analyze". It never
  runs analyze, deliberately: a full analyze takes minutes, blocks the agent and has
  corrupted the database when a timeout killed it. The reminder goes to whichever agent made
  the commit, and subagents rightly do not act on it.
- **One index per checkout, all named `boardstudio`.** Every worktree has its own ignored
  `.gitnexus/` and its own registry entry, and the registry holds at least ten entries with the
  same name for different paths. `--repo boardstudio` and the MCP default are therefore
  ambiguous, and often hit the main checkout, which does not contain this branch's symbols. That
  is why `impact` kept answering "not found" or UNKNOWN for new code.
- **Only commits count as stale.** The hook looks at HEAD only. Uncommitted work is missed
  unless the index was built with `--force`.
- **Concurrent writers are unsafe.** The database takes a write lock and the MCP server can hold
  it; several agents running analyze at once fight over it.

## The rule

1. **Agents never run analyze.** An implementer or verifier who sees "stale" does not fix it. They
   say "GitNexus index stale" in the Outcome, fall back to `rg` for callers, and treat any
   `detect-changes` result as provisional (as before).
2. **The orchestrating session is the single owner.** It refreshes the index with
   `python3 scripts/gitnexus-refresh.py` at these points only:
   - at the start of a batch, if `--status` says stale;
   - after a batch's full `check.py` passes (before the next batch starts), so the next batch's
     implementers and verifiers see the new code;
   - after a merge or rebase that moves many files.
   Never while an agent is running timing-sensitive checks (the analyze takes CPU).
3. **The script is safe to call twice.** It skips when the index is current, runs one analyze at a
   time for the whole repository family (a lock in the git common dir; a second caller exits
   immediately), names the index after the checkout (`boardstudio-<worktree>`) so it no longer
   collides with `boardstudio`, passes `--index-only` (agent instruction files are untouched) and
   `--force` (uncommitted source is included).
4. **Address the index by its name.** Use `--repo boardstudio-<worktree>` in CLI calls
   (for example `node .gitnexus/run.cjs impact SYMBOL --repo boardstudio-tce-matrix`). The bare
   `boardstudio` may be another checkout.
5. **Check before trusting.** `python3 scripts/gitnexus-refresh.py --status` prints "current" or
   why not. A result from a stale index is a hint, not a clean check.

A refresh of this repository takes about one minute (55 s measured on 2026-10-10, 22k nodes), so
it is cheap enough to do at every batch boundary.

## The Stop hook (orchestrating session only)

`scripts/gitnexus-refresh.py --background` is wired to the **Stop** event of the session that
orchestrates the work, so the index catches up between turns without anyone remembering to. It
is deliberately cautious:

- It acts only when the index is behind **HEAD** (committed staleness). Uncommitted edits never
  trigger it, so it does not re-index on every turn of an editing agent.
- It does nothing while a build, test run or browser is going (`cargo`, `rustc`, `wasm-pack`,
  `chromedriver`, `chrome`, `podman`, `check.py` and the browser runners), so timing runs are
  not disturbed.
- At most one attempt per 15 minutes per checkout (a stamp in the git common dir), one analyze at
  a time for the whole repository family (the lock), detached, `nice 19`, silent, always exit 0.
  Output goes to `.gitnexus/refresh.log`.
- A hook fires only in the session whose settings contain it. Subagents do not run it for their
  own commits, and the rule above still holds: agents never analyze.

The hook lives in the checkout's local, untracked settings (`.claude/` is gitignored), so each
checkout that wants it needs the entry below. It is a guarded one-liner, so it is harmless in a
checkout that lacks the script:

```json
{
  "hooks": {
    "Stop": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "[ -f \"$CLAUDE_PROJECT_DIR/scripts/gitnexus-refresh.py\" ] && python3 \"$CLAUDE_PROJECT_DIR/scripts/gitnexus-refresh.py\" --background; exit 0",
            "timeout": 10,
            "statusMessage": "Checking the GitNexus index"
          }
        ]
      }
    ]
  }
}
```

**Codex.** The same script is wired into Codex's `~/.codex/hooks.json` as a second command in the existing `Stop` group (a backup is
beside it as `hooks.json.bak-gitnexus-refresh-*`). Because that file is global, the command first checks that the session's git checkout
contains `scripts/gitnexus-refresh.py` and does nothing elsewhere:

```sh
root=$(git rev-parse --show-toplevel 2>/dev/null) && [ -f "$root/scripts/gitnexus-refresh.py" ] && python3 "$root/scripts/gitnexus-refresh.py" --background; exit 0
```

It refreshes the index of whichever checkout (main or worktree) the Codex session is in. Re-add it if Codex's hooks are regenerated.

Settings changes take effect in a new session (or after reopening `/hooks`). To stop it, delete the
entry. A hook on **PostToolUse** would instead fire after every agent commit, which is what the rule
avoids.
