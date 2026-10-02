# RF-006 bounded encoder default projection handoff

A fresh public candidate at source `e82c039b` / build `frontend-encoder-bindings-20261002` (34685) rendered the clockwise, counterclockwise, and reported push selectors as `key-press` while the accepted archive had no corresponding values (revision 9). Read-only document evidence confirmed the accepted sensor/binding maps were empty and no keycode fields appeared. This was a real public UI expected-red, not a data mutation. The exact baseline artifacts and hashes are recorded in the migration run ledger.

The narrow correction at source `868edfcb` declares each dynamic option's selected state from the accepted binding. The fresh build `frontend-encoder-select-fixed-20261002` (34687) now reports `none` in all three control values and selected options, with no code inputs. This establishes the targeted empty-value UI regression only. Genuine archive edit/recovery, attached-module behavior, F5.2 physical inputs, F8.2 firmware delivery, actual assistive technology and full F6K.4 acceptance remain open. No Core/API change or broader scope/design finding is proposed; retain under existing RF-009, with no new RF ID.

Evidence: `.scratch/dioxus-keymap-layers/evidence/encoder-public-workflow/` (34685 red) and `.scratch/dioxus-keymap-layers/evidence/encoder-select-regression/` (corrected source/build, 34687 targeted green).
