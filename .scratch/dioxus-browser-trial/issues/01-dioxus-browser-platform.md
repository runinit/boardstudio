# Verify the Dioxus browser platform and toolchain

Labels: wayfinder:research
Type: research
Mode: AFK
Status: resolved
Assignee: /root/dioxus_platform_research
Parent: [Dioxus browser trial and Rust application migration](../map.md)
Blocked by: none

## Question

Which version-pinned Dioxus web architecture and toolchain can support a
browser-first Rust application with the existing editor's browser capabilities?

Use official documentation and source, version-matched Context7 and verified
release metadata. Establish the Dioxus/web dependency and CLI versions, Rust
requirements relative to this repository's `rust-toolchain.toml`, static hosting
and non-root base paths, generated browser bindings and any maintained runtime
JavaScript requirements. Investigate Rust access to SVG/pointer/keyboard events,
canvas lifecycle/resize, WebGL, web workers, file/ZIP downloads and IndexedDB,
and offline service-worker/cache deployment. Identify precisely which behaviors
the framework supports directly and which remain Rust browser adapters.

Separate documented capabilities from untested integration assumptions. Do not
claim a runnable application, offline guarantee or performance gain from docs.
Do not implement a new app, alter the toolchain or install global tools. Capture
one cited research artifact on an isolated `research/` branch and link its
revision and local artifact here. The selected trial architecture remains a
human decision in the dependent ticket.

## Comments

- User selected a browser-first trial and permits generated JavaScript bindings
  and development/test scripts in the final Rust application.
- Context7 resolved `/dioxuslabs/dioxus`; available versioned entries include
  `v0.7.10`. Verify a release/version before using it as the recommended pin.
- Research context: branch `research/dioxus-web-20260930`, based on `96dd51d3`,
  in `/tmp/boardstudio-research-dioxus-web-20260930`. The researcher owns only
  `.scratch/dioxus-browser-trial/research/browser-platform.md` on that branch.

## Answer

Resolved 2026-09-30 from `/root/dioxus_platform_research` findings. This resolves
the platform investigation, not the trial architecture or application build.

The [versioned browser platform evidence](../research/browser-platform.md)
identifies Dioxus 0.7.10 as a verified candidate. Its declared Rust floor does
not require changing this repository's pinned toolchain. The CLI is absent on
PATH, and actual dependency resolution/build is still untested. SVG/events,
mounted canvas access and browser bindings offer integration routes; workers,
storage/files, renderer lifetime, static subpaths and offline policy need
Rust application adapters and executable evidence. Generated runtime glue is
consistent with the user's boundary; authored worker/service-worker application
policy cannot silently remain JavaScript. Proposed checks are inputs to the
human acceptance decision, not already chosen requirements.

The parent captured the artifact on `research/dioxus-web-20260930` at
`b1eb69c18213278899c5c394f7244454296bf588` in the standalone repository
`/tmp/boardstudio-research-dioxus-web-capture-20260930`. The
[portable research bundle](../research/browser-platform.bundle) preserves that
commit and branch, requiring the existing baseline `96dd51d3`.
`git bundle verify` passed. SHA-256 of the Markdown artifact is
`d4b63980b4d603fe9c9cdbfb307366b3f8e247261572ff38f59d2c887608c000`.

The original linked worktree could not stage because its index under the main
`.git/worktrees/` was read-only. The original main-repository research ref
therefore remains at its baseline; the captured commit lives in the standalone
clone and bundle. No source checkout, toolchain or production configuration was
changed to overcome that restriction. The Markdown copy here matches the
captured artifact; main production code was not built or changed by this research.
