# Case Escape diagnosis — current result: not reproduced; no fix justified

Candidate `http://127.0.0.1:34663/`, published source `dc81237c`. Working HEAD at inspection was `49b792eb11d94b8535f00fac46005f04c1e12c1a`; `git diff dc81237c -- web/src/presentation/{case_bodies.rs,panels.rs} web/src/presentation.rs` was empty. No source edits or Cargo runs.

## Oracle and loop

Approved `/tmp/frontend-run/case-private-contract.md:103` explicitly requires Escape to restore the accepted field value, despite pinned React CaseNumber lacking Escape handling. For an accepted Z=0, the correct field value is `0`; preserving draft `1.5` is not the candidate oracle. The exploratory verifier report of blank/blur is retained in `.scratch/dioxus-frontend-v1/evidence/case-keymap-current/public/case-drafts/RESULTS.md` and is not erased by these passing controls.

Own disk profile `/var/tmp/frontend-run/profiles/case-escape-astra`, session `case-escape-astra`. Public import: `web/target/builds/frontend-parts-context-aria-20261002/site-subpath/boardstudio/assets/fixtures/sofle.boardstudio` (SHA256 `0e1e06beabfce1c5a2d6bfc472c85ed281435ff2382174b32f9a2bd3d6d58899`). After landing load: upload `input[type=file]`; Case tab; Board=Right PCB; + New case body; wait for saved/enabled fields. No storage writes, manual model mutations or provider substitution.

Exact red-capable command, run successfully:

```
python3 /tmp/frontend-run/case-escape/repro.py case-escape-astra
```

It publicly fills spinbutton `Z offset mm` with `1.5`, presses `Escape`, asserts DOM input value=`0`, and compares saved revision/full caseBodies before/after with readonly IndexedDB. Current evidence `case-escape/tray-green.json`: value0, focused=true, activeINPUT, valid=true, revision5 unchanged, tray z0 unchanged. Six consecutive checks passed across Plate, Tray, desktop and compact720. No failing application loop currently exists; the diagnosing-bugs workflow therefore does not authorize a speculative fix or source-based root-cause claim. Requested original exact key commands/state from the verifier.

## Source trace, not a defect diagnosis

`case_bodies.rs:575` owns a draft signal; `:742` Escape prevents default, assigns accepted.to_string(), resets dirty/errors/submission state, and does not blur. The controlled value is `value: "{draft}"`. Its keyed editor identity is Scope/body/editor-instance dependent, not draft dependent (`:300–313`, `:385`). Canvas Escape lives on canvas nodes, not a Case input ancestor. Panel Escape returns for pinned mode and checks raw-event default_prevented; this is not evidence of a reproduced bubbling defect.

Installed Dioxus0.7.10 `dioxus-interpreter-js/src/ts/set_attribute.ts:24–34` writes input `node.value` rather than its HTML `value` attribute. Null `getAttribute('value')` is expected and cannot prove the field is empty. Actual `.value` was inspected. No speculative framework workaround is proposed.

## Separate confirmed native validity mismatch

Untouched Thickness3 on Plate, and Thickness3/Wall height14/Wall thickness2 on Tray, have min0.001/step0.1 and native `validity.stepMismatch=true`. Browser message for3: “The two nearest valid values are 2.901 and 3.001.” Clearance0.5/min0 and Z0/no min are valid. Readonly captured `case-escape/native-step-validity.json`. `case_bodies.rs:684–688,710–712` supplies these attributes; the application commit validator accepts finite positive values independently. Pinned React `InspectorControls.tsx::CaseNumber` has the same min/step combination, so this is an inherited native-input semantics issue, not a newly reproduced migration regression or explanation for Escape. Correcting it needs an explicit separate scope/oracle; no edits made.

Source SHA256:
- case_bodies.rs `c4e881e4359408867ae4e3befa3fc4037a13738bc196a6cd364533ac8480993b`
- panels.rs `56014451de70c5528922c743bc149cd2554ceb5e04fb76602c903cb783188b8b`
- presentation.rs `8429c7dbfff59f23b675cbe8958c897ecaf5a0af7619dbfd61387a013185e42e`
- installed Dioxus set_attribute.ts `1798e6d22351f80ab207b6f22dd2990d841243b644baada68a042cb676d9c364`

Existing public Undo/Redo green is retained, not rerun or waived. Mismatch/default cards are separate Spec work. No new RF architecture finding.

## Final classification — invalid original driver action; no application regression

The original verifier confirmed its command was `agent-browser press @e521 Escape` (reference `press @e1026 Escape`). `press` accepts only the key at current focus, so the selector was incorrectly supplied as the key argument. The preceding fill already focused the field; the valid command is `agent-browser press Escape`. The reported blank/blur comparison is therefore invalid evidence for the app's Escape behavior and must not be retained as a reproduced regression. Preserve the historical observation with this correction rather than rewriting it as an app defect. The six correct-command public greens above are the valid cancellation result. No source fix is needed or applied. The separately confirmed inherited min/step validity issue remains distinct.
