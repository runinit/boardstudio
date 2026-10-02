# 03: Find saved keyboards by name

**What to build:** Users can search the saved-keyboard cards, clear the search and open a matching result.

**Blocked by:** 02: Browse and open saved keyboards.

**Status:** draft — awaiting breakdown approval

- [ ] Match reference case-insensitive, trimmed name filtering, including the current keyboard, Untitled keyboard fallback, Clear search and No keyboards match your search.
- [ ] Opening a filtered result uses the same safe open action as unfiltered cards; clearing restores the correct current/sorted list.
- [ ] Filtering is local presentation state: it changes neither saved records nor active document/history/selection and does not trigger a new provider query for every keypress.
- [ ] Confirm search keyboard focus, long names, empty search, no match and failure/retry coexistence in the real library. Demo cards remain independent of the saved-keyboard search.

- [ ] Complete the tranche’s shared acceptance and independent review; record refactoring takeaways or explicitly record none observed.
