# 06: Page `--list` removal

Status: claimed
Type: task
Parent: [CI wall time map](../map.md)
Related: [12: Browser step quick wins](../../check-speed/issues/12-browser-quick-wins.md)

## Question

Produce the fresh evidence check-speed 12 needs on the duplicate serial page `--list`: re-measure
with receipts under the check-speed-12 protocol. This ticket is evidence for check-speed 12's
claim, not
part of the 17-minute arithmetic.

## Pointers

- Check-speed 12 owns the candidate; coordinate there, duplicate nothing without new numbers.
- Expectation reset: the 35.8s cold `--list` likely includes the page test binary's first
  compile, which removal moves to the page runner rather than saving — budget ~6s either way.
- State where the three green runs happen: locally per check-speed 12's protocol (not three
  ~34-minute CI runs).
- Protocol: executed-test-name multiset equal to baseline, `scripts/test-check.py`
  command-order update, three consecutive green runs.
- Cold green log shows the `--list` at 35.8s; warm local runs show 5–6s. Report both.

## Comments

- Claimed for implementation delegate
  `node:delegated-task:command%3Amcp%3A4a120a31-63bc-4bf2-95d6-d041fca261b3%3Adelegate-task%3A1016ac7d-c5e2-44f3-927d-1eba4a40a134`
  (local evidence runs; branch `research/page-list-removal`, no PR).
