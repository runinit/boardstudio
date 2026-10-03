# Shared Layout findings footer and route — candidate 34767

**Candidate:** <http://127.0.0.1:34767/boardstudio/>  
**Integrated source:** `3a080e0e99d0638ddc1b562b3fc5a8ac91f3fefd`  
**Provenance:** `96d7bc6289aedaadd8198bb9038c67d24fc659fde847362b24c7504313cbc733`  
**Package:** Full 22-command verification passed; 1,372 source inputs and 146 assets per route had zero drift or mismatches; root/subpath routes returned 200 with COOP/COEP. Combined strict page WASM all-target Clippy passed.

I used named `agent-browser` sessions `layout-findings-react-20261003` and `layout-findings-author-20261003`, both at 1280×940 with the REVIUNG41 fixture. This is the changed public journey for the shared findings entry and footer alignment; prior unchanged Layout same-board evidence remains in [34765](public-34765.md).

In React 5173 and Dioxus 34767, I selected Keymap and opened `Layout findings: 1`. The findings page showed the single `Keyboard PCB · Outline` group while the Keymap workspace tab stayed selected. Activating `Show outline` then selected Layout and opened Board outline; Dioxus also selected `Outline Generated` in the tree. Thus opening preserves the current workspace, and the supported target action performs the Layout route.

The same-viewport captures show the finding trigger at the right side of the footer with the amber warning icon inline with its label, matching the React placement and treatment:

- React Keymap with findings open: [react-keymap-findings.png](public-34767/react-keymap-findings.png)
- Dioxus Keymap with findings open: [dioxus-keymap-findings.png](public-34767/dioxus-keymap-findings.png)
- Dioxus after Show outline routes to Layout: [dioxus-layout-outline.png](public-34767/dioxus-layout-outline.png)

The fixture exposes one same-board Outline target only; cross-board continuation and Case-body/mechanical target routing are not verified or claimed. This journey does not qualify every non-Export workspace, responsive/theme variants, or full F3.5/F3.7 acceptance. Parent criteria and the canonical task count remain open.

**RF handoff:** No new refactoring finding was observed. Keep RF-015 as the existing source-selection finding; the global register remains owned by the coordinator.
