# Compact Case CSS: independent Spec review

**Source clear for the bounded layout repair; fresh public regression remains open.** Reviewed final actual `web/assets/m1.css` SHA-256 `ed3fcce8677e261167f247acf5304205f309cca62b593444d934bd089a0da009` against `9ef5bc5cad09ab3064711b45fc348b4cc09a74d4`. I did not author this patch.

The retained mechanical-settings QA records 924px document height at a 390×844 viewport; the earlier fixed workspace height/minimum could overflow the compact shell. The new rules establish a 100svh Case shell with shrinkable Editor/body sizing, reserve normal flex space for compact panel navigation and footer/status, and put the Case workspace in a single bounded grid row. Removing the arbitrary workspace height and 390px minimum addresses that source cause.

All changes are scoped to compact width and the presence of the Case panel. Desktop layout and other workspaces retain their prior rules. Objects/Inspector occupy the same bounded row as overlays, use existing configured widths capped at 100%, and retain the existing compact-closed display rule and component visibility/inert behavior. Their contents scroll independently with overscroll containment; selecting a drawer cannot add another full-height row below the canvas. Existing Case settings/view disclosures, 44px compact inputs, canvas minimum, themes and event/camera/domain owners are untouched.

No material source-level Spec mismatch found. `git diff --check` passed. No Cargo, source edits or browser probes performed here, and temporary injected-CSS measurements are not production acceptance.

Final both-open correction also clears at source: only when Objects and Inspect are both compact-open, the body gets two shrinkable equal columns, one for each panel; the workspace spans beneath both. Neither interactive panel obscures the other. Single-panel preferred widths, existing closed/inert rules and independent scroll containment are preserved. Public testing must include keyboard access to both narrow columns and closing either panel without losing viewport bounds.

Fresh source-matched browser evidence must still verify document height/viewport intersection at 390×844 and 760-wide, open/closed Objects and Inspect drawers, settings/long findings scrolling, footer/navigation reachability, disclosure keyboard focus, and workspace switching. Short-height/zoomed layouts deserve an explicit check because the existing 160px canvas minimum remains. Broader panel threshold/resizing and Case/viewer joins are not closed by this patch.
