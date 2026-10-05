# Published Layout replay, 113d76fd

Candidate frontend-case-layout-controls-20261005 on34822 loaded actual script dxh5b19dcbbdef29743. Reference exact5a472a9 on5175. Input is case-manufacturing-repair-20261005/input.boardstudio. Candidate manufacturing edits were already savedr31; Layout edits use the same project and Right board.

Mixed selection passes: public canvas keySW1 + Ctrl-click U1 gives two-part Position Inspector. X212.77→213.77 moves SW1 and U1 each+1mm in one edit, oneUndo restores both, Redo restores both edited coordinates. Candidate save and reload/reselectRight retain SW1X213.77 andU1X193.27. Same originating reference edit yields identical positions. Actual candidate-mixed-group-after.boardstudio and candidate-mixed-group-journey.json retained. Same-matrix multiple selection shows Key Inspector with2keys selected.

Failure: actual canvas Ctrl-toggle deselection still leaves the removed key as Inspector owner. Isolated settled clicks SW1,CtrlSW2,CtrlSW2 leave onlySW1selected but headingKey2.1. LocalX1Enter moves deselectedSW2X231.77→232.77 whileSW1remains212.77. Actual revision34 download and candidate-canvas-toggle-failure.json prove the mismatch. Prior reference-toggle-removal.json documents the same unsafe TS behavior; intended correction remains incomplete. RootUndid the test edit. Prior helper-level GREEN does not establish real canvas correctness; author investigates exact route.

Escape routing improves but focus remains incomplete: candidate OutlineGenerated→Editperimeterpoints→Done.press(Escape) returns RightPCBInspector, activeElementBODY. Same reference route returnsRightPCBInspector with activeINPUT aria-labelBoardname. Explicit focus-restoration requirement remains open; author assigned this concrete gap. No wholeF3.5acceptance claimed.
