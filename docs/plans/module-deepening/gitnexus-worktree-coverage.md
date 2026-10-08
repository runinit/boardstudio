# GitNexus coverage for agent worktrees

Use this workflow when a branch introduces symbols that the canonical `boardstudio`
index cannot resolve. Root owns that index on `dev`; each stream may maintain a
separately named index in its own worktree. A comparison using the root index sees
branch diff hunks but cannot establish coverage for definitions absent from `dev`.

## Index the assigned worktree

Run from the assigned worktree root. Set the alias from this table and retain the
ticket's pinned base; do not substitute a moving `dev` ref for that base.

| Stream | Index alias | Pinned base |
| --- | --- | --- |
| PCB 08 | `boardstudio-08-pcb` | `8ce860a09` |
| Parts 08 | `boardstudio-08-parts` | `8ce860a09` |
| Case / Keymap / Keycaps / Library 09 | `boardstudio-09-panels` | `8ce860a09` |
| Root integration (09 / PCB acceptance) | `boardstudio-module-deepening-integration` | `974e367e7` |
| Root integration (Parts / Layout / CAD acceptance) | `boardstudio-module-deepening-integration` | `30e922ac6` |

For example, the PCB agent runs:

```sh
cd /home/chris/01_Projects/ts-boardstudio2/.worktrees/08-pcb-pending-edits
gitnexus_runner=/home/chris/01_Projects/ts-boardstudio2/.gitnexus/run.cjs
gitnexus_alias=boardstudio-08-pcb
gitnexus_base=8ce860a09
node "$gitnexus_runner" analyze "$PWD" --index-only --name "$gitnexus_alias" --force
node "$gitnexus_runner" context PartNetOwner --repo "$gitnexus_alias" --file web/crates/pcb/src/pcb_wiring/controller.rs
```

`--index-only` leaves AGENTS.md, CLAUDE.md and skill files untouched. The named index
lives in this worktree's ignored `.gitnexus/`, and the unique alias preserves root's
`boardstudio` registration. Verify that the context result names this worktree's
current symbol and file. Refresh with the same analyze command after source changes
and before the final comparison; `--force` includes uncommitted additions even when
HEAD has not changed. Indexing does not execute tests or reviews.

The repository's `.gitnexusignore` explicitly includes `web/crates/parts/` because
GitNexus's default artifact exclusions otherwise skip every directory named
`parts`. Keep this override in agent worktrees before analyzing Parts. Verify that
the index includes a Parts definition; a zero-symbol comparison across changed
Parts files is unresolved coverage, even if the reported risk says low.

If root's generated runner is unavailable, bootstrap from the same worktree with:

```sh
pnpm --allow-build=@ladybugdb/core --allow-build=gitnexus --allow-build=tree-sitter dlx gitnexus@latest analyze "$PWD" --index-only --name "$gitnexus_alias" --force
```

## Check impact and the final diff

Before editing a symbol, use its name and file to avoid unrelated candidates:

```sh
node "$gitnexus_runner" impact use_pcb_part_net_edits --repo "$gitnexus_alias" --file web/crates/pcb/src/pcb_wiring/controller.rs --direction upstream
```

Warn before HIGH/CRITICAL changes. An empty or UNKNOWN result remains unresolved;
confirm actual current-source callers with `rg` and record the analyzer limitation.

Before each commit and in the final acceptance report:

```sh
node "$gitnexus_runner" detect-changes --repo "$gitnexus_alias" --scope all
node "$gitnexus_runner" detect-changes --repo "$gitnexus_alias" --scope compare --base-ref "$gitnexus_base"
```

The CLI uses the current worktree as the diff directory. Check exit status and the
reported files against `git diff --name-only`; partial/truncated output is incomplete
coverage. A clean branch can legitimately have zero working-tree changes, so use the
pinned-base comparison for its committed implementation. Preserve both commands,
HEAD, base, index alias, counts, risk and unresolved symbols in the report.

MCP callers use the same alias and explicitly pass the absolute `worktree` to
`detect_changes`; using `repo: boardstudio` would select the canonical graph again.
Root refreshes `boardstudio` after integration and checks the integrated diff.
