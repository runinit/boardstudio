# Independent Parts Spec review

Reviewed `edf59161...5b84096f`, F4.1a/dispatch, cleared source-loading contract, and React source unchanged from pin `5a472a9426e6e38993361da402cd4ec730feb369`. **Four source findings; private implementation not cleared as-is.** No source edits/builds/browser execution; `git diff --check` passed.

1. **P1 — Construction parameters become JavaScript Maps.** The cleared contract requires preserving “constructionDefinition … reversible parameters and the Gateron solder-only variant.” `parts/catalogue.rs:274` uses default `serde_wasm_bindgen::to_value`; installed 0.6.5 serializes `BTreeMap` to JS Map. `PartGenerator.parameters` is such a map, but retained `normalizeDefinition`/`render` reads object properties/spreads (`ergogen/src/index.ts:75,114,363`). Inserted reversible/hotswap/solder values therefore do not drive generated geometry; defaults win even while returned parameters retain the requested values. Serialize maps as plain objects at this boundary and prove typed reversible/Gateron results before acceptance.

2. **P2 — Category searches omit aliased entries.** Ticket: “Search names, source/generator terms, category and supported aliases.” `catalogue.rs:48–63,330–335` includes category only as the fallback alias. Switches and SK6812 replace it with other aliases: searching “Switches” or “Passives & LEDs” omits reference matches. React `PartsLibrary.tsx:16` always appends category independently. Preserve both fields.

3. **P2 — Selected-detail fallback differs.** Ticket: “empty and no-match feedback match the reference.” `parts.rs:99–119` returns no selected definition when an initial search has no matches; React `Workbench.tsx:305` ultimately falls back to `availableLibrary[0]`. Matching fallback also uses category-group order/category search, whereas React uses original catalogue order and its separate `filteredLibrary` predicate. Port that exact resolution order while keeping explicit choices stable.

4. **P2 — Assembly exclusion loses case-insensitivity.** Ticket: “Preserve … assembly-snapshot exclusion.” `catalogue.rs:351–354` uses case-sensitive splitting/prefix matching; React `partsCatalog.ts:22` uses `/i`. IDs such as `ASSEMBLY-x/definition/switch` or UUID `/DEFINITION/` snapshots incorrectly become products. Preserve the reference matcher.

Precedence and insertion position, project override timing, immutable accepted Arc input, memoized merge, scoped async results and read-only callbacks otherwise match the cleared ownership. Meaningful selected metadata exists. No API/schema expansion found.

Root mount/CSS, typed actual-catalogue execution, failure/offline root/subpath proof and inherited browser/accessibility gates remain open; unmounted code cannot complete F4.1a. No new structural refactoring takeaway; boundary-shape evidence reinforces existing RF-002. Other panel/T1-02 findings remain outside this review.
