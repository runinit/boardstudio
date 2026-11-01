# 02: Recover prototype anchor highlighting

Status: resolved
Type: research
Map: [map.md](../map.md)

## Question

What anchor-relationship highlighting existed on `prototype/ergogen-parity-inspector`
(and related Claude branches) that never shipped, and what is the delta vs shipped
ticket 24?

## Outcome

An inventory of the lost highlighting (triggers, visuals, navigation) plus the delta
vs shipped ticket 24 behavior — enough for ticket 06 to place it. No behavior change.

## Answer

Lost prototype = variant D on branch `prototype/ergogen-parity-inspector` (44c80fa98, prototype HTML). It had a "How it is placed" chain (kernel order, own step accent, downstream dashed, clickable owners), an Anchor section (Follows X, Follow x/y/r, Pick, Detach), and per-field hover/focus amber "Moves N keys" affected preview. Ticket 12 decided transported cumulative shift (10,10 not 20,0), violet dashed references, amber = edit previews only. Ticket 24 shipped the chain + drag previews. MISSING: per-field hover/focus amber preview at rest; anchor framing lives in the Relations tab (`layout_workspace.rs:535`), not beside selection. Branches `claude/strange-kalam` + `codex/inspector-copy` hold no competing prototype.

(pending)
