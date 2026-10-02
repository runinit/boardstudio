# Compact layout command pill repair

The CSS repair is frozen in source commit `1cbcb0401de751a0ab268bd9091fc0bb74ed835c`, based on icon implementation `40dc6d99b60d5062bb21e7793d244df576a5922e`. `web/assets/m1.css` SHA256 is `13393fe376f9f02d8cdb32cf05cfd5944acca0b44e4b429e258366c5eb70dc4a`.

I loaded the reviewer-retained production-like HTML fixture (`/home/chris/.local/share/boardstudio/reviews/layout-icons-frozen-css-fixture-20261002.html`, SHA256 `87da02e1730a87e8b2d0e802954461f50df3ce71c2f0f595586b98ce8f6c90d1`) with the candidate stylesheet in named Chrome session `layout-pill-compact-c46656dcb387` (agent-browser 0.38.1). At both widths I set the static fixture summary labels to `Select: Matrix` and `Column` (the latter replaced the normal Transform label); the Snap disclosure was open. These measurements establish overflow behavior for that fixture state, not the real longest Select-label state.

At 375×812, the pill bounds were x=12..293.77 (281.77px), its scrollWidth and clientWidth were both 280px, and each of the four trigger bounds stayed inside the viewport. Leading icons were hidden while text and chevrons remained. The Snap menu bounds were x=13..273 (260px), so it stayed in the viewport.

At 390×844, the pill bounds were x=12..349.77 (337.77px), scrollWidth and clientWidth were both 336px, and all four triggers stayed inside the viewport with icons visible. The Snap menu again measured x=13..273 (260px).

The paired pinned React 375px capture uses the same durable layered Sofle archive as the earlier wide captures. It shows the source compact behavior: all four text triggers and chevrons remain visible while leading icons are hidden. Candidate fixture captures, the reviewer fixture, and the candidate fixture with its stylesheet link are included here. The independent source reviewer tested all five actual Select labels with the unchanged Transform label at 375px, 390px, and 1280px; all fit, with the worst 375px Snap trigger ending at x=320.328 and the worst 390px Snap trigger ending at x=376.328 (pill right edge 378px). This is static production-CSS geometry evidence, not a compiled Dioxus production DOM or public-build acceptance; root's reviewed real-reuse/browser qualification remains separate.

| Capture | SHA256 |
| --- | --- |
| `layout-pill-candidate-375.png` | `f5b5afb55486c1175c338840976503d396a3088760f5e533e953094ed1123da4` |
| `layout-pill-candidate-390.png` | `fcc45c10d9686580501f692e07b0390a3207d4a9ca52aedc019ca8620eb0696e` |
| `layout-pill-react-375.png` | `eeacde9fb47c93795f6152921d9d80c16a68d43a29e3f249e67fdb79af4a58e3` |

Validation after the source change: `python3 scripts/test-build-m1-sources.py`, `python3 scripts/test-build-m1-reuse.py`, and both commands with `python3 -O` passed (1 source test and 17 reuse tests each); `git diff --check` passed. No full or real reuse build was run in this candidate worktree.
