# Browser action trail (exploratory, isolated)

Environment: React `http://127.0.0.1:5175/`, Dioxus `http://127.0.0.1:34687/`; 1280×577; browser sessions `case-parity-react-2318ea1a1376` and `case-parity-dioxus-2318ea1a1376`. React opened public REVIUNG41; Dioxus opened the public saved REVIUNG41 copy. The documents were not hash-compared.

| Sequence | Session | Public action | Observation / evidence |
|---|---|---|---|
| 1 | React | Enter REVIUNG41, open Case | 85 component board; separate Case assembly hierarchy and PCB group. `react-case.png`, `react-case-snapshot.json` |
| 2 | React | Expand Main case assembly; configure mechanical stack | Bodies listed separately; mechanical fields in right Inspector. `react-case-tree-expanded.txt`, `react-mechanical-open.png/.json` |
| 3 | React | Select Plate, then a PCB component from tree | Inspector context changes to Plate thickness / Part appearance, then PCB properties. `react-case-body-selected.*`, `react-component-selected.*` |
| 4 | Dioxus | Open saved REVIUNG41 copy, select Case | Generic board/layout tree remains; Inspector Case bodies region is empty-state. `dioxus-case.png`, `dioxus-case-snapshot.json` |
| 5 | Dioxus | Select `right keys Independent` in tree | Layout group selected while Case workspace is active. `dioxus-case-tree-selection.txt` |
| 6 | Dioxus | Open Case settings; click Generate case; wait for completion | Controls are in the central panel disclosure. Exact Case CAD meshes appear. `dioxus-case-settings-open.png`, `dioxus-case-after-generation-wait.*`, `dioxus-case-generated-ready.*` |
| 7 | Dioxus | Expand Case layers and colors; select generated `plate`; open View controls | Per-generated-layer select/visibility/color/reset controls exist; Inspector did not change to a body editor. Camera/render/section controls visible. `dioxus-case-layers-open.*`, `dioxus-generated-layer-selected.*`, `dioxus-view-controls-open.png` |
| 8 | Dioxus | Open Case settings and inspect mechanical stack | Construction method/mount style/dimensions/clearance/switch-profile and resolved stack rows present. `dioxus-settings-expanded.txt/.png` |
| 9 | Dioxus | Click Disable mechanical stack; wait until state saves | Inspector changes from “0 authored … remain saved” to “Case stack” with `+ New case body`; profile revision 4. `dioxus-stack-disabled.*` |
| 10 | Dioxus | Click + New case body; wait for “Case body saved.” | Inspector shows Plate body type, thickness 3, clearance 0.5, Z offset 0, Mounting, Gasket channel; profile revision 5. `dioxus-authored-body-added.*` |
| 11 | Dioxus | Click + Add mount; wait for save | Mount 1 exposes Hole/Boss, X/Y, hole diameter, remove; profile revision 6. `dioxus-authored-mount-added.*` |

No page errors were returned by agent-browser `errors` for either session. A read-only request to candidate `/provenance.json` returned HTTP 404; the audit therefore pins source to the integration commit and file digests rather than asserting served-manifest identity.
