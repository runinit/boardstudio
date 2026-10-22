# Shared control prerequisites for 016, 017 and 020

This slice breaks the shared-control dependency cycle without importing any workspace migration.
The coordinator owns closure review and local feature landing; workspace claims and global acceptance
remain unchanged.

## Source pins

- Feature base: `f11e6bd84456b94b2d7657f57a0d05b42184c4e7`, fast-forwarded from the clean bound
  `feat/shared-control-prereq` checkout at `b237903705244a266b07a64e5f23b8a3b5219c03`.
- Button: cherry-pick `01c631e8ba1033515d8716db8ffec32c3673add6`, tree
  `2bdf85a35f7bca06b2a59a4e494d608584061021`, parent equal to the feature base. Git source contains
  145 additions in `ui/button.rs`, including the forwarding test module. The contributor's earlier
  statement that those tests were uncommitted was contradicted by that source.
- Checkbox: cherry-pick `150670bad0c4add2e4f6b63bcc49b6fe68d0868b`: only the public upstream
  `CheckboxState` reexport in `ui/checkbox.rs`; `CheckboxProps` remains privately imported.
- Input Group: cherry-pick `df96cddad9a326a25bdfd0acff6692ef6eacfd3d`: 169 additions and one deletion
  in `ui/input_group.rs`, including focused tests. No Case migration ancestry is included.

The previously reported Button forwarding 1/1 pass used dirty workspace dependencies. Its source
blob matched the isolated contract, but that receipt is not clean combined verification. The
`href: Option<String>` fixture correction was compile evidence, and the 020 `TooltipTrigger.onclick`
E0599 was a fixture compilation failure, not a behavioral RED.

## Approved contracts

- Button adds optional keyboard, focus, blur, pointer-enter and description props. Existing owners
  remain in control; native and link branches forward click and pointer callbacks without adding
  navigation suppression or changing disabled-link behavior.
- Button additionally accepts an explicit `attributes: Vec<Attribute>`, without a blanket
  `GlobalAttributes` extension. Tooltip composition uses the existing public `TooltipTrigger.as`
  render callback and this vector on the same Button element. Raw listeners run before typed owner
  listeners, once each. The pinned Tooltip uses **mouse** enter/leave, focus, blur and Escape;
  pointer events remain separate.
- Button's class, style, `data-slot`, type, href, disabled and explicitly represented scalar
  attributes remain authoritative, including omitted scalar defaults. A raw id is a fallback when
  the owner omits id. Description id tokens are joined and deduplicated. Other raw attributes are
  retained. Composition does not introduce a separate style implementation or a new dependency.
- InputGroupInput adds max and native global attribute/event forwarding. Only exact collisions
  with explicit native attributes/listeners are filtered; explicit value, class and input listener
  remain authoritative. Other events, namespaced styles and validation ARIA pass through using
  public Dioxus APIs. The spread stays last, as Dioxus 0.7 requires.
- Checkbox retains upstream controlled, uncontrolled and disabled behavior; no visibility is widened
  beyond reexporting the already-public state enum.

## Verification and handoff

The first prerequisite link-callback run at `f42029438168e5296e15c7216593217762b23c33` compiled, but
timed out waiting for native pointer callbacks because the fixture's synthetic events did not bubble
to Dioxus's delegated listener. `.evidence/shared-control-prereq/button-link-red.log` preserves that
fixture failure; it is not the expected product RED. `bd311f9b` corrects the event initialization to
bubble (and makes the click cancelable so navigation suppression can be detected).

Pending: clean combined focused native/WASM checks, formatting and repository checks, source/test
pin and evidence paths, one combined Standards and Spec review through the coordinator, then
approved local feature landing. No workspace ticket is closed by this note.

Builds use the exclusive target
`/home/chris/.cache/boardstudio-target/shared-control-prereq-t3-b5fe12f0`, outside `.evidence/`.
The coordinator allocated the 017 slot to this slice while 017 continues independent source work;
the slot is returned explicitly when this build phase ends. Target cleanup waits for approved
landing and confirmation that no active owner remains. Evidence is preserved.
