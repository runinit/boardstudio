# Tree hierarchy repair diagnosis

Integration source: 96811f92 (tree source same as frozen candidate8ea71a84). Candidate http://127.0.0.1:34653/boardstudio/, pinned React reference http://127.0.0.1:5173/ at5a472a. Scope: objects.rs + objects/tree.rs only, no Outline/Bridge activation (T1-11), public API or CSS changes. Root owns builds.

## Input equivalence prerequisite

Verifier publicly imported the same archive into fresh isolated browser sessions tree-react-import and tree-dioxus-import:

`web/target/builds/frontend-tree-8ea71a84-20261002/site-root/assets/fixtures/reviung41.boardstudio`

SHA-256: `672d5f581e65bf74b9a5df336f734167bba7642865fa7a0dfa12a96347d64a8f`.

Readonly IndexedDB captures `/tmp/frontend-run/ui-verifier/tree-reference-project-doc.json` and `tree-candidate-project-doc.json` contain metadata plus record. Reviewer independently compared their `record` objects: exact deep equality of every field/array, and same active ID `edbec892-9952-4b6b-a7ba-cb6b0a514ac4`, revision3. The only wrapper difference is database metadata. The accepted saved document contains85 parts,3matrices,3layouts. Right layout owns main/U1 and main/RST.

Earlier separate demo-start screenshots are exploratory and are not treated as paired-input regression proof. Public red oracle pending; no source edits yet.

## Actual red and reference control

Both scripts drive public archive import and rendered disclosure controls; readonly IDB supplies evidence only. They contain no DOM/handler instrumentation.

```sh
python3 /tmp/frontend-run/tree-hierarchy-fix/tree-hierarchy-public-regression.py http://127.0.0.1:34653/boardstudio/ tree-hierarchy-fixer-scoped-red candidate /tmp/frontend-run/tree-hierarchy-fix/scoped-red.json
python3 /tmp/frontend-run/ui-verifier/tree-hierarchy-public-regression.py http://127.0.0.1:5173/ tree-hierarchy-fixer-ref reference /tmp/frontend-run/tree-hierarchy-fix/reference-green.json
```

Candidate exited 1: root collapsed, one direct child `right keys 18 keys`, and incorrect root Components group. Reference exited 0: root expanded, direct Columns and owned U1/RST. The scoped repair oracle excludes Outline/Bridge rows from assertions because ticket T1-11 owns them; it retains raw rows for diagnosis.

Ranked hypotheses shown before implementation: (1) default disclosure state differs; (2) owned matrices incorrectly use a header-emitting path; (3) standalone components are grouped without consulting split ownership policy. Confirmed by retained React Workbench.tsx:514–520 and useWorkbenchTree.ts:256–323. Objects initialized an empty set; append_matrix always emitted a matrix header; Components was emitted for both split and nonsplit layouts.

A second public regression minimizes the unowned-retention finding: copy the same archive and remove only `main/RST` from its owner's layout.partIds. Everything else stays in the archive. Both applications accept exactly equal complete ProjectDoc records for the variant. Expanded Layout in React shows RST; candidate drops it because rendering was conditional on layouts.is_empty().

```sh
python3 /tmp/frontend-run/tree-hierarchy-fix/unowned-public-regression.py http://127.0.0.1:34653/boardstudio/ tree-hierarchy-unowned-red /tmp/frontend-run/tree-hierarchy-fix/unowned-red.json
python3 /tmp/frontend-run/tree-hierarchy-fix/unowned-public-regression.py http://127.0.0.1:5173/ tree-hierarchy-unowned-ref /tmp/frontend-run/tree-hierarchy-fix/unowned-ref.json
```

Candidate exited 1; React exited 0. Variant archive retained at `/tmp/frontend-run/tree-hierarchy-fix/unowned-component.boardstudio`.

## Bounded repair

Only objects.rs and objects/tree.rs: initialize board/Layout/unowned-matrix disclosures from current scope; retain independent toggles; omit owned matrix header and adjust descendant levels; put owned standalone rows under their layout unless split-axis grouping applies; retain unowned rows under Layout even when layouts exist. No selection callback, scope adapter, CSS, API, or Outline/Bridge edits. Owned-file rustfmt and git diff --check passed. Root compiler/build and independent review plus rebuilt-artifact public green remain required.

RF: no new refactoring takeaway observed; existing shared presentation composition finding covers this private hierarchy builder. No framework/domain restructuring proposed.

## Final artifact green

Repair commit `b51353bb` is included unchanged in source `515f390d`, served at http://127.0.0.1:34655/boardstudio/. Both original scoped public regressions passed (exit 0) in fresh fixer-owned sessions:

```sh
python3 /tmp/frontend-run/tree-hierarchy-fix/tree-hierarchy-public-regression.py http://127.0.0.1:34655/boardstudio/ tree-hierarchy-fixer-final candidate /tmp/frontend-run/tree-hierarchy-fix/scoped-green.json
python3 /tmp/frontend-run/tree-hierarchy-fix/unowned-public-regression.py http://127.0.0.1:34655/boardstudio/ tree-hierarchy-unowned-final /tmp/frontend-run/tree-hierarchy-fix/unowned-green.json
```

Initial board is expanded; right layout expands directly to Columns 1–6 and its owned U1/RST; there is no spurious Components group. Unowned RST remains directly under expanded Layout with other owned layouts still present. Fixed variant accepted ProjectDoc remains exactly equal to reference variant record. Independent verifier separately reports both greens in `/tmp/frontend-run/ui-verifier/tree-515f390d-{hierarchy,unowned}.json`.

Two earlier attempted green runs failed before fixture adoption because /tmp tmpfs was full (16GiB,152KiB available), producing IndexedDB transaction failures and no accepted tree. These were environment prerequisite failures, not hierarchy test results. Closing only this fixer's completed test browser sessions freed361MiB; unchanged code/scripts then passed sequentially in fresh sessions. All own repair browser sessions are now closed; captured artifacts remain. No debug instrumentation was introduced. Root owns independent source reviews and affected compiler checks.
