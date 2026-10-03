# F2.4 Project shell and Appearance parity receipt

**Candidate:** public `http://127.0.0.1:34763/` and `/boardstudio/`, frozen source `99ec041a2895e5ab23880be25501beb9360487db`, provenance `d3c03d6db107ab7a55753b5798d99444f7d0843540ef96530c00d9f44296fe08`.

**Package evidence:** root reports strict page all-target check PASS in 17.97s; 1,371 input hashes, 145 assets, zero route mismatches; 8 fresh and 22 inherited checks; package 95.62s. No Cargo/tests were run by this author.

**Paired setup:** pinned React `5a472a9426e6e38993361da402cd4ec730feb369` at `http://127.0.0.1:5173/` versus Dioxus candidate above. Both used 1280×940 and the same immutable Sofle archive, SHA-256 `5b17071a819e4cfa28531913685e8fedf84cbdd361cefccb3b16befd6c0776df`. Own browser sessions: `f24-react-bde2e5973e19` and `f24-dioxus-bde2e5973e19`.

**Measured topbar after the CSS correction:**

| Region | React (x, width, height) | Dioxus (x, width, height) |
|---|---:|---:|
| Project | 16, 106.66, 44 | 16, 108.13, 44 |
| Save status | 138.66, 32, 44 | 140.13, 32, 44 |
| Workflow navigation | 186.66, 977.92, 43 | 188.13, 976.88, 42 |
| Export | 1180.58, 83.42, 44 | 1181, 83, 44 |

The Project trigger is content-sized rather than reserving 260px; Project and Export have transparent borders and no visible rounded boxes. Dioxus no longer renders a visible “Saved” label. The measured horizontal offset after Project is at most 1.47px in this fixture.

**Changed journey:** both pages opened Project → Workspace settings → Appearance. The select was named “Color theme” and exposed System, Light, Dark. In each page Light and Dark changed the persisted `boardstudio:v2:theme` preference and resolved root theme; System was restored and remained `system` after reload. Back to project menu returned to the Project view. Escape closed the menu; Dioxus focus returned to its Project summary (`role=button`, accessible name Project), and React focus returned to its Project trigger. Both reloaded with Sofle v2 and Layout selected. No archive contents or project editing controls were changed.

**Visuals:**
- `react-final.png`, `dioxus-final.png`
- `react-appearance.png`, `dioxus-appearance.png`

**Scope:** this proves the bounded desktop shell/menu/Appearance change on this candidate and fixture. Responsive/compact behavior, all keyboard interactions, other project states, remaining F2.4 criteria, and parent criteria remain open.
