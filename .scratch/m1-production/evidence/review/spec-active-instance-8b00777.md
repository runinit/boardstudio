# Spec review: active-project restore and physical-instance navigation (`8b00777`)

Review base: `b9748745ec444416136a97d4befd075670ef49b7`  
Reviewed source commit: `8b00777fd499aeafe660ad0cb7f11eef97d7bbb3`  
Production diff: `web/src/presentation.rs`, `web/src/runtime.rs`; SHA-256 `403aff19ae3be655873a9ae1074689b9065393858bee68bdf39a87bf6b329f52`.

**Spec result: no blocking findings.** M1 story 4 asks for active-project restoration so editors can resume saved work. `Runtime::new` now reads the scoped active-project preference and opens its saved document through the existing asynchronous `open_saved` path. A missing preference remains the library start state; read/open failures are reported with a public recovery choice. `open_saved` uses the existing open-sequence guard, so a later explicit open can supersede delayed startup restoration.

M1 story 5 asks for part selection and board/instance navigation. The new selector is populated only from physical instances attached to the active board and includes a canonical-board option. It submits the existing `Navigate` event with the captured board ID; the session validates that the requested board and instance belong to the accepted document before changing scope and cancelling in-flight work/exports. The existing board selector clears instance scope when changing boards. This adds no persisted field, public API or alternate document authority.

The new controls retain a visible label and accessible name. This review is source-only; provider browser red/green, final build and renewed full acceptance are separate pending evidence.
