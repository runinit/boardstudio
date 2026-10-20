# 013: Kernel rejects out-of-range mount positions

Status: resolved
Type: build
Blocked by: none
Parent: [CAD deepening](../map.md)
Related: [008: Kernel rejects coordinates Core would never send](008-kernel-coordinate-limit.md)

## What to build

`build_case` rejects non-finite or beyond-limit ring points (008), but mount positions (closure
bosses, standoffs and other **Case boss** centres) still reach the kernel unchecked. Extend the
same entry check to every mount position and any other point or length a prepared region carries
into the kernel, so a **Case boss** Core would never produce returns an error naming the body
instead of reaching the kernel.

## Acceptance criteria

- [x] A non-finite or beyond-limit mount position returns `Err` naming the body, tested through `build_case`.
- [x] A mount exactly at the limit still passes the check.
- [x] Kernel and CAD worker tests pass; kernel Clippy is clean.

## Comments

Checked `Mount.at` (the only position a mount carries) in the existing entry check, renamed `check_coordinates`. Hole/boss diameters and height are lengths, not positions, so they stay with their existing rules. The at-the-limit test asserts the error is not the coordinate error, since a mount at 1e6 mm is a geometry question, not a check question.
