# F3.5 matrix-context Properties / Relations source implementation

**Source base:** `e57fd3a87a8b3821ac5458cf8d2988a596e14bfc`. **React reference:** `5a472a9426e6e38993361da402cd4ec730feb369`, `app/src/ui/Workbench.tsx:1319,1400-1401`.

Dioxus now shares the Layout Properties/Relations tab state between its existing standalone-component Inspector and Matrix/Row/Column/Key contexts. Matrix-owned Relations projects the pinned precedence (linked layout, independent layout, active selected-part constraint, then empty state), keeps the matrix geometry note, offers the reference Edit placement relationship route for a live selected member, and is reachable from Align → Relationships. The action reuses the current accepted scoped tree-selection callback. Existing matrix structural/transform controls, component constraint editor and findings page remain with their current owners.

The accepted source also now resolves matrix-member layout context like React and hides standalone Layout assignment for those members. This is needed when the Relations action enters the explicit component context. No new edit operation, document store, public API or refactoring finding is introduced; preserve RF-001 and RF-006.

**Qualification:** Source implementation only. No build, tests or browser journey was run for this packet; integration compilation and the changed mounted journey remain open. This does not accept F3.5 or its parent criteria.
