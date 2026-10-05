# Routed PCB directory and import lifecycle qualification

Published candidate eac47f42 / frontend-case-export-20261005 on34822; pinned reference5a472a9 on5175. No source change or new application test was needed. Both publicly imported identical input.boardstudio (input.json pins the retained source hash and new id/name). Only project identity/name changed from the prior f57 saved archive. The original board reference starts enabled at X4.2,Y-3.1,Z2.5,rotation15 with one explicit model mapping.

## Actual folder upload and persistence

Both opened Layout > Left PCB > Routed PCB reference > Attach model directory. The tool delivered the actual directory containing six original model payloads extracted from the saved archive, named exactly by the KiCad board model-path basenames. No event was synthesized and no application state was injected. All six selectors changed to explicit model assets. New asset UUIDs differ naturally.

Both Save project copy outputs are revision22. archive-comparison.json proves all six model-path/SHA-256 pairs match and the actual archived blobs hash correctly. Enabled state and the complete nonidentity pose match. Reload in both apps retains all six selected asset IDs and X4.2/Y-3.1/Z2.5/rotation15. observations.json records the actual visible selectors and controls. This closes the earlier automation limitation; it does not reinterpret that older failed directory attempt as passing.

## Remove, import and replace

Remove PCB reference in both changes the originating action to Import KiCad board. Importing Left_PCB.kicad_pcb (136274bytes, pinned hash) installs the reference, exposes six distinct model paths and initializes pose0. Replace KiCad board then accepts Replacement_PCB.kicad_pcb: same supported board structure with two trailing newlines, deliberately distinct valid bytes/name. Both saved replacement archives contain that exact new SHA-256 payload under the accepted BoardReference assetId. replacement-comparison.json records actual accepted metadata and hashes. No 32MiB boundary stress or all invalid variants are claimed; retained owner guards and earlier malformed-input/retry evidence remain bounded separately.

Candidate/reference package paths, inputs, actual outputs and observations are beside this receipt. F5.7-C01/C02 are ready for independent criterion review. F5.7-C03 asynchronous stale/missing-asset composition and C04/F5.8 viewer effects remain open; importing/attaching files does not by itself prove those viewer paths or parent acceptance.
