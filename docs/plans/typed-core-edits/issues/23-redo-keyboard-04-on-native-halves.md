# 23: Redo keyboard 04 on the native mirrored halves

Status: wontfix
Type: task
Blocked by: [25](25-shipped-boards-review.md), [56](56-sofle-sign-off.md), [104](104-preserve-reviung41-original-archive.md)
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Decision: [ADR-0010](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md) · [16](16-native-mirrored-halves.md)

## Request

[Keyboard ticket 04](04-refresh-included-keyboards.md) stays `claimed`: its pair and bay work
is superseded by 16-22. Resume it on the native model.

## What to do

- Re-inventory the shipped keyboards; confirm each split is one measured half plus the
  native mirror, with exceptions recorded.
- Carry over 04's "Not done" list: mounted-browser and live paired-edit checks, the Standards
  and Spec reviews, the full check, the Sofle v2 closure-mount decision, and the fit
  findings recorded by generator-parameters 02 and 06 (KLOR bay overlap, tight switch gaps).
- Resolve 04 with an Outcome, or record what remains.

## Integration gate

Consume the saved board projects under25, including Sofle sign-off56 and preserved original REVIUNG41 archive104, before final global review. Shared mirrored-half/link functionality remains a separate required gate under19/22 and is not recreated in board projects. Ticket 56 owns its targeted mounted/live Sofle verification;
reuse sufficient fresh evidence here, while retaining 23's final Standards/Spec reviews and full integration check.
Optional decision 45 and deferred VIK work do not become implicit blockers. Planning completion alone does not
satisfy this gate: require the applicable board implementations, their independent verification and recorded human decisions.

## Comments

- 2026-10-14: User deferred built-in board work under [138](138-retire-shipped-demos.md). This ticket's board-specific work is out of scope for now; re-triage only if the user brings built-in boards back into scope. Existing findings remain historical evidence, not implementation instructions.
