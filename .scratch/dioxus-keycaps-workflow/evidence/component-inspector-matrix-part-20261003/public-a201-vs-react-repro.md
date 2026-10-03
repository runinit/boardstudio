# Keycaps finding navigation repro: explicit matrix Part Inspector

## Identity

- React oracle: `http://127.0.0.1:5173/`, repository HEAD `5a472a9426e6e38993361da402cd4ec730feb369`.
- Dioxus candidate: `http://127.0.0.1:34740/`, build source `a201a96a76c0d908580793e36e4c7d315155fbb3`.
- Dioxus package provenance: `/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001/web/target/builds/frontend-candidate-batch-20261003/provenance.json`, SHA-256 `a94e2c6989ea98664f31e5dd2f57d62a853b40fcdce348c701f0c80d40eafc7f`.
- Fixture: `/home/chris/.local/share/boardstudio/retained-tmp/20261002/keycaps-findings-f6c4-c6.boardstudio`, SHA-256 `9027125846d2878176f127ec39b60c9ab605a0eb2a4b9019e10dbba846fe87a6`.
- Both browser sessions used a 1280×577 viewport and separately imported the unchanged fixture. The React KeycapPanel and Workbench blobs equal those at the pinned React HEAD.

## Journey and observed behavior

In each app, open Keycaps, locate the `left-keys-SW1` finding group, and choose its first `Select affected geometry` action (`left-keys-SW1 and left-keys-SW2: keycap clearance -18.00 mm (required 0.50 mm)`).

React switches to Layout with the toolbar set to `Select: Part`, retains `left-keys-SW1` as the selected part, and renders the component Inspector with Properties and Relations tabs. The accepted marker contour spans the two-key collision. Dioxus switches to Layout and retains the part identifier, but the toolbar remains `Select: Key` and the Inspector falls back to key position controls; the component Inspector does not mount. Thus the inspected identifier alone does not establish matching selection semantics.

The same-viewport screenshots also show different fit scales: React status reads 139%, Dioxus reads 100%. Their red dashed target extents are visibly different in screen coordinates. This receipt preserves that observation for post-fix comparison; it does not claim camera-fit parity from the old candidate.

## Captures

- React: `/home/chris/.local/share/boardstudio/retained-tmp/20261003/keycaps-react-c6-after-part-finding.png`, SHA-256 `033b752016a73eb676eca9041ce4fe9d5d25160c1df6d348c1507a7f3cce8c27`.
- Dioxus a201 candidate: `/home/chris/.local/share/boardstudio/retained-tmp/20261003/keycaps-a201-c6-after-part-finding.png`, SHA-256 `de7259646238f6d3ce1ab500085d1a0e80c6788632bd5fb3475ece5cc0859ffd`.
- Both captures are 1280×577 PNGs.

This is a historical paired red repro against candidate a201. Re-run on the next candidate containing the Inspector projection fix before making current browser-acceptance claims. The fixture contains no authored Case bodies, so it provides no Case body/layer navigation evidence.
