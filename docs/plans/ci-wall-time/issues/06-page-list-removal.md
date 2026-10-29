# 06: Page `--list` removal

Status: resolved
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

## Answer

Evidence supports check-speed 12's claim. Removal (serial page `--list` launch plus the dead
`--keep-going` prerequisite special-case and now-unused `wasm_pack` prefix var; `test-check.py`
keep-going test updated, 31/31 green) keeps the executed set identical: three consecutive local
green runs, 77/77 passed each, byte-identical `(test, status)` multisets versus baseline.
Warm-state saving is **~5s** (cold `--list` 141s vs warm 5s — the removed launch was ~5s of
launch plus a first compile the page runner still performs, relocated not saved). Branch
`research/page-list-removal` (`457d5fa0`, pushed, no PR); evidence in gitignored
`.evidence/page-list-removal/`. One red herring, not a regression: the first attempt failed 1/77
on the model-decoder test because gitignored `web/assets/renderer/` was absent in the fresh
checkout — green after the normal build preflight, same lesson as check-speed 11's recovery.
