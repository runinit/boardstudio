# New keyboard and setup guide: paired browser evidence

This packet records a fresh React oracle run and a paired Dioxus release
candidate run. It records exact route/state observations and keeps unresolved
joins and parity defects explicit; it is not a full-guide acceptance claim.

## Fresh React oracle

- Pinned React source: `5a472a9426e6e38993361da402cd4ec730feb369`, live at
  `http://127.0.0.1:5173/` (root verified PID 32534, source working directory,
  and matching TS bundle).
- Fresh named isolated session: `new17-react-pinned-5173-c8a853a97c72`.
- Fresh localStorage was empty before interaction. No existing project data was
  cleared or overwritten; the test project remains in this isolated profile.
- Project ID: `84ea8fb0-3f4b-497c-8501-4f6812f46c65`; Main board ID:
  `d2ae3a6c-e59c-4998-934b-baf2fbb38436`.
- At the landing screen, both `New project` and `New keyboard / Start with
  guided setup` appear. The Dioxus candidate likewise offers `New project` and
  `Create new keyboard`, so both entry points are source-backed.
- The blank project uses `Untitled keyboard`, one `Main board` at 1.6 mm, an
  empty part-envelope outline (margin 4, add operation, default outline
  settings), PLA material (3 mm), and no parts, matrices, or case bodies. The
  physical setup chooser starts unresolved. See `react-fresh-landing.*` and
  `react-fresh-00-project.*`.
- The name was set to `New17 Fresh Oracle`; subsequent transient/trimmed-name
  edits and Undo restored the last accepted value. See `react-fresh-08` through
  `react-fresh-12`.
- The five-stage journey ran Project → Layout → Wiring → Case → Review. Guide
  stage and workspace route advanced together. Finish closed the guide, and
  Project → Setup guide reopened at saved Review. See `react-fresh-02` through
  `react-fresh-06`.
- At 640×900, the Case guide was visible with Objects open and Case settings
  closed; `Open case settings` hid Objects and revealed Case settings. The
  reliable observation is panel visibility; focus stayed on `BODY` in the
  post-transition probe. See `react-fresh-07-compact-case-settings.*`.
- Settled physical changes advanced the React project revision 5→6 for Split,
  6→7 for Reversible (right half flipped), then 7→8 for Wired. See
  `react-fresh-14` through `react-fresh-16`.

### Superseded React captures

The earlier unprefixed `react-*` packet came from a 5175 URL that had no live
listener and may have shown a cached or unknown build. It is retained as
historical evidence, but is not the canonical source oracle. A separate old
profile project (`16904bd7-baf5-472e-8cf8-908954e55f3f`) remains intact and is
not used for current claims. The first stale-base alert recorded in RF-006 was
not reproduced in the verified source run: fresh settled Split, Reversible,
and Wired actions succeeded sequentially without an alert. Treat the earlier
in-flight race as an observation, not proof of a persistent React defect.

## Fresh Dioxus candidate

- Candidate source commit: `4bbafae0c1d12b765e27b4262d094401c8e8a65a`.
- Candidate URL: `http://127.0.0.1:34722/boardstudio/`. Root recorded the
  release-build and static-worker provenance before the browser run.
- Fresh named isolated session:
  `new17-dioxus-candidate-34722-c8a853a97c72`.
- Project ID: `75923945-958d-406d-9e84-9e438d5bf59d`; Main board ID:
  `f494db0c-94d5-434a-bb24-a19861cb9231`.
- Create new keyboard opens Project & hardware with `Untitled keyboard`,
  One/Split/Reversible controls, and an empty layout. Enter commits
  `New17 Fresh Oracle`; a transient rename followed by Undo restores the
  accepted name. A whitespace-padded name commits trimmed, and Undo restores
  its previous accepted value. See `dioxus-fresh-00` through `dioxus-fresh-03`
  and `dioxus-fresh-18`/`19`.
- Physical setup changes settled without stale-result errors: Split (revision
  4), Reversible (revision 5), Wired (revision 6). The project identity and
  guide preference remained stable. See `dioxus-fresh-04` through `06`.
- The guide advanced through Layout, Wiring, Case, and Review, and persisted
  each selected stage. Finish closed it; the project menu reopened at saved
  Review. A page reload retained the active project and the last saved
  `open:true,currentStep:layout` route. See `dioxus-fresh-07` through `11`,
  `17`, and `20`/`21`.

## Unresolved parity gates

- At 640×900 on Case, localStorage says `open:true,currentStep:case`, but the
  Dioxus guide is absent from the visible/accessibility UI. The screen shows
  Case assembly and Inspector only, making `Open case settings` unreachable.
  At desktop width the guide returns and the action is available. This is an
  acceptance blocker under repair; see `dioxus-fresh-12` through `14`.
- The Dioxus Layout stage currently exposes `Edit existing objects` and
  navigation but no `Add key matrix`; the real MatrixSetup owner/callback join
  remains pending. This is not accepted as parity.
- Dioxus Case visibly repeats `Optional: configure a case or continue without
  one.`; React displays that sentence once. See `dioxus-fresh-09-case.png`.
- Do not claim full guide acceptance until the compact guide visibility defect
  and MatrixSetup join are resolved and the paired browser checks are repeated.

## Artifacts

`react-fresh-*` and `dioxus-fresh-*` files are canonical screenshots,
accessibility captures, localStorage state, and read-only document summaries
from the named isolated sessions above. The older unprefixed `react-*` files
remain as preserved historical material and are excluded from fresh React
claims.
