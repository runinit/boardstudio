# F3.6-C01: Layout camera and Space-pan check

Date: 2026-10-04. This is one bounded Sofle journey for F3.6-C01. The frozen
Dioxus candidate is `frontend-context-gap-20261004` at `34804`, source
`58145c178156ee0a88ccaadc33dcae194c0f1639`; the pinned TypeScript reference is
source `5a472a9426e6e38993361da402cd4ec730feb369` at `5175`. Both sessions
imported the same saved layered Sofle archive, SHA-256
`5b17071a819e4cfa28531913685e8fedf84cbdd361cefccb3b16befd6c0776df`.

## Journey

- **Pointer-centered zoom:** a trusted `deltaY=-400` wheel input at client
  `(650, 300)` changed candidate viewBox width `206.0809 → 138.1402` and
  reference width `206.9069 → 138.6938`. The world point under the pointer
  stayed stable within `0.000004 mm` in each app.
- **Space-pan regression:** from the initial unfocused canvas, Space-down then
  a `90 × 40 px` canvas drag moved the reference camera but left Dioxus
  unchanged. The source gap was that TS tracks Space on `window`, while the
  Dioxus listener was attached only to the focused SVG. After a normal blank
  canvas click focused both SVGs, the same drag moved both by the expected
  amount (candidate `−31.18, −13.86 mm`; reference `−31.49, −14.00 mm`). The
  source repair adds a Layout-scoped window listener with key-up cleanup, and
  ignores Space in text inputs. The frozen `34804` browser build predates that
  source repair; no rebuilt public UI claim is made here.
- **Fit selection:** selected `thumbs · Key 4.1` (`left-thumbs-SW4`, column
  splay `−23°`) and used Fit selection. Both views framed the rotated key
  similarly (candidate `245%`, reference `244%`). Captures:
  [Dioxus](dioxus-fit-rotated-thumb.png),
  [TypeScript](typescript-fit-rotated-thumb.png).
- **Fit board with visible outline:** both Fit board actions framed the full
  generated outline and board at `82%`. Captures:
  [Dioxus](dioxus-fit-board-outline.png),
  [TypeScript](typescript-fit-board-outline.png).

## Focused regression

The source regression exercises the installed window listener with an unfocused
Space keydown, confirms pan is enabled and page scrolling is prevented, checks
keyup cleanup, and rejects activation outside Layout. The targeted headless
Chrome test passed: `1 passed, 219 filtered` using
`wasm-pack test --headless --chrome --mode no-install web --no-default-features
--features page --bin boardstudio-web --
layout_space_pan_uses_global_space_key_without_a_focus_precondition`.
`rustfmt --edition 2024 --check` and `git diff --check` passed for the two
changed Rust sources. No package was built and no tracker record was changed.
