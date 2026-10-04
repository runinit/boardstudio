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

## F2.1-C02 public search supplement — 2026-10-03

Pinned React reference `http://127.0.0.1:5175/` and current Dioxus candidate `http://127.0.0.1:34786/` were exercised in separate named agent-browser sessions `f21c02-ts` and `f21c02-dioxus`. Full initial Project/landing snapshots showed the saved-keyboard search and all 19 demo cards on each page. Both new browser profiles had `Your keyboards 0`; no project was opened or edited.

The mixed-case query `sOfLe` was accepted in each search field and exposed `Clear search`, but there were no saved cards to match, so this run cannot qualify a populated case-insensitive result. The distinct query `zz-no-such-keyboard-20261003` displayed `No keyboards match your search.` in both clients. Clicking `Clear search` reset the query, removed that status, and restored the initial zero-saved-card list; the 19 independent demo cards remained available. Captures: `c02-reference-no-match-20261003.png` SHA-256 `795740948b79112eec64a168a07f18b759b4723d90e5c090b7d5ee7edd31d1a4`; `c02-candidate-no-match-20261003.png` SHA-256 `5beb261200daf247bd1a67946aa9b992f85512a6568724b1ea8fa48c714e44f3`; `c02-reference-clear-20261003.png` SHA-256 `61afa2a87e916ff102b086477ae51b1969334f05af69dfb766424889ee7b5c63`; `c02-candidate-clear-20261003.png` SHA-256 `95b14b7761666058f42aa517f4f3138cbc7fb43407980f552d6b01a3fd1bd7dc`.

Source inspection at the current candidate confirms the intended trimmed, case-insensitive name match (`matches_project_search`) and clear action (empty query), with no source change warranted by this empty-library journey. The initial blank-profile pass could not qualify matching a populated saved keyboard. A public demo-seeded follow-up in the same packet is below. No task/run JSON, source, build, or tests were changed/run for C02.

### Populated saved-keyboard follow-up

Using the same owned sessions and pinned origins, `Start Sofle v2` created one locally saved demo copy on each app. The candidate Project menu showed `Your keyboards 1` and `Revision 3 · Saved`; the React menu showed `Your keyboards 1` and saved-local status. No project fields were edited. With that single saved Sofle record, the mixed-case `sOfLe` query returned exactly `Open Sofle v2` on both sides and no no-match message. The unique query `zz-no-such-keyboard-20261003` returned zero saved cards and the exact `No keyboards match your search.` message on both. Clearing it restored the one Sofle card and `Your keyboards 1`, with the Demo keyboards gallery still available. No other project was opened or changed.

Populated-state captures: `c02-reference-mixed-match-20261003.png` SHA-256 `dc71002a3d3b4676c6ba6d567ce423e9b665d357c3a711aa54a486700c2eb4ad`; `c02-candidate-mixed-match-20261003.png` SHA-256 `6045d75134b48c8430835c1d528588e957dc730954da0c548640ce1cb333627f`; `c02-reference-no-match-populated-20261003.png` SHA-256 `eba9004dfbef18f48ec083c7b025e1d8a4ee3027a31737f531c02dd40cd5d41b`; `c02-candidate-no-match-populated-20261003.png` SHA-256 `2f28ca28143204808f7754028857571accc76b1c147864af353b955944aa7c02`; `c02-reference-clear-populated-20261003.png` SHA-256 `608ce32599645b978afd85c09e6d850bd3885ba4feb71118ca0ca185bbfdbb3a`; `c02-candidate-clear-populated-20261003.png` SHA-256 `10b6649e1ebff44d655d4a38d7bec2a7bb4bfbd4ff9733b97af39b10b162cf09`.

### F2.1-C01 Project gallery qualification — 2026-10-03

Reused the paired empty-library, demo-card/count/preview and populated saved-card journeys recorded above. On pinned React `http://127.0.0.1:5175/` and current Dioxus `http://127.0.0.1:34786/`, owned sessions `f21c01-ts` and `f21c01-dioxus` each opened one Sofle v2 demo through its public `Start Sofle v2` action. Their Project galleries showed `Your keyboards 1`; the Sofle saved card showed `58 keys · 2 boards` and `Current`. Both retained the demo gallery and its per-card key/board counts. Candidate full DOM inspection found 19 demo-card SVG previews rendered; paired React and candidate gallery previews are visible in the retained C04 gallery captures. Empty state (count 0), saved card, and demo preview evidence is also retained in the C02/C04 captures above.

Additional paired current-marker captures: `c01-reference-gallery-20261003.png` SHA-256 `ad4b9207e9a4361505e3fe23851720382ab2d19a06516afad6462041b6d7be29`; `c01-candidate-current-20261003.png` SHA-256 `284da1913b13503282ee2e2191d80bf13aee34431cae15556570cef4a878c2bd`. The candidate's empty Project gallery is captured in `c01-candidate-empty-20261003.png` SHA-256 `95b14b7761666058f42aa517f4f3138cbc7fb43407980f552d6b01a3fd1bd7dc` (same capture as the C02 empty-profile evidence).

Loading is the only branch not directly captured: the first full candidate landing snapshot was already Ready with zero saved cards, so a user-visible Loading frame was not observable in this normal-speed journey. Current `web/src/presentation/library.rs` source initializes `ListStatus::Loading`, sets it while a list request is pending, and renders `Loading saved keyboards…`; no delay, storage manipulation, or fabricated error was used to force that transient. No exact behavior gap was reproduced, so no source edit was warranted. No task/run JSON, source, build, tests, package or commit changes were made for this packet.

### F2.1-C03 list failure and retry qualification — 2026-10-03

Source comparison: pinned React `http://127.0.0.1:5175/` uses `listProjects()` in `app/src/storage.ts`; `ProjectLibrary.tsx` renders distinct `loading` and `failed` states and increments `retry` from the same `Try again` button. Its existing `ProjectLibrary.test.tsx` test queues a rejected list promise followed by recovered projects. Pinned Dioxus `http://127.0.0.1:34786/` calls `BrowserStore::list_documents()` in `web/src/presentation/library.rs`; the mounted component likewise sets `ListStatus::Loading`, maps `Err` to `Failed`, and increments the same request generation/retry signal from `Try again`.

A paired browser failure journey was not feasible through the public UI: neither client offers a supported action or fixture to make its own saved-project list request fail, and artificially altering browser storage/schema would not be an equivalent public fault path. In particular, malformed-record failure is not paired-equivalent: Dioxus deserializes each returned record and can reject it, whereas React `listProjects()` resolves the raw IndexedDB values without record validation. Existing C01/C02/C04 public success journeys remain the paired browser evidence; this error branch is supplemented by a mounted candidate regression, not claimed as paired public evidence.

Added `saved_project_list_loading_error_and_retry_recover_in_mounted_library` in `web/src/presentation/library.rs`. A wasm32-only private result queue holds a list response long enough for the mounted DOM to expose Loading, then supplies one list error and a recovered saved document on retry. The test confirms distinct status/alert, same-action Retry clearing the alert and showing the recovered saved card, and preservation of the Current marker. Focused headless Chrome check passed: `CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=…/wasm-bindgen-test-runner CHROMEDRIVER=/usr/bin/chromedriver WASM_BINDGEN_TEST_ONLY_WEB=1 cargo test --manifest-path web/Cargo.toml --locked --no-default-features --features page --target wasm32-unknown-unknown --bin boardstudio-web presentation::library::mounted_tests::saved_project_list_loading_error_and_retry_recover_in_mounted_library -- --exact` — 1 passed, 0 failed, 196 filtered. No product behavior gap was reproduced; the queue and response delay are compiled only for wasm tests. No tasks/run JSON, package build, or commit was made for this packet.

### F2.1-C04 damaged-preview fallback supplement — 2026-10-03

The public paired journey above still has no damaged-preview fixture: all 19 candidate demo previews rendered normally, and no public UI route creates an invalid saved preview. The fallback is now qualified by one focused mounted Dioxus regression rather than by storage/network manipulation or a fabricated public failure. `damaged_saved_preview_keeps_open_action_and_healthy_card_available` supplies a controlled saved document whose switch keycap has zero width, causing the production `preview()` path to return its existing `Err(())`. The mounted Project library renders `Preview unavailable` and `Open to check this keyboard`; the saved-card Open button remains enabled and accessible as `Open Damaged preview keyboard`, while a healthy saved card remains visible. This exercises the candidate Library/KeyboardCard render path but is not paired React browser evidence and does not claim the invalid document opens successfully.

Focused headless Chrome check: `CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=…/wasm-bindgen-test-runner CHROMEDRIVER=/usr/bin/chromedriver WASM_BINDGEN_TEST_ONLY_WEB=1 cargo test --manifest-path web/Cargo.toml --locked --no-default-features --features page --target wasm32-unknown-unknown --bin boardstudio-web presentation::library::mounted_tests::damaged_saved_preview_keeps_open_action_and_healthy_card_available -- --exact` — 1 passed, 0 failed, 197 filtered. No product behavior gap or production code change was needed; the fixture uses the existing wasm-test-only list result queue. Existing C04 editable-copy and non-mutating-browse evidence above is reused without repetition.
