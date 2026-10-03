# Wiring context source handoff

Base7ea6372c includes the current accepted PCB owner and Parts generator packet. This source changes only the board Wiring display and scoped CSS. The source reference is pinned5a472a9426e6e38993361da402cd4ec730feb369 WiringPanel plus main consumer; the actual VIK34750/React screenshot mismatch is retained with PCB15. Controller/plan/admission/actions and selected-part panes remain unchanged. Diagnostics now precede firmware/assignments, controller/topology share Wiring header, pins require a selected controller, and assignments include the count and reference empty guidance.

No new refactoring takeaway observed in this bounded presentation change. Retain RF-001 shared composition and RF-009 source/visible-parity evidence. The missing Add controller action is concrete separate owner work: the existing chooser only admits the open Wiring setup-guide stage, so calling it from an ordinary PCB would be inert. Do not show an inert button or silently open a different guide UI. Existing connection release and general findings remain existing parent frontiers.

Formatting/diff pass. Root will run one combined affected strict page check with the ready Parts/Keymap/Keycaps source and validate the changed board context on the next candidate. No new ordinary-UI tests or isolated compiler.
