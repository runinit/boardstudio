# Preference storage fallback

Candidate `frontend-functional-controls-20261003`, source `ad26b07a`, root URL `http://127.0.0.1:34778/`; owned helper session `bs-mig-726e71041a27`.

Opened Sofle v2 and selected Dark in Project → Workspace settings. Filled this isolated browser's real localStorage quota with one test key (5,242,539 characters; actual QuotaExceeded failures while converging), without changing application/runtime code or project data. Selected System, whose longer stored value could not fit. The live theme changed to light for System while stored theme stayed dark. Exactly one status notice appeared: “Preferences are session-only because browser storage is unavailable.”

Objects → Auto-hide worked in memory: the Objects rail appeared while stored `boardstudio:v2:panel:left` remained `{"mode":"pinned","width":null}`. The notice remained singular. Edited Board name to “Preference fallback board” and reloaded; the board edit persisted, while the nonpersisted theme and panel settings returned to stored Dark/Pinned. This distinguishes failed preference persistence from functioning project persistence.

Pass: real write-quota failure, visible fallback, usable theme/panel controls and independent saved document editing. Browser API access/read denial was not separately forced. The notice is an explicit migration acceptance requirement; no claim that the TypeScript app displayed this same notice. Existing ordinary theme/panel reference evidence remains unchanged.

A second helper-owned session `bs-mig-b2fb10cdd285` launched Chromium with its real `--disable-local-storage` flag. `window.localStorage === null` was confirmed. The library stayed usable; Start Sofle v2 opened the editor. The same single preference notice was present. Project → Workspace settings → Dark changed the live `data-theme` to dark and selected value to dark. Objects → Auto-hide produced the Objects rail. No application/browser Storage API was mocked. Separate active-keyboard restoration warnings were expected because that browser capability was intentionally absent. This adds actual missing-storage access coverage to the earlier write-quota test.
