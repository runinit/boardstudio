# Audit correction log

Corrections are append-only so copied audit packets retain their original actions and timestamps.

## Parts selected Inspector / source provenance

- The Parts report recorded the served candidate at `868edfcb` as showing only the selected MCU title, while the checkout at `89b1de8a` had a richer `parts/details.rs` subtree. Root subsequently verified that `web/**` is identical between `868edfcbdf93315e866962c9c57543d26672f379` and `89b1de8a28fdf02db91d972c90a69235bfbbbffb`; therefore this is not a later-source/not-in-build change. Root and the Parts author’s corrected observation is that the candidate catalogue and meaningful selected-definition Inspector details are present. Treat the original “title only” pixel/snapshot as a possibly transient or profile-specific observation, not the current supported gap.
- The stable missing Parts surface is the separate library preview/control path in the center (selected footprint/model view, layer controls and fit-profile entry), plus any source-backed editing actions still absent. The selected-definition row and Inspector details are present; exact detail-field parity and full workflow still need a byte-identified paired run.
- The Project-menu gap remains independently supported: React has New/Create, saved library/search, setup guide, portable copy and 18 demos; candidate start in the captured fresh profile exposed two demo copies and archive import only.
