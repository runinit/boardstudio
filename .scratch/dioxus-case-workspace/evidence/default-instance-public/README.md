# Physical instance default public verification

Candidate build/source: `ed242c02`, production site `http://127.0.0.1:34665/` and `/boardstudio/`.
React reference: existing `http://127.0.0.1:5173/`.

Fixture: exact served Reviung archive SHA-256 `672d5f581e65bf74b9a5df336f734167bba7642865fa7a0dfa12a96347d64a8f`, imported through each app's public file-import UI into fresh disk-backed task profiles (`default-instance-candidate`, `default-instance-reference`). Read-only IndexedDB ProjectDocs are preserved as `candidate-initial-doc.json`, `reference-initial-doc.json`.

At the settled initial load, the complete saved ProjectDocs are exactly equal to each other and the source archive: id `edbec892-9952-4b6b-a7ba-cb6b0a514ac4`, name `REVIUNG41`, rev3, 85 parts, no authored case bodies. Canonical mechanical is null. The one physical instance `main` owns populated saved mechanical/shared-construction data.

Candidate defaults to physical instance `main`; its public selector exposes only `main` (not Canonical). Case shows the generated assembly (bottom thickness 3 and Generate case) and no “+ New case body”. React independently shows the generated `Main case assembly` and physical assembly settings. These baseline results match. Candidate and React screenshots and full initial accessibility snapshots are retained.

No saved document was modified. This packet does not yet cover multiboard instance preference persistence, fallback navigation, same-ID reopen, no-instance canonical route, or pending canvas gestures; these require suitable public fixtures/scenarios.

## Sofle board/instance fallback

Paired public imports of the exact Sofle archive hash 0e1e06beabfce1c5a2d6bfc472c85ed281435ff2382174b32f9a2bd3d6d58899 start on Left PCB with candidate current instance button "left half"; React has Left case assembly selected. Candidate's public Board selector to Right PCB updates current instance to "right half"; React public selection of the Right case assembly shows the corresponding scope. Switching both back selects left again. Candidate selector exposes the matching current instance button only; React's case tree exposes both instances. This fixture has two instances on two separate boards, not multiple instances mapped to the same board.

Initial and post-switch saved docs remain exact same project id m1-sofle-v2-copy, revision3, no case bodies; browser/session selection did not change ProjectDoc or revision. Readonly records and snapshots are in this directory.

## Same-ID reload

On the paired Reviung profiles, browser reload reopened the same saved ProjectDoc ID edbec892-9952-4b6b-a7ba-cb6b0a514ac4, revision3. Candidate returned to Layout and resolved current instance main; there is no candidate public close-project/list route from the loaded editor in this view, so this is reload/auto-reopen evidence, not a distinct Close→Open project-menu flow.
