# 03: Find saved keyboards by name

**What to build:** Users can search the saved-keyboard cards, clear the search and open a matching result.

**Blocked by:** 02: Browse and open saved keyboards.

**Status:** ready-for-agent

- [ ] Match reference case-insensitive, trimmed name filtering, including the current keyboard, Untitled keyboard fallback, Clear search and No keyboards match your search. The Dioxus implementation keeps the query in presentation state and filters the current-plus-saved card projection only.
- [ ] Opening a filtered result uses the same safe open action as unfiltered cards; clearing restores the correct current/sorted list.
- [ ] Filtering is local presentation state: it changes neither saved records nor active document/history/selection and does not trigger a new provider query for every keypress. Demo actions remain independent of the saved-keyboard search.
- [ ] Confirm search keyboard focus, long names, empty search, no match and failure/retry coexistence in the real library. Demo cards remain independent of the saved-keyboard search.

- [ ] Complete the tranche’s shared acceptance and independent review; record refactoring takeaways or explicitly record none observed.

**Implementation source pin:** React `app/src/ui/ProjectLibrary.tsx` at pinned baseline `5a472a9426e6e38993361da402cd4ec730feb369`, search projection and controls at lines 122–139. The isolated Dioxus patch adds local query filtering in `web/src/presentation/library.rs` and focus/compact styling in `web/assets/m1.css`; mounted regression coverage has been added for trimmed/case-insensitive matching, current and Untitled keyboard results, no-match copy, clear behavior and demo independence. Browser pairing, failure/retry coexistence, landing-flow/public acceptance and shared tranche review remain open.
