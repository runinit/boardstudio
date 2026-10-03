# Saved-keyboard search and delete implementation checks

Source: React baseline `5a472a9426e6e38993361da402cd4ec730feb369`; isolated Dioxus worktree `project-library-search-delete-20261003`, based on `7ae521b66af22c6c812cd85cf9bf38e7fb0360c6`.

## Browser regressions

The final mounted Library test module passed **8/8** on 2026-10-03. The exact output is `library-mounted-final.log`. It covered the existing name/reopen regressions plus noncurrent deletion/history preservation, current-project no-record fallback, retryable failed replacement, raw stored-name ordering, and cancel/focus behavior.

An earlier redundant module rerun was interrupted before Chrome startup while other browser runners were active. Its partial output remains in `final-combined-rerun-interrupted.log`; the later complete 8/8 run is authoritative.

The ordering regression uses a blank stored name and an `Alpha` candidate. React sorts the raw stored `name`, so the blank record becomes current even though its displayed label falls back to “Untitled keyboard.” The failed-replacement test verifies the original current project returns to `Lifecycle::Ready`, remains saved, and can be retried successfully after replacement persistence fails.

## Static checks

- `cargo fmt --check --manifest-path web/Cargo.toml` — passed after the final changes.
- `git diff --check` — passed after the final changes.
- `cargo clippy --manifest-path web/Cargo.toml --target wasm32-unknown-unknown --no-default-features --features page --all-targets -- -D warnings` — passed on the final source.

The checks used the retained wasm target at `/home/chris/.local/share/boardstudio/worktrees/project-menu-name-19-20261002/web/target`; no release build or public root/subpath browser acceptance was performed in this slice. F2.1/F2.2 and tranche acceptance remain open.
