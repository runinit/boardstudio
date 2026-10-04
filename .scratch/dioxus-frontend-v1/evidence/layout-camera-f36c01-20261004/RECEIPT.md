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

## Combined candidate Space-pan GREEN retest

The combined candidate `frontend-keyboard-focus-20261004` at `34805`
(source `916a40549444e7ac73c144e422f632a129fd2eaa`) was checked on the same
saved Sofle archive (SHA-256 unchanged). From an unfocused canvas (`BODY` was
active), a held Space key followed by a `90 × 40 px` pointer drag moved the
viewBox from `-57.8859 -26.9120 206.0809 142.409` to
`-83.4684 -38.2820 206.0809 142.409` (camera delta `−25.5825, −11.3700 mm`).
After Space key-up, the same drag left the viewBox unchanged. With the Board
name text input focused, Space key-down was not prevented and the same canvas
drag also left the viewBox unchanged. The earlier pointer-zoom and Fit-board /
Fit-selection measurements and captures above remain the reused evidence; this
retake did not repeat those actions.

## F3.6-C02 paired placement / mode-switch journey

On 2026-10-04, the same saved layered Sofle archive (SHA-256
`5b17071a819e4cfa28531913685e8fedf84cbdd361cefccb3b16befd6c0776df`) was
opened in pinned TypeScript source `5a472a9426e6e38993361da402cd4ec730feb369`
at `5175` and frozen Dioxus source `916a40549444e7ac73c144e422f632a129fd2eaa`
at `34805`. Both started with the Left PCB selected; Zoom in set a non-default
120% camera. Before switching, the TypeScript viewBox was
`-13.452689378757526 -3.912407314629263 172.42238643954576 118.67416666666666`;
Dioxus was
`-13.116602794411165 -3.8403955422488423 171.7340735196274 118.67416666666666`.

The battery connector placement tool was armed in each app (visible prompt:
“Place battery connector jst ph 2 · Click or Enter to place · Esc cancels”).
After switching 2D → 3D, that prompt was absent in both. Returning 3D → 2D
restored the selected Left PCB and the exact pre-switch camera in each app:
TypeScript returned to
`-13.452689378757526 -3.912407314629263 172.42238643954576 118.67416666666666`;
Dioxus returned to
`-13.116602794411165 -3.8403955422488423 171.7340735196274 118.67416666666666`.
Neither app retained the placement tool after returning. This paired journey
found no F3.6-C02 functional mismatch.
