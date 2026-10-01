# P2-r1: bounded Dioxus event/canvas lifecycle probe

Status: proposed; execution approval pending. This continues the accepted P2
charter in CAPABILITY-MAP.md, not a competing architecture or migration backlog.

## Exact base, specifications and approval boundary

Base: 7ed7b5ec9a58449615b05bf7e95eb888711d864b on migration/boardstudio-m1.
P1-CORE/P1-CAD and the two findings are complete at this integration revision.
RUN.md states “No next eligible approved task remains” and excludes P2/P3 from
its completed run. The new implement invocation requests continuation, but
names no new task set. Earlier advance approval remains valid for defined work;
this proposal defines P2-r1 for one explicit approval before implementation.

Accepted references, resolved relative to /tmp/boardstudio-migration-m1-20261001:
- CAPABILITY-MAP.md, P2 Dioxus/canvas lifecycle charter.
- SPEC-dioxus-presentation.md, SVG/canvas, disposal, final gesture sample and U4.
- SPEC-rendering.md, existing generated entrypoints and R4 lifecycle contract.
- SPEC-host-platform.md, H5 resource release and replaced/disposed scope guards.
- SPEC-editor-session.md, gesture ownership, cancellation and one-step history.
- CONSTRAINTS.md and accepted ADR0003; original Wayfinder decisions stay settled.

## One task: P2-LIFECYCLE

Category: isolated feasibility investigation preserving characterized behavior.
Coordinator implements serially; no parallel writers. After approval create a
new task-owned worktree and branch from the exact base, verify actual cwd/HEAD/
status, and record approval, spec hashes, fixture identities and task ownership
before edits. Scope only .scratch/prototypes/p2-lifecycle/ and coordinator-owned
architecture/status/evidence documents. Existing worktrees remain untouched.

1. Verify selected Dioxus0.7.10/CLI0.7.10 and existing binding pins against
   version-matched Context7 or official tagged/installed source for mount/DOM
   conversion, events, effects and teardown. Keep three-d0.19.0 and the existing
   renderer. Record unavailable APIs as blockers, not an implicit version switch.
   Complete the execution preflight and fresh read-only Luna/medium smoke with
   independently observable routing before assigning any writes.
2. Implement one SVG gesture surface over a copied existing fixture through the
   public CoreEngine/accepted worker seam, and one canvas using the existing
   generated renderer API. The headless probe coordinator owns gesture identity
   and policy; host modules own DOM/capture/listeners/observers/frames; renderer
   owns GPU/camera; Dioxus signals contain presentation snapshots only. Separate
   modules in one isolated package reflect lifetimes, without prescribing new
   production crates. No shared contracts/provider visibility is widened.
3. Exercise mount, reactive rerun and repeated unmount/remount; pointer capture,
   final pointerup sample, unrelated-pointer rejection, Escape/pointercancel and
   keyboard/focus; one committed drag plus one-step Undo/Redo through the real
   public engine. Exercise resize/DPR, a visibly painted renderer frame, disposal
   and late-result rejection. Compare the corresponding reference actions using
   the same immutable fixture and retain screenshots/resource/frame evidence.
   Test context loss as explicit failure; do not assume recovery.

TDD seams proposed for this task: public engine request/reply/history behavior,
external gesture intentions/effects and renderer/browser lifetime outcomes.
Define their RED expectations before replacing behavior; use actual public
providers, not stubs or mirrors of private implementation. This approval covers
those bounded seams and scenarios, not a production editor-session port.

## Verification and acceptance

- Focused native probe tests and affected existing interaction/render-client
  regressions during development; regular native/WASM type checks and strict
  fmt/Clippy. Preserve existing thresholds and assertions.
- Full probe suite and affected core/renderer native suites; full app Vitest suite;
  fresh eligible core/renderer WASM and reference frontend build; contracts,
  boundary and repository checks. The existing CAD module can be reused only
  with exact source/provider/artifact identity evidence; no CAD changes planned.
- Real Chromium via agent-browser at root and /boardstudio/ with pinned release
  builds. Assert final-sample transaction/history, capture/cancel/focus behavior,
  actual visible canvas frames, DPR/resize and teardown resource counters. Paired
  affected reference interaction/renderer browser tests and screenshots must
  pass. Apply existing affected accessibility/resource checks; blocked or missing
  evidence remains explicit. Do not substitute a callback for a painted frame,
  or claim full performance/whole-editor parity from this fixture.
- Document the experimental browser boundary in architecture.md, including
  owner/data/errors/cancellation/cleanup/copy costs and named retirement criteria.
- Fresh independent Luna/high Standards and Spec reviews inspect the exact
  candidate against the then-current integration base. Maximum three child
  threads, only coordinator delegates, no child delegation or provider switch.
  Root alone integrates locally, validates that revision, and updates task status.
  Serialize shared-interface/manifests/locks/state changes. Preserve all evidence
  and worktrees, use at most two diagnosed repairs, and block failed gates.

Record exact executable commands after inspecting the real toolchain/scripts;
base command set is the existing core/renderer Cargo test/fmt/Clippy commands,
pnpm run build:core, pnpm run build:renderer, pnpm --dir app test,
pnpm --dir app build, pnpm run check:contracts, pnpm run test:contracts,
pnpm run check:boundaries, pnpm run check:repo, and scoped existing interaction/
render-client/browser tests. Prototype release commands use the accepted locked
Dioxus root/subpath flags. Dependency or unavailable-check repairs retain their
commands, exit codes, logs and attempts; no silent omissions or gate replacement.

## Exclusions and stopping

No P3 persistence/offline/service-worker work, workspace port, production session
implementation, renderer/backend/schema/API change, CAD-u64 repair, new framework
version, production adoption/cutover, main merge, push or deploy. No UI redesign.
Successful P2 accepts only this lifecycle feasibility proof. If an existing public
renderer API cannot support a required lifecycle gate, preserve the evidence and
stop before proposing visibility changes. On completion/blocker leave a resumable
handoff with task status, next eligibility, base/candidate/reviewed/integration
commits, owned worktrees, commands/exits/log locations and unresolved approvals.
