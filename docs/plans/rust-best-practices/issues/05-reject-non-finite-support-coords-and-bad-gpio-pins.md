# 05: Handle untrusted support-polygon floats and malformed firmware GPIO pins without panicking

Status: ready-for-agent
Type: build
Blocked by: None (can start immediately)

**What to build:** Support-polygon validation and orientation checks reject non-finite (`NaN`, `±Inf`) coordinates with a validation error instead of panicking in rational conversion, and firmware devicetree generation falls back to its `/* unresolved GPIO ... */` marker when an imported `P<port>.<pin>` string has a non-numeric pin suffix instead of panicking.

## Acceptance criteria

- [ ] Passing a polygon ring containing `NaN` or `±Inf` coordinates into the support-region validation/normalization API returns `Err` instead of panicking on `BigRational::from_float`.
- [ ] Firmware generation with a malformed GPIO pin string such as `"P1.X"` in either direct-key or flags mode emits `/* unresolved GPIO P1.X */` instead of panicking on `u32` parsing.
- [ ] Valid finite polygon rings and valid `P0.17` / `P1.02` GPIO strings continue to produce identical output.
- [ ] Unit tests in `contracts` and `core` cover non-finite polygon rings and malformed GPIO pin strings.
