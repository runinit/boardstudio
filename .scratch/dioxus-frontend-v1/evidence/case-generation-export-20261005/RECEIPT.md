# Required grouped Case generation and export journey

Starting fixture: `input.boardstudio`, pinned in `input.json`. It copies the existing initial Sofle archive with only project ID/name changed, preserving prior saved qualification projects. Both apps publicly imported these exact bytes, entered Case, added the default Plate through `+ New case body`, and invoked `Configure mechanical stack`.

Pre-fix candidate: frontend-layout-attachment-20261005, e12b6ab0, served34822. Pinned reference: 5a472a9, served5175. Actual scripts are in `baseline-scripts.json`; status/button observations in `pre-fix-baseline.json`. Both reach current geometry at revision5. Candidate lacks the local action; reference enables Export geometry while retaining warnings.

Reference action produced `reference-left-half-mechanical.zip` (2,243,512bytes). `reference-export.json` pins hash, members and STEP signatures. ZIP integrity passes; assembly.step plus four individual part STEP files have ISO-10303-21 headers. assembly.json has revision5 and expected plate/foam/PCB/bottom stack. The nominal PCB reference excludes component solids. Warnings are present, so this is also an observed allowed-warning export baseline.

Pending: new candidate integration/publication, actual candidate local output comparison, required setting edit and stale/current recovery, physical-scope change, and save/reopen. Neither the pre-fix baseline nor mounted event interception completes this journey. Mobile and optional work are excluded.
