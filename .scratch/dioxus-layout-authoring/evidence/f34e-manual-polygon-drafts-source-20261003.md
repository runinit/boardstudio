# F3.4e manual geometry source receipt

This bounded source packet compares the pinned React `5a472a9426e6e38993361da402cd4ec730feb369` (`OutlineInspector.tsx`, `OutlineFeatureEditor.tsx`, `useOutlineEditor.tsx`, and `outlineEditing.ts`) with the mounted Dioxus Layout outline route. The source base is `826885159...`; root's compiler repair was joined as `b13983c6` and its fixes are retained in this packet.

The mounted Inspector now provides addition/cutout and Connect points drafts; draft snapping uses the existing Layout grid/geometry snap policy and Alt bypass. Polygon completion follows the existing `CopyOutline.feature` or fixed-version `ReplaceDocument` path. Connection completion requires Generated plus an automatic envelope, associates endpoints with eligible parts using the React 10 mm rule, and updates the accepted envelope through `SetOutline`. No second document/history authority or bridge algorithm was added.

The saved geometry list selects authored features. Polygon edits enter the existing perimeter point editor. Rectangles expose board-space center coordinates, width, height, and corner radius; center changes preserve the existing part-local anchor. Later fixed-version authored features can be removed; the first authored perimeter is protected. Saved connections expose width and point coordinate editing in world space while preserving attached-point local coordinates, and can be removed through the owning envelope edit. Fixed-version feature mutation uses `ReplaceDocument` to update only the active version.

The pinned React gap rows also expose **Show gap**, which stores the selected source gap and calls `fitParts([], [gap])`. The mounted Inspector now exposes the same action; it revalidates the current accepted `OutlineGap`, converts attached points through their current part poses, and submits only a `SetCamera` view update. Keep/unkeep and removal of protected gaps continue through accepted outline settings edits.

**Known limits:** connection control-point insertion/removal and attachment picker/detachment are not included; point keyboard/guide and capture/unmount parity are not claimed. Core validation remains authoritative and existing outcome feedback reports rejected geometry. This child does not close F3.4.

**Checks:** `rustfmt --edition 2024 web/src/presentation/outline_lifecycle.rs` and `git diff --check` passed. No Cargo, browser, or test command was run, per the assigned source-only packet. Root owns the combined check and changed public journey.

**RF:** retain RF-001 (single Editor operation authority), RF-006 (accepted Board/scope and geometry ownership), and RF-009 (fixture/source/provenance evidence). This continuation produced no distinct refactoring finding.

**Acceptance remains open:** combined source check and the changed paired public draft/connection/editor journey remain coordinator-owned; all F3.4 parent criteria and other joins remain unchanged.
