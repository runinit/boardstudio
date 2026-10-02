# Binding select option selection: independent Spec review

**Source clear; no material findings.** Reviewed worker binding_editor.rs SHA-256 `01354a385e46ce283f73427f993bd74eb1f0d4385abd76a89d990c14909ff74e`, exact diff against `0302177a`. Read retained encoder-public-workflow baseline DOM evidence showing clockwise/counterclockwise/push selectors displaying key-press despite the reported accepted None state. No browser replay or Cargo performed.

The delta adds declarative option.selected to the four dynamic select families. Behavior compares each existing candidate with Behavior::from_binding of the accepted value; Hold compares the existing modifier with accepted hold; layer and macro compare stable IDs with their accepted selected IDs. Option values/order, supported behavior defaults, disabled conditions, input handlers, field merging, request identity and Core submission are unchanged. No synthetic change event or default binding write is introduced.

This correctly addresses the reported mount-order failure by marking the matching option when options are inserted, rather than relying solely on the parent select value applied earlier. It applies consistently to ordinary keys, encoder directions and reported push because they share BindingEditor.

Root must still verify the exact implementation on a fresh build: initial None/transparent and non-first values display correctly without document mutation; native selection commits the intended value; hold/layer/macro selections remain correct after accepted updates and Undo/reopen. Genuine public red remains retained. Source clearance does not claim regression green or whole binding/encoder acceptance.
