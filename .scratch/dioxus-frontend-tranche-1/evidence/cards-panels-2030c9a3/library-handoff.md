# T1-02 saved-library handoff

Author worktree: `/home/chris/.local/share/boardstudio/worktrees/frontend-library-20261002`

- Branch: `codex/frontend-library-20261002`
- Owned implementation commit: `bdb3edfd` (`feat(web): render saved keyboard library cards`)
- Integration tip merged normally: `cb8203fc`
- Current HEAD: `449c718e277bd897a78528c136fc5ff6c88d90e5`
- Worktree is clean.
- Owned source: `web/src/presentation/library.rs`
- Source SHA-256: `8d62664ab47da36575a21835e8e2566ae90c38d904df1a8fad7e4d3cfe365d94`
- Shared CSS remains unapplied. Reviewable patch: `/tmp/frontend-run/library-shared.patch`; SHA-256 `1e7faa2ecc0dc81336b0f2a61fd09b4a3c36a39a8ed8783a3bb0de53b1536d85`. `git apply --check` passes. It copies the React card sizing/tokens/icons and leaves project-menu shell sizing to the coordinator.

The private library renders the accepted snapshot first, deduplicates saved IDs, preserves original nonblank names, orders saved cards with browser `localeCompare`, derives switch-only previews and summaries, and isolates invalid geometry into an openable per-card fallback. It includes current/open semantics, local loading/failure/retry state, prior-list retention, request generations, accepted-ID freshness checking and unmount protection. Existing Reviung41 and Sofle copy buttons and import remain in that order.

Checks passed:

- `rustfmt --edition 2024 --check web/src/presentation/library.rs`
- `cargo fmt --manifest-path web/Cargo.toml --check`
- `git diff --check`
- `cargo check --locked --manifest-path web/Cargo.toml --target wasm32-unknown-unknown --features page,core-worker`
- `cargo clippy --locked --manifest-path web/Cargo.toml --target wasm32-unknown-unknown --features page,core-worker -- -D warnings`
- `git apply --check /tmp/frontend-run/library-shared.patch`

No tests, production build, browser parity, keyboard/focus or axe checks were run by this author; these remain coordinator integration evidence.

T1-02 remains open. Contract review at `.scratch/dioxus-frontend-tranche-1/evidence/contracts-active/library-contract-review.md` confirms two blocking joins: (A) the reproduced post-Session-submit open supersession race, with red trace in `/tmp/frontend-run/open-supersession-reply-result.json`; and (B) atomic `BrowserStore::list_documents` decode failure for one malformed saved record. A narrow read-only per-record BrowserStore addition is proposed in `.scratch/dioxus-frontend-tranche-1/evidence/contracts-active/tolerant-listing-api-proposal.md`, but is explicitly unapproved because it adds a public API across the host-library/page-binary boundary. No weakened interpretation or workaround was included.

No new refactoring takeaway observed.
