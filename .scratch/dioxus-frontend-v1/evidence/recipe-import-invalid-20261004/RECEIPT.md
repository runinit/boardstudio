# F4.6-C02 empty model import — paired public continuation

Candidate frontend-parts-library-repairs-20261004, source3a412978, root34822; read-only reference root5175. Desktop only. Open each retained `Import qualification20261004` saved recipe. Use the supported Import model file chooser to supply this directory's zero-byte `empty.step` fixture.

Candidate reports `Choose a nonempty model file no larger than 32 MiB.` Reference reports `Error: Choose a file smaller than 32 MiB`. Both editors retain exactly Model1/Model2/Model3, with no fourth model. No Save is submitted and no document edit is claimed. This is an empty-file rejection proof, not arbitrary STEP syntax validation or a 32MiB boundary test.

Together with recipe-import-qualification-20261004.md and consolidated-34822-20261004/RECEIPT.md this covers supported import/save/reload/reopen, isolated candidate preview, empty-name/empty-member save rejection, nonpositive model scale rejection, and first-member pose validation on matrix Apply. Additional non-public identifier guards are source-corroborated only; they are not claimed as executed tests. Existing successful branches were not repeated.

## Real model conversion failure and recovery

Without saving, import the nonempty `malformed.step` fixture into each same recipe. Both add Model4 bound to malformed.step. Candidate isolated preview reports `P1: Failed("STEP import failed: IO failed: step: STEPControl_Reader could not read the stream")` while its aggregate status remains `3D preview ready` for the available models. Reference reports `3 / 4 models · 1.6 mm PCB`; it retains the earlier empty-file alert, so that alert is not counted as malformed-STEP feedback.

On the candidate draft preview only, switch 3D -> 2D: the alert/canvas are removed and usable 2D remains. Switch back to 3D: the same explicit STEP conversion failure is reported again. Remove Model4 in each unsaved editor: candidate returns to `3D preview ready` without that alert and reference reaches `3 / 3 models`. This proves a publicly reachable isolated assembly-sample model failure, usable 2D return/retry and recovery after correcting the draft. It does not prove renderer initialization failure, worker cancellation/resource release, or a single-library-definition sample route. No save or accepted-document mutation is submitted.
