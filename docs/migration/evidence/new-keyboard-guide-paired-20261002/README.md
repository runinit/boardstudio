# New keyboard and setup guide: paired browser evidence

This packet records fresh React oracle runs and paired Dioxus release
candidate runs. It records route/state observations and keeps remaining joins
and parity gates explicit; it does not claim full-guide acceptance.

## Fresh React oracle

- Pinned React source: `5a472a9426e6e38993361da402cd4ec730feb369`, live at
  `http://127.0.0.1:5173/` (root verified PID 32534, source working directory,
  and matching TS bundle).
- Fresh named isolated session: `new17-react-pinned-5173-c8a853a97c72`.
- localStorage was empty before interaction. No existing project data was
  cleared or overwritten; the test project remains in this isolated profile.
- Project ID: `84ea8fb0-3f4b-497c-8501-4f6812f46c65`; Main board ID:
  `d2ae3a6c-e59c-4998-934b-baf2fbb38436`.
- The landing screen shows both `New project` and `New keyboard / Start with
  guided setup`. The Dioxus candidate likewise has `New project` and `Create
  new keyboard`; both are source-backed.
- Blank-project defaults are `Untitled keyboard`, one `Main board` at 1.6 mm,
  an empty part-envelope outline (margin 4, add operation, default outline
  settings), PLA material (3 mm), and no parts, matrices, or case bodies. The
  physical setup chooser starts unresolved. See `react-fresh-landing.*` and
  `react-fresh-00-project.*`.
- The name was set to `New17 Fresh Oracle`; transient and whitespace-padded
  renames followed by Undo restored the last accepted value. See
  `react-fresh-08` through `react-fresh-12`.
- Project → Layout → Wiring → Case → Review kept the guide open and selected
  the corresponding workspace. Finish closed the guide; Project → Setup guide
  reopened at saved Review. Previous-step navigation returned Review → Case →
  Wiring → Layout → Project. See `react-fresh-02` through `06` and `18` through
  `21`.
- At 640×900, the Case guide was visible with Objects open and Case settings
  closed. `Open case settings` hid Objects and revealed Case settings. The
  post-route activeElement stayed BODY; panel visibility is the reliable
  observation. See `react-fresh-07-compact-case-settings.*`.
- Add key matrix opens the real Matrix Setup form with Rows focused while the
  saved guide remains open at Layout. Cancel restores the guide and focuses
  its `Setup guide` heading. See `react-fresh-22` and `react-fresh-23`.
- Settled physical changes advanced revision 5→6 for Split, 6→7 for
  Reversible (right half flipped), and 7→8 for Wired. Reload preserved the
  active project, accepted name, revision 8, physical state, and open Project
  guide stage. See `react-fresh-14` through `17`.

### Superseded React captures

The earlier unprefixed `react-*` packet came from a 5175 URL that had no live
listener and may have shown a cached or unknown build. It is retained as
historical evidence, not the canonical oracle. Its separate project
(`16904bd7-baf5-472e-8cf8-908954e55f3f`) remains intact. A stale-base alert in
that session was not reproduced by the fresh settled Split, Reversible, and
Wired actions; treat it as an observation, not proof of a persistent defect.

## Initial integrated candidate (34722)

- Candidate source: `4bbafae0c1d12b765e27b4262d094401c8e8a65a`, served at
  `http://127.0.0.1:34722/boardstudio/`.
- Named isolated session: `new17-dioxus-candidate-34722-c8a853a97c72`.
- Project `75923945-958d-406d-9e84-9e438d5bf59d`; Main board
  `f494db0c-94d5-434a-bb24-a19861cb9231`.
- New, name/Undo/trim, settled physical setup (revisions 4–6), all five stage
  routes, saved Review reopen, reverse navigation, and reload were verified.
  See `dioxus-fresh-00` through `21`.
- This candidate exposed three defects: compact Case hid the open guide;
  Layout lacked Add key matrix; Case repeated the readiness sentence. They
  were corrected in the later integrated candidate below. Its compact
  pre-fix captures remain for review.

## Integrated compact-guide and MatrixSetup retest (34723)

- Candidate source: `d425a2dfb56ac2d5576da076bed11272437f1ea2`, served at
  `http://127.0.0.1:34723/boardstudio/` with release assets verified by root.
- Fresh named isolated session:
  `new17-dioxus-34723-final-c8a853a97c72`.
- Project `3ae9b7d4-77fc-4783-abb9-5cc63dc4d10b`; Main board
  `50eb9949-f37b-472f-9f21-ad1698454290`.
- Layout now exposes the real `Add key matrix` owner. Opening the form hides
  the guide while preserving `open:true,currentStep:layout`. Cancel restores
  Layout. Creating a 4×4 MX solder matrix closes the form, restores the guide,
  selects Matrix 1 in the Inspector, and saves revision 2. See
  `dioxus-final-02` through `05`.
- At 640×900, Case keeps its guide visible once with Objects open. `Open case
  settings` closes Objects and reveals Inspector, matching React. The guide
  hides while the inspector takes the compact workspace, with saved guide
  state still `open:true,currentStep:case`. See `dioxus-final-06` and `07`.
- One joined fix remains: after Matrix Setup Cancel, React focuses its `Setup
  guide` heading while Dioxus leaves focus on BODY. This production-composition
  focus repair is pending. React/Dioxus evidence is in `react-fresh-22`/`23`
  and `dioxus-final-03`/`04`.
- The Dioxus Wiring stage still lacks React's `Choose controller` action; only
  `Review controller & wiring` is present. Controller placement remains a
  capability join and is not claimed complete.
- After creating the test matrix, Case preview displayed an Ergogen conversion
  error for `${KIPRJMOD}/models/boardstudio/kiswitch/SW_Cherry_MX_PCB.stp`.
  See `dioxus-final-06-compact-case.json`; investigate this separately from
  guide composition before using the test project to assess Case readiness.
- Do not claim full guide acceptance until Matrix Setup Cancel focus and the
  controller-choice action join are resolved and the paired checks are
  repeated.

## Artifacts

`react-fresh-*`, `dioxus-fresh-*`, and `dioxus-final-*` files are screenshots,
accessibility snapshots, localStorage state, and read-only document summaries
from the named isolated sessions. The older unprefixed `react-*` files remain
as historical material and are excluded from fresh React claims.
