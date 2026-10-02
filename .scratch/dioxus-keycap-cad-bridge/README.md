# BND.1 Private keycap CAD bridge proof

This automatically published ticket refines BND.1 in the existing 62-task graph. Publication leaves parent status and acceptance unchanged. Shared authority and acceptance remain in `../dioxus-frontend-tranche-1/AUTHORITY.md` and `../dioxus-frontend-tranche-1/ACCEPTANCE.md`.

Before implementation, complete the source contract review in [DISPATCH.md](DISPATCH.md). The goal is a private proof path to the existing packaged Rust `build_keycaps` export. The existing public CAD enum/request are outside scope: do not add a variant or field, widen visibility, or invent a new facade. If no current private callable seam exists, record the precise gap and stop at that boundary for a separate decision.

The agent-prepared ticket is in [issues/01-private-keycap-cad-bridge.md](issues/01-private-keycap-cad-bridge.md). The pre-publication planning manifest is retained as a historical snapshot at `planning-manifest.md`; the independent Astra review is retained under `evidence/`.
