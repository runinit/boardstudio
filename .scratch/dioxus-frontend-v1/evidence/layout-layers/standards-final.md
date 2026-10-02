## Standards disposition

**Resolved**

- The retained generator boundary is documented in `docs/architecture.md:412–449`, and the F3a ticket links to it. It records ownership, input/output, resource cleanup and stale publication, failures/retry, synchronous UI-thread execution and cost, cache bounds, offline packaging, and retirement criteria.
- Commit `7268628d` factors the part/member-definition/pitch sizing rule into a shared private resolver. Commit `f44a3d1b` selects the matrix definition only for cells with no visible member. This matches the React fallback at `app/src/ui/CanvasObjects.tsx:31`: member override → member definition keycap → pitch when a member exists; matrix definition keycap → pitch when no member exists. The suspected parity issue is resolved; no remaining standards or smell findings from this review.

**Verification disposition**

- Supplied browser evidence covers all five layer switches, shared Footprints state, unchanged revision/camera, cap edit and Undo, root/subpath generated graphics, 27×18 override, back-side reflection, edge hit target, both Sofle boards, and compact 44px controls with Escape/Close focus restoration. Axe reports zero violations in dark, light, and compact captures.
- Screen-reader verification remains unavailable; axe reports SVG label contrast as incomplete. Carry that limitation forward under `CONSTRAINTS.md:156–160`, which requires manual keyboard and relevant assistive-technology checks. No introduced axe violation was observed.
