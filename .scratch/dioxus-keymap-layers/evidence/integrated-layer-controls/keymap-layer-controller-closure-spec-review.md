# Keymap layer controller — closure Spec review

**Both remaining branches are corrected; no remaining material Spec source findings. Clear for root-owned mount and verification.** Reviewed clean worker `/home/chris/.local/share/boardstudio/worktrees/frontend-keymap-layers-20261002` at `a8c9524ba6084aca1e777bdb76b6c3ff0fd6bc94`, exact delta from34af9174 against the corrected private contract and prior reviews. No edits, Cargo or browser runs.

Real Rename failures now retain a bounded admission accepted-name key, captured from the accepted map (or virtual Base). Failure relevance compares the current accepted target name with that key. Thus unrelated snapshot-token changes retain the failed draft's error, while an accepted rename changes the keyed input and expires the old error. No full-map copy or duplicated Core mutation was introduced.

Synthesized Completed-result mismatch now uses its own exact acknowledgement token plus `!request.is_applied_to(map)`. It no longer passes through rejection-specific relevance. The completed Add→later Rename case therefore retains the discrepancy at the observed acknowledgement snapshot and expires it when that token changes. Current editor/Scope/generation/target checks still surround both branches.

The earlier fixes remain unchanged: requested layer IDs survive failed Add/removal for fallback and Undo restoration; fresh resolved-layer admission permits fallback/custom-first-ID rename; captured generation and full Scope/token/revision guard stale callbacks; single-flight tracking is registered before submit; Completed waits through transient busy states; actual terminal outcomes clear tracking; Arc-backed accepted snapshots and small operation intents bound tracking cost. Panel remains unchanged, with blur-only raw names, maxlength32, first-index protection and keyed drafts.

New pure tests characterize admission-name relevance and exact-token synthesized mismatch, alongside prior fallback tests. These tests were inspected, not executed. They do not prove mounted outcome ordering, actual persistence, queued-edit recovery, keyboard or browser behavior. Root still must complete actual compiler/native checks, mount review and fresh public add/rename/remove/fallback/Undo/Redo/invalid-retry/reload tests. F3.1/shared acceptance and parent joins remain open. No new RF architecture conclusion.

SHA256:
- controller `2bf2eba706fd9f243bc436e28a06d94443468521088b97c71dd631afc58034a8`
- unchanged panel `4c0b9d4c150ab78a0a2b0a128edc43cda44ecbba6da192b8efd96177d1c02c76`
- governing contract `3dd9ba6fd2a8bb16654a50fc41ec2077d29d5817970ea5b7e0b711a580846b94`
