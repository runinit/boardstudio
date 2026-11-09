# 05: Handle untrusted support-polygon floats and malformed firmware GPIO pins without panicking

Status: wontfix
Type: build
Blocked by: None (can start immediately)

**Original proposal (not scheduled):** Reject non-finite support-polygon coordinates and make malformed imported GPIO pin strings produce an unresolved-GPIO comment rather than panic.

## Answer (2026-10-19)

Not required as a product fix. [Research 05](../../../research/rust-best-practices-ticket-validation.md#05--non-finite-rings-and-malformed-gpio) traced the public boundaries:

- Support-region/profile validation already rejects non-finite coordinates before rational conversion, which itself uses `ok_or`; an infinity regression already exists. The separate boolean self-intersection predicate guards non-finite coordinates before its private orientation assertion. Do not change that predicate's signature to satisfy a panic-removal heuristic.
- Public firmware admission validates scan, UART and encoder GPIOs against the selected built-in controller profile before calling the formatting helpers. Malformed `P1.X` requests return `Err`; they must not become successful packages containing unresolved markers. No caller-provided controller table bypass was found.

The original marker acceptance criterion would weaken existing validation. Leave production code and guarded invariants alone. If later work warrants additional regression coverage, test `NaN`/positive/negative infinity in exterior and hole validation, and malformed scan/encoder pins on both firmware halves returning `Err` without a package. Private helper fallback cleanup is optional maintenance, not a reason to reopen this bug. Reopen only with a reachable counterexample against the public APIs.
