---
status: accepted
---

# Recreated inputs start unassigned

An input recreated after deletion or removal by matrix shrink starts unassigned,
even if it reuses the same coordinate or generated ID as a former key. Do not revive
the previous input's bindings merely because the position returns, including after
saving and reopening between removal and recreation. The user chose this behavior
on 2026-10-20 rather than automatic binding inheritance for replacement inputs.

Apply the distinction to typed keymap bindings and legacy key-position assignments;
do not let one retained representation restore the old assignment behind the other.
A surviving key moved by row/column compaction is not a replacement and must not
inherit its removed neighbour's binding. Ordinary Undo restoring the original input
and its prior state is distinct from creating a replacement without Undo. No new
ID/tombstone or migration framework is prescribed by this decision.

## Amendment: temporary disable retains bindings (2026-10-20)

The user chose that temporarily disabling and re-enabling an input resumes its
prior bindings as the same input, including after saving and reopening between
those actions. A disabled key bound to A resumes A when enabled; it is not a
replacement under the deletion/shrink-regrow rule above. Retained bindings must not
make the disabled input active in generated firmware while it is disabled.

This applies to typed layer bindings and legacy key-position assignments. It does
not establish retention of every Part-owned appearance, generator override, net or
constraint record; those are separate from the binding choice confirmed here.

The [input-lifecycle research](../research/2026-10-20-compatibility-identity-provenance-deep-dive.md#b-input-identity-and-binding-lifecycle)
traces divergent generic shrink/removal and structural-deletion cleanup, including
retained maps across Open. Current source behavior is not a uniform implementation
of this policy; public edit/save/reopen/firmware tests remain necessary. This
clarifies logical binding ownership without changing [ADR-0024](0024-share-reversible-pcb-routing-not-half-firmware-configuration.md)'s
assembly-specific electrical mapping or inventing a persisted hardware revision.
