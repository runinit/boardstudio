# Encoder context cleanup: independent Spec review

**Clear; no material Spec findings.** Reviewed worker `binding_controller.rs` SHA-256 `e5e96a4b35e2b7590d8072b08822fec0e2a8f17440bcb810f67dfb9cb6c36898`, exact diff against `fa355edd`.

`BindingProjectionSources` groups the same accepted source, canvas view, display Memo and independent live input getter. The display still reads the Memo and verifies its identity against the live getter; admission, terminal handling and feedback continue querying the live getter separately.

Every `BindingReadContext` call preserves the prior argument values. Admission alone uses `ExactAdmission`, equivalent to the former true branch; all success/failure/cancellation acknowledgements and visible feedback use `StableLineage`, equivalent to the former false branch. The helper maps these modes to unchanged complete-identity equality versus stable-lineage comparison. Ordinary-key behavior, target membership, full Scope guards, fresh field merge, exact operation ownership and pending release are unchanged. The borrowed layer argument is a type correction only.

No new operation/API/schema or ownership is introduced. Root call-site adaptation, compilation and public integration checks remain their existing gates; this review does not claim they ran. No source edits or Cargo performed.
