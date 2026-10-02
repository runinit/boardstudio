# Standards review: startup restore and physical-instance selection

- Base: `37cc0580fc9a3e7ab569cad1ff1bbc47de1ad7b0`
- Candidate: `8b00777fd499aeafe660ad0cb7f11eef97d7bbb3`
- Reviewed production paths: `web/src/runtime.rs`, `web/src/presentation.rs`
- Exact production diff SHA-256 (`git diff --binary BASE...CANDIDATE -- web/src/runtime.rs web/src/presentation.rs`): `403aff19ae3be655873a9ae1074689b9065393858bee68bdf39a87bf6b329f52`

## Findings

No findings in the reviewed source diff.

The change keeps restoration in the browser runtime and presentation. It reads the existing prefix-scoped active-project preference through `BrowserStore::active_project_id`, then uses the existing `open_saved` path and its monotonically increasing open sequence so a later manual open/import supersedes a late startup read. Empty preferences leave the library available; preference-read, missing-project, and load errors are surfaced through the existing live status. The diff does not change storage schema, `ProjectDoc`, session API, exported member visibility, or preference write behavior.

The new selector is derived from the accepted document's instances filtered to the active board. It presents an explicit canonical-board option and submits the existing `Event::Navigate` with the existing board/instance scope. The application session already validates the mapping and cancels scoped gesture, generation, and export work on navigation. The native select has both a visible label and explicit accessible name. The patch leaves the prior `Rc<Vec<_>>` Inspector and keyboard-list allocation change intact.

This is a source-only Standards review. Provider checks/build and public green browser QA were still pending when reviewed; this report does not claim those gates passed. The independent Spec review is tracked separately.
