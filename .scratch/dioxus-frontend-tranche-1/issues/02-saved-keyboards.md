# 02: Browse and open saved keyboards

**What to build:** Your keyboards displays usable saved-project cards and opens the chosen keyboard through the existing storage/session flow.

**Blocked by:** 01: Isolate the existing workspace UI for parallel work.

**Status:** ready-for-agent

- [ ] Show the current keyboard once, followed by saved keyboards in reference name order, with reference name fallback, key/board summaries, thumbnail geometry, current marker and accessible open action.
- [ ] Cover loading, empty and populated storage, list failure with Try again, no-keys preview, and damaged-preview fallback. One preview failure must not hide healthy records or prevent an available project from opening.
- [ ] Use real saved data. Open succeeds into the selected project; unavailable/error responses leave the existing project usable. Repeated or superseding opens and late list responses cannot overwrite newer project/list state.
- [ ] Browsing, listing and retry leave document revision/history/selection unchanged. Exercise the same library from entry and the existing project menu, with reference keyboard/focus and light/dark/compact behavior.
- [ ] Integrate and verify this slice independently; create/delete/import/portable-copy redesign stays with its existing parent tasks.

- [ ] Complete the tranche’s shared acceptance and independent review; record refactoring takeaways or explicitly record none observed.
