# F5.2d Apply mode-consistency repair

Source repair commit: `300a9d5dceb4f08ee24de5c898f2420ef92b80b3`.
Parent source packet: `e68702543397e7cf5301fed7e9b93286a6b2bf44`.

`Apply wiring` now admits a resolved plan only when `plan.mode` equals the accepted target board's configured electrical mode; an absent board configuration means the Core default, Matrix. This closes the source-review finding where a Direct plan could be applied to a board whose accepted document still resolved to Matrix. The positive mounted fixture now uses an error-free Matrix plan returned by `electrical::resolve`. Negative mounted production-hook coverage includes a valid Core Direct plan against an accepted default-Matrix document, an error-bearing current plan, Pending, and Failed resolutions.

The expected-defect red is `mode-mismatch-regression-red.log` (SHA-256 `786a371be6dfefdf9382fbd5b63194537748557f66884400a996c40b9ccc2485`). With only the new mode equality guard temporarily removed, the mounted mismatch assertion failed because Apply remained enabled. The guard was restored before the green run. `mode-owner-green.log` (SHA-256 `cce70329c31616e26885236b66f8e598ada327810b1aee10a56db16908fc17f9`) records 9 passed, 0 failed, including the mismatch, error, Pending, and Failed cases.

These are native `VirtualDom` tests mounting the production owner/hook with the test Runtime stub. They manually settle the captured operation outcome; they do not execute a real Session or establish packaged public behavior. Formatting and `git diff --check` pass. Fresh affected WASM and integrated strict checks remain for the root serial join.

The earlier e687 source-review report at `/home/chris/.local/share/boardstudio/reviews/pcb-apply-ui-source-review-e6870254-sol-20261002.md` (SHA `4fcf03e9f533efbacdb344275853d23774d30db8fde6732607435b8f145b4cc4`) remains the review record for the finding; this addendum identifies the exact repair packet for rereview. Public Apply/history/reopen acceptance remains open.
