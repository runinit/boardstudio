# 02: Edit matrix keycap profiles and socket settings

**Parent:** F6C.2 — Board, matrix, and per-key keycap controls.

**What to build:** The Keycaps Inspector lets a user edit profile, first profile row, wall thickness, and socket/inherited socket for each active-board matrix. Matrix settings remain separate from per-key overrides and feed the existing physical projection and keycap fit resolver.

**Blocked by:** F6C.1's board-scoped Keycaps projection and the existing accepted-document edit/history path. This ticket can run independently of issue 01; F6C.2 and all F6 parent joins remain open.

**Status:** ready-for-agent

- [ ] Each active-board matrix has a contextual settings group with TypeScript-equivalent matrix identity/name and Profile control.
- [ ] Profile choices include no generated keycap and the existing supported profiles: Cherry, OEM, DCS, DSA, SA, Hi-Pro, G20, and Choc.
- [ ] Expanded dimensions/socket controls expose first profile row, wall thickness, and inherited or explicit socket, with TypeScript-equivalent bounds, steps, labels, and interaction boundary.
- [ ] Socket choices remain the existing inherited-from-switch-profile, MX, Choc v1, Choc v2, and Alps options.
- [ ] Changes use existing matrix-keycap edit authority, target stable matrix identity on the active board, and do not overwrite per-key overrides.
- [ ] Reuse the Editor-owned scoped edit admission/outcome lifecycle: capture and revalidate accepted token, revision, full scope, active-board and matrix membership; settle exact outcomes before suppressing hidden Keycaps UI; add no panel-local edit owner.
- [ ] Browser evidence maps React select/number `onChange` behavior to Dioxus native events by exercising select, typing and spinner increments, and records accepted revision/history timing instead of assuming the handler names are equivalent.
- [ ] Physical appearance and fit findings update after accepted edits; pending/failure feedback and stale-result rejection remain correct.
- [ ] Empty matrix state explains standalone switches use their per-key override; no absent setting is materialized merely by viewing the editor.
- [ ] Undo, Redo, save, reload/reopen, and selection/board changes match the reference on the same fixture.
- [ ] Paired browser evidence compares the same TypeScript and Dioxus journey and records build/fixture/project/board/matrix provenance.
- [ ] Required independent Standards/Spec review and RF handoff are recorded without changing parent task status or shared graph.
