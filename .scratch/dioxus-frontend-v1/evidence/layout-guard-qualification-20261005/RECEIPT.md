# Locked Layout position guard

Candidate: frontend-layout-attachment-20261005, source e12b6ab0, http://127.0.0.1:34822/. Loaded script boardstudio-web-dxh20d9fd77d91a53a.js. Reference: pinned 5a472a9 at http://127.0.0.1:5175/. Desktop only.

Both applications publicly imported the same validated input.boardstudio; input.json pins its hash and original fixture. Only project identity/name differ from the prior locked-U1 starting fixture; all other archive data was compared. Selected main/U1 in the Layout canvas.

Candidate Inspector shows Locked and disables X mm and Y mm at 273.20 / -15.00. Reference shows Locked but permits typing into those drafts. Typing 274.20 into reference X and blurring left the selected component at X273.20; reload restored X273.20. Therefore the reference accepted document did not move; its editable draft is not evidence of an admitted edit. Candidate disables the action earlier. Raw DOM observations are in locked-position.json.

This proves the locked-position branch only. Driven fields and remaining draft boundaries are still unqualified; F3.5-C02 stays implemented. No product source changed, no mobile or parent acceptance claim.

## Driven position continuation on eac47f42

Current Case journey Sofle fixture, Right board: created the same Offset relationship from right-keys-SW1 to MH5, offset[-20,-40]. Both resolve MH5 to[192.77,-111.548]. Candidate disables X/Y, as the explicit F3.5-C02 requirement specifies, and preserves that disabled state after reload/reselect. Portable r20 archive retains the exact relationship.

Reference allows direct X193.77 and **updates the relationship offset to[-19,-40]** in accepted r21; this is a real supported reference edit, not a discarded draft. The earlier source comment that relationships overwrite typed values does not describe this reference path. Candidate retains explicit Offset X/Y controls as its editing route while making direct driven X/Y unavailable per the migration criterion. Reference Undo restores[-20,-40]. Do not claim identical direct-field behavior or no reference edit. driven-position.json and both portable archives retain these facts.

Candidate public observations are in ../case-generation-export-20261005/layout-keyboard-observations.json. This adds the driven-field presentation/persistence branch; parent acceptance and remaining draft semantics still require review.
