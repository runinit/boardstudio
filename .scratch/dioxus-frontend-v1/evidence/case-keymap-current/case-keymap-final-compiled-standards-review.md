# Independent Standards: final compiler-driven Case/Keymap corrections

Reviewed dirty integration against HEAD `75139d74bd164695c84683a3174b8ec705ec2763` / editor `03cacf16`. Exact final blobs: `case_bodies.rs` `495bdf1b6ee403031953c9bca175548ecc805d48`; untracked `case_controller.rs` `ace540efa40d0a0f383398f9256c9ae6eec431b5`; `keymap.rs` `4166adda881715c70b32b64c6b97aeb9cc2bd1cd`. SHA-256 respectively: `c4e881e4359408867ae4e3befa3fc4037a13738bc196a6cd364533ac8480993b`, `7ae83620f6d9e725664f8bac6ca92be78d72b5473404484e9e4fb4855e9b87fe`, `f845988588f2cad55e0de512bdbef5d5c6a37c5e7331b73b8a727d7fab96ba6e`.

**No material Standards finding in the released corrections.**

The Rc<dyn Fn> callbacks now create mutable local copies of Signal handles on invocation; they still mutate the same component-owned state. This neither clones the underlying draft/document nor creates another state owner. The explicit pending read guard retains its borrow only while checking correlation. Input/Escape callbacks capture owned request identities instead of borrowing feedback props, preserving the existing suppression semantics and valid callback lifetimes.

Owned `use_reactive` tuple handling matches installed Dioxus 0.7.10 `dioxus-hooks/src/use_reactive.rs`: reference dependencies produce cloned owned `D::Out` tuples. Removing dereferences and borrowing `&identity` therefore fixes ownership without changing dependencies. Controller token/scope `?` exits remain inside its unconditional memo callback. Removing unused imports/mutability has no behavioral consequence.

Moving the editor key and feedback state calculation outside RSX preserves full target identity and field/global status separation. Full-Scope child key, clean-versus-dirty reconciliation, per-request failure retry, synchronous busy guard and exact terminal operation observation remain present; prior source findings stay closed. KeymapView’s `pub(super)` reexport is used by the private parent’s bounds helper; no public library API is widened.

These changes follow `CONSTRAINTS.md` ownership, deliberate reactive dependency and copy rules. No allowance/lint suppression was added. RF: no new material Fowler smell; no measured performance claim.

Read-only source review; tracked diff whitespace passed. I ran no Cargo/build/browser checks. The filename describes compiler-driven corrections, not an independently verified compiler pass. Root compilation/public behavior, retry/Undo/navigation/recovery, responsive layout and accessibility gates remain separate. CSS P2 remains source-closed at blob `508205e13d4f81e9c7c133acc0c0f9a9529f88c6`.
