# 012: Parts single-draft owners

Status: ready-for-agent
Type: build
Parent: [Draft-safe update adoption spec](../spec.md)
Blocked by: [Draft-owner registry and Geometry scripts tracer at both Update Now checkpoints](008-registry-and-geometry-tracer.md)

## What to build

Registry adapters and per-key deferral copy for the Parts single-draft
editors sharing the one-Save/Apply pattern: generator parameters (including
its Upload admission key), the module profile editor, and the mechanical fit
profile editor. Their unapplied Draft values defer Update Now until resolved
through each editor's own controls.

## Acceptance criteria

- [ ] Each owner defers Update Now with a real valid unapplied draft and with a realistic invalid unsubmitted draft, at both entry points, via the mounted Update Now seam.
- [ ] Generator parameters are judged per admission key: submitting one key (Apply or Upload) does not clear an unresolved draft belonging to the other.
- [ ] Each deferral message names its editing area and per-key remedy; resolution happens only through that editor's own controls.
- [ ] Per-key behaviour follows the contract: landing updates the baseline, failure/refusal never clears, revert-to-baseline clears, newer typing re-raises, older settlement never clears newer input.
- [ ] Hidden or keep-mounted instances keep blocking; unmount or owner change retires them.
- [ ] Each adapter change carries its own RED/GREEN evidence through the mounted seam; focused checks pass and evidence logs follow the plan's conventions.
