# 172: Refuse WRL polygons the preview cannot triangulate correctly

Status: wontfix
Type: build
Priority: P1
Blocked by: None (can start immediately)
Related: [170: Combined WRL decoding correction](170-bound-wrl-traversal-work.md); [Research](../../../research/audit-input-boundaries-2026-10-18.md)

## What to build

A WRL model outside the preview's supported convex polygon subset receives a
clear unsupported-input error rather than a mesh that fills empty source space.
Use explicit refusal as the smallest complete fix; general concave polygon
triangulation is not required for this ticket.

The audit's valid planar U-shaped face declares nonconvex polygons. A triangle
fan currently covers its empty notch. The VRML97 format permits that face, so
the preview must either support it correctly or acknowledge its narrower subset.

## Acceptance criteria

Moved into [170](170-bound-wrl-traversal-work.md), which owns implementation and
verification of both WRL findings. Do not implement or claim this ticket separately.

## Comments

Created from the user's audit findings after primary-source research.

2026-10-18: The user requested merging the two WRL findings. Retired as merged
into 170; `wontfix` means no separate implementation, not rejection of the defect
or a claim that it has been fixed. The original finding above remains for history.
