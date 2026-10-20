# 013: Kernel rejects out-of-range mount positions

Status: ready-for-agent
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

- [ ] A non-finite or beyond-limit mount position returns `Err` naming the body, tested through `build_case`.
- [ ] A mount exactly at the limit still passes the check.
- [ ] Kernel and CAD worker tests pass; kernel Clippy is clean.

## Comments
