# F6C.4 contextual finding navigation — paired receipt (2026-10-03)

This receipt pins one Keycaps finding-to-editor route in the existing fit workflow. It finds no missing visible control or navigation defect in this route and does not accept F6C.4 or its parent joins.

## Run

- Reference: React `http://127.0.0.1:5173/`, pinned source `5a472a9426e6e38993361da402cd4ec730feb369`.
- Candidate: Dioxus `http://127.0.0.1:34761/`, source `2107f980e9b45c341bff10548c7156b28de0e388`, provenance `2d218936e847ba8f4d8680e5ecc4b3f5a05c3ce88c85a2152e2397cd84020ac9` (root-reported).
- Both: 1280×577; imported `/home/chris/.local/share/boardstudio/retained-tmp/20261002/keycaps-findings-f6c4-c6.boardstudio`, SHA-256 `9027125846d2878176f127ec39b60c9ab605a0eb2a4b9019e10dbba846fe87a6`.
- At Keycaps entry, both expose matrix profiles, key search/selection, fit findings and the keycap STEP action. The fixture's `left-keys-SW1 · switch mx` group has three target actions; the shared Layout footer reports 29 layout findings.
- In each app, activated the first `Select affected geometry` action in the SW1 group. Both switched to Layout, selected `left-keys-SW1`, showed its Properties inspector, and exposed position X `9.18` mm / Y `-78.05` mm. The canvas highlighted the same SW1-area geometry. There was no visible route, selection, or inspector mismatch in this action.

## Captures

The retained captures are outside the repository so the browser profile and fixture state remain intact:

- React entry: `/home/chris/.local/share/boardstudio/retained-tmp/20261003/keycaps-gap-react-entry.png` (SHA-256 `b74519dc5baeb7a47b0636c9feee81beefacfd61e0e420a7a4cd5af005c28477`)
- Dioxus entry: `/home/chris/.local/share/boardstudio/retained-tmp/20261003/keycaps-gap-dioxus-entry.png` (SHA-256 `7152ba9ac1a2bb8219504a74211141a8f291cc6253efb933f2d31932450dfb08`)
- React after navigation: `/home/chris/.local/share/boardstudio/retained-tmp/20261003/keycaps-gap-react-after-finding.png` (SHA-256 `033b752016a73eb676eca9041ce4fe9d5d25160c1df6d348c1507a7f3cce8c27`)
- Dioxus after navigation: `/home/chris/.local/share/boardstudio/retained-tmp/20261003/keycaps-gap-dioxus-after-finding.png` (SHA-256 `43b8668845e8b8d686bb575f1f7106f578701413d879a7d11de76cf675ddceba`)

## F6C.4 criterion reconciliation

| Criterion | Current source/evidence | Remaining qualification |
|---|---|---|
| Resolve accepted keycap specs/findings and keep request identity scoped | `web/src/presentation/keycaps_fit.rs` owns accepted/stale fit state; the fit action carries its source to navigation. Existing source review covers identity admission and stale targets. | This paired run does not induce a failed or superseded resolve. The public stale/failure behavior remains open. |
| Show conservative swept-envelope language and case-check scope | `KeycapsFitInspector` explicitly names conservative full-switch-travel checks and states that case walls/solids are included only for a current Case preview. The paired Keycaps entry and finding list are visible in the capture. | This fixture/action does not prove a current-case body/feature warning or the stale-case-revision message against a changed Case preview. |
| Finding activation reaches an affected key | Paired SW1 action above lands on the same selected key and Layout Properties pane in both apps. | Only this key target is exercised. Exact case-body/feature and non-key geometry focus is not proven by this route. |
| Inherited Keymap legend changes refresh fit state | The existing accepted fit-state source includes the revision-scoped resolution result; the Keymap 3D receipt demonstrates generated Keymap caps and its inherited SW1 legend. | A paired edit of a binding that changes an inherited legend followed by current fit-result refresh is not established by those receipts. |
| Invalid/unsupported profiles and individual setting inputs produce their resolver findings | F6C.2 receipts cover a rejected wall setting and ordinary profile/legend persistence. Existing source renders resolver-provided findings without taking over validation authority. | The paired journey does not enumerate invalid profile/socket/row/dimension/color/legend/roof states or unsupported inputs. No additional permutation run was done. |

No production source change was justified by this paired route. Keep F6C.4 open for the listed current-case, inherited-binding refresh, stale/failure, and non-key geometry qualifications plus its `INT.2` join. Do not infer full parent acceptance from this receipt.

RF handoff: no new refactoring takeaway observed in the Keycaps entry/finding-navigation source and browser scope reviewed for this receipt.
