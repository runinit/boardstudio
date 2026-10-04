# Project demo cards and fresh copies — bounded candidate receipt

## Candidate and package

- Candidate: `http://127.0.0.1:34767/boardstudio/`
- Integrated source: `3a080e0e99d0638ddc1b562b3fc5a8ac91f3fefd`
- Build: `frontend-six-stream-parity-recovery-20261003`
- Provenance SHA-256: `96d7bc6289aedaadd8198bb9038c67d24fc659fde847362b24c7504313cbc733`
- Coordinator package evidence: full 22-command check set passed; strict page WASM all-target Clippy passed in 18.35s; 1,372 source inputs and 146 assets per route had zero mismatch; root and `/boardstudio/` returned 200 with COOP/COEP.
- No additional build or test was run by this browser author.

## Paired reference

The reference is React at `http://127.0.0.1:5173/`, source `5a472a9426e6e38993361da402cd4ec730feb369`, using isolated agent-browser profile `parts-demo-fresh-copy-177491532993`.

The bounded cards match the packaged project fixtures: REVIUNG41 shows an SVG layout preview and `41 keys · Single board`; Sofle v2 shows an SVG layout preview and `58 keys · 2 boards`. The React menu exposes a wider 19-card catalogue; this child covers the two existing Dioxus actions only. Starting REVIUNG41, then Sofle v2, then REVIUNG41 again produced three accepted IDs: `6fa08c96-513c-428e-9e2b-d4629e474b31`, `2fe15aa3-0344-49df-921a-9ab44c19f190`, and `d8b8365a-611e-4a80-a170-d3feb07ecf6d`. The menu showed three saved cards, and reopening the first REVIUNG41 card restored its original ID.

## Dioxus candidate journey

The candidate was tested in isolated agent-browser profile `parts-demo-candidate-177491532993`.

- The library landing screen exposes REVIUNG41 and Sofle v2 cards with SVG previews and the matching fixture summaries above.
- The Project menu contains the same two cards and previews. Starting REVIUNG41 from that menu accepted project ID `03dcbad4-c8df-4cf1-a3ce-6c05d2720ac8` (85 parts, one board, revision 3).
- Starting Sofle v2 accepted project ID `a98cebf5-7fe6-4830-93fe-9a16dd0df3d5` (140 parts, two boards, revision 3).
- Starting REVIUNG41 again accepted a distinct project ID `9e29545d-dfbf-4e56-af6d-823332304334` (85 parts, one board, revision 3). A read-only IndexedDB inspection found all three documents present with distinct IDs.
- The Project menu showed `Your keyboards 3` and both REVIUNG41 cards. Opening the saved Sofle v2 card returned to the Sofle workspace and marked that card `Current`; the saved menu remained at three cards.
- No manual archive import was repeated for this changed path. Source mapping at this exact candidate passes `None` from `Runtime::import_file` into the shared archive decoder, and only fixture opens pass `Some(copy_id)`; the decoder rewrites `document.id` only for `Some`. Thus the fixture identity choice does not alter generic archive import identity handling. Existing archive import/reopen evidence is reused from `.scratch/dioxus-frontend-v1/evidence/case-readiness-parity-20261002/RESULTS.md` and `.scratch/dioxus-frontend-tranche-1/evidence/tree-and-parts-4b05d451/ui-verifier/README.md`.
- The paired journey used the healthy packaged fixtures. It did not inject malformed/missing preview JSON to exercise the fallback card state.
- The changed browser journey did not inject demo fetch/decode failure or a superseded completion. Existing `begin_open`/sequence guards remain on the fixture-open path and the new branch still checks sequence before acceptance; those error and race branches are not claimed as freshly qualified here.
- Packaged fixtures read by this candidate: `reviung41.json` SHA-256 `b577dd2009fffbf00489cc8d0f2ccc088861f62a7f30af42470d35c11534bdae`; `sofle.json` SHA-256 `800080b686e449d3e14490e3f176a3c069d0d4d0f50e3e25a94440f6201f64e4`. Their matching archives are `reviung41.boardstudio` SHA-256 `672d5f581e65bf74b9a5df336f734167bba7642865fa7a0dfa12a96347d64a8f` and `sofle.boardstudio` SHA-256 `0e1e06beabfce1c5a2d6bfc472c85ed281435ff2382174b32f9a2bd3d6d58899`.

## Evidence files

- `reference/RESULTS.md` — pinned React journey and screenshot hash.
- `reference/project-menu-demo-cards.png` — React menu screenshot, SHA-256 `6d825fd8cb6e504a0d5ec39738e804b139420070d270c8a3e41bc7662b370f64`.
- `dioxus-library-cards.png` — candidate landing-page cards, SHA-256 `6d72224ff413501a7a6aa8d36eb04b8142d0cf9a3d1b912dc4ec7508fdcfb597`.
- `dioxus-project-menu-cards.png` — candidate menu with both demo cards and saved-card region, SHA-256 `68f0e4337144b973d4f604b9a7e02b99b0b96e09f9c003888e00ff91861bd345`.
- `dioxus-reopened-saved-copy.png` — candidate saved-copy menu with Sofle v2 marked current, SHA-256 `eaf406b5afff6f45f2c84f220a60eab101850c366f3039353192d58cfc0969e0`.

## Scope and limitations

The bounded card/copy child is qualified for the two existing demo actions: fixture-backed cards and summaries, fresh identities on repeated starts, saved listing, and saved-card reopen. Generic archive identity is supported by direct source mapping plus unchanged archive-flow evidence, not by another archive-import click in this changed journey. Preview-load fallback and demo-open failure/supersession injection remain unqualified. This does not close the F2.1 parent, the broader demo families, or other existing Project workflows.

## F2.1-C04 public journey supplement — 2026-10-03

Pinned React reference `http://127.0.0.1:5175/` and Dioxus candidate `http://127.0.0.1:34785/` (source `f6aa4cb0`) were exercised in separate named agent-browser sessions `f21c04-ts` and `f21c04-dioxus`. Each fresh browser began with no saved keyboards. `Start Sofle v2` opened the 58-key/two-board demo in the editor on both clients. Renaming its selected board from `Left PCB` to `F2.1 C04 proof` made the demo an editable copy and saved it locally; the candidate Project menu displayed `Revision 4 · Saved` and one saved Sofle card, while React displayed `Saved locally` and one saved Sofle card. This was a representative edit/autosave, not a separate portable `Save project copy` download.

With `left-U1` selected in the inspector, the Project menu and keyboard gallery were browsed, including a temporary `Sofle` saved-keyboard search that was cleared before returning. Both clients returned to the same `left-U1` selection and the renamed board. The candidate menu still showed `Revision 4 · Saved`; React still showed its saved-local status. Undo and Redo remained available after the browse. No other project was opened and the browse/search itself issued no project edit. Paired captures show the saved card, demo gallery previews and retained inspector selection: `c04-reference-project-menu.png` SHA-256 `1662d82ce434be7b0466f6deac6a23a9106f3e324be48aca42866feed2a6ad16`; `c04-candidate-project-menu.png` SHA-256 `04d4bb03b397b1f1a77c1fe393d084a8ff53cc6d8219ccc2685b0faf927993f6`.

The damaged-preview branch remains unqualified by public journey. All 19 candidate demo cards loaded on this run and each rendered an SVG preview; there was no malformed or unavailable packaged demo fixture accessible through the public UI. I did not alter storage, network responses, or fixture data to synthesize a failure. A public damaged-preview fallback and continued open action therefore remain the precise evidence gap for C04. No source was edited and no tests or build were run by this browser author; the coordinator's combined package build completed successfully during this run.
