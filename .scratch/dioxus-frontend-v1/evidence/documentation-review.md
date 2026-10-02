# Dioxus frontend v1 documentation review

Reviewed integration commit `ea82ef17` read-only: `docs/migration/DIOXUS-FRONTEND-V1.md`, `.scratch/dioxus-frontend-v1/spec.md`, `PLAN.md`, all nine issue files, `docs/migration/dioxus-frontend-v1-run.json`, and the amended source inventory/reference notes. No full tests run and no repository changes made.

## Findings

1. **P1 — Shell nav is still described as seven tabs, and the stale UI label says Design.** The product has six workspace tabs: Layout, PCB, Keymap, Keycaps, Case, Parts; Export is a separate topbar action. The authoritative roadmap and F1 issue mostly distinguish this correctly, but the committed companion reference notes still say “all seven tabs … (Design, …, Export)” (`.scratch/dioxus-frontend-v1/evidence/reference-inventory.md:11`) and the inventory's F1 exit criterion says “seven-tab order” (`evidence/tsx-inventory.json`, F1 row). This can steer implementation toward the exact navigation distinction the user clarified. Update both to “six workspace tabs plus a separate Export action,” and use the displayed React label **Layout** (the internal React mode enum is `Design`). Search for this wording in other artifacts before calling the correction complete.

2. **P2 — Hook inventory includes a test twice.** The `theme_and_orchestration_hooks` list contains `app/src/ui/useWorkbenchNavigation.test.tsx`, while the same TSX is already in `verification_tsx`. `hook_notes` correctly says it is a test, but the reference summary calls these “11 `use*` hook files,” which reads as eleven production hooks. There are ten actual UI hook source files plus this test TSX. Keep the test in verification only or explicitly call the field an inclusive `use*` file scan and report the 10+1 split. This is a small ledger clarity issue, not a missing component.

## Coverage and scope audit

- The manifest exactly matches current React `app/src/**/*.tsx`: 79 unique paths, comprising 63 production entries (each assigned exactly once), 15 test TSX files and one bench harness. I recomputed the 63 production source hashes against `dev` `5a472a9426e6e38993361da402cd4ec730feb369`; all matched.
- All F1–F9 IDs appear in the inventory mapping. `Workbench.tsx` is identified as a shared source for shell, Export and crosscutting behavior; Export has zero primary TSX entries but explicitly points to its Workbench route and `main.tsx` callbacks, accurately reflecting the React composition.
- The 18 stylesheet paths all have notes; the two source font assets and selected visual reference captures are distinguished. Adjacent UI orchestration files (`main`, project/session/export/electrical and UI hooks) are enumerated. The duplicate test hook above is the only accounting ambiguity found.
- The frontend-only boundary is clear and consistent across roadmap/spec/plan/issues/run record: existing Rust services are reused; backend, generator, CAD, format, schema and kernel rewrites are out of scope. Service gaps become explicit dependencies rather than stealth backend work or permanent placeholders.
- Dependencies are directionally sensible: shell→shared/project UI→Layout/Parts; PCB and Keymap/Keycaps/Case follow their stated common UI and hardware dependencies; Export joins supported workspace providers; final qualification depends on all frontend milestones plus applicable carried gates. F1's `M1 implementation` prerequisite is reasonable, and its `implementing` status aligns with active uncommitted changes in `/tmp/boardstudio-rust-v1-shell`.
- M1 screen-reader/performance/material/resource limits are retained as applicable qualification gates, not claimed resolved by UI placeholders. No additional concrete readiness contradiction found.

No findings require changing the frontend-only roadmap itself beyond the two documentation cleanups above.

Live follow-up: the F1 description now says **browser-scoped persistence**, which matches React `useWorkbenchTheme` using `localStorage` key `boardstudio:v2:theme`; no finding remains on theme-preference scope. This wording is already updated in the worktree after commit `ea82ef17`.
