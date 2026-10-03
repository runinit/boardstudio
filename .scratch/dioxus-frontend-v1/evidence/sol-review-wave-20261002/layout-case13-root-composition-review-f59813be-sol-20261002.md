# Layout layers and Case13 transport root composition review

Sol 6.1 High: **Standards CLEAR; Spec CLEAR** for bounded root composition `f59813bebe2512f1660b33871f560413cc2febad`. The subsequently frozen build source `b9e74e37fc34948be9fee97c918b644200778bca` differs only in documentation; I verified no diff in the maintained app/provider/script source directories.

The independently cleared Layout source is `4c2809a604820aab7c79bf2508739ce01bd3cf9f`, report SHA-256 `2f3b948f24699db40b4e5a11a7e91d87bcdd2d6f95b158d31d67cc1b2ec7e968`. Root `shared_viewer.rs` is byte-identical, SHA-256 `b4d80211b58ceced7797ede0aceae03ee624e740fa36c6c4a0c580e89d54bf5a`.

The independently cleared Case13 source is `6dfb7fe4b22b74eab878a1c590e3fe302d2bdd0b`, integrated at `2d6e9c2699a175ca5476d61ce4a3abf113007318`. I read and reused the independent Case13 source review and its evidence follow-up (root report SHA-256 `0157af3fb6d589d8aac8f7d180ae643e94b87c0ad0e670b2e7bcfe30f3ae959b`). Root `presentation.rs` is byte-identical to that cleared source, SHA-256 `aeb6b4fb641f28501eb3db05561ed7912d7ab1905b735dcae3aaa1411a014017`.

Root stylesheet differs from the cleared Layout stylesheet only by the independently approved ready-dot success-surface token. Layout container/compact placement is preserved. Root CSS SHA-256 is `096f24e191223580878d00d9a34458b0a54f031b2f1ef4ca357413f8ef756977`. RF-006 exactly matches the reviewed Case source update; every other existing finding, including RF-015, remains unchanged from the preceding Layout join.

Verified checks: exact frozen commit diffs and source hashes; refactoring JSON composition; app-source whitespace check; retained combined strict page/core-worker all-target WASM Clippy completion, 8.00 seconds, log `/home/chris/.local/share/boardstudio/retained-tmp/20261002/layout-case-transport-root-join-clippy.log`, SHA-256 `6dc2baca9b24445549dec650cb3cbe9c659ed81cd6f100264e5fb695c83972b9`. No heavy rerun, source edit or browser acceptance was performed.

This clears composition of the already-reviewed bounded fixes. The in-progress package is not thereby qualified. Actual Layout per-model visibility/geometry, Case transport control acceptance, archive persistence, Undo/Redo/reopen, wireless battery behavior and all existing public/parent joins remain governed by their separate receipts. Source presence and combined Clippy do not waive them.
