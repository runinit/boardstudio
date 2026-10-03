# Issue 07 React generator-settings reference

**Status:** reference journey pinned; Dioxus implementation and paired acceptance are still open.

- React URL: `http://localhost:5173/`
- Reference source: `5a472a9426e6e38993361da402cd4ec730feb369`
- Browser profile/session: `parts-frontier-936481b4b15c` (agent-owned profile)
- Project: `Sofle v2` demo in this profile
- Selected definition: `MX switch`, ID `ergogen:ceoloide/switch_mx`
- Input fixture: retained packaged Ergogen definition and schema; no external file.
- Screenshot after Apply and before Undo: `/home/chris/.local/share/boardstudio/reviews/parts-generator-issue07-20261003/react-generator-applied.png`

The Parts Inspector exposed `side`, `reversible`, `hotswap`, `solder`, `include_keycap`, `Keycap Width`, `Keycap Height`, and `Apply generator settings`. Keycap Width changed from 18 mm to 20 mm; Apply accepted the candidate; Undo restored 18 mm. This is the bounded React oracle for a typed draft → generated preview → one accepted edit/history action. A separate pinned selection of `utility keepout zone` also exposed its retained generator fields and the same Apply action.

Issue 06 model attachment is deferred. On the imported THQWGD001C definition selected in this same served journey, the mounted inspector showed the selected-definition summary and `Place component`, but no static-model attach/alignment controls. The static model editor exists in the inspected React source, but the served journey did not mount it for that fixture, and the Parts surface has no corresponding Dioxus model-viewer owner. This ticket therefore does not invent model controls or a viewer.

RF note: reuse the existing Parts preview/source adapter and normal accepted edit path under RF-001/RF-002/RF-009; no new refactoring takeaway observed in this reference-only pass.
