# Inspector keyboard item cache candidate

The focused public pointer run for candidate 9a5b20985a9db1f4c0f0d635c43e1dd5439bbfd9 improved the 100-key p95 from the prior red run but still missed the limits: 30 keys 22.7 ms (limit 33), 100 keys 96.3 ms (limit 50), and 200 keys 344.5 ms (limit 100). Its 9a build and run evidence remain preserved as the second red baseline.

Read-only source inspection found the Inspector rendered component_items.clone() for the list and cloned the full vector again into every button’s key handler. Each tuple owns three strings (id, reference, kind); at 600 parts, per-button whole-vector clones create roughly 1.08 million string clones per rerender. The change stores that ordered vector behind Rc<Vec<_>>; each handler captures an Rc clone and reads the same vector for ArrowUp/ArrowDown/Home/End. The Dioxus iteration uses a single .iter().cloned() traversal to satisfy its owned element lifetime, so remaining list tuple cloning is linear in item count. Ordering, indexes, active element IDs, and selection behavior are unchanged. No runtime, transform, snapping, or API changes.

Focused page built successfully with the locked release command. Public UI keyboard verification on Sofle v2: ArrowDown selected/focused index 1; Home selected/focused index 0; End selected/focused index 69; ArrowUp selected/focused index 68; Space selected focused index 5; Enter selected focused index 2. Browser reported no page errors. The old 9a stage remains available on port 46911; this candidate is staged separately on port 46912.

Checks: cargo fmt --manifest-path web/Cargo.toml --check, cargo test --manifest-path web/Cargo.toml --locked --no-default-features --features page,cad-worker (12 passed), and strict WASM page Clippy all passed. git diff --check passed.

New pointer performance measurement has not run yet. This staged artifact intentionally omits service-worker/offline initialization and is not release/offline acceptance. Exact source and all staged SHA-256 values are in inspector-cache-candidate-20261001.json.
