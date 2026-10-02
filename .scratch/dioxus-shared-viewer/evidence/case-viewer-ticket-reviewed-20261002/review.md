# Independent Astra review — F7.3 Case viewer draft

**Approve publication and bounded Luna High dispatch. No material blocker found.** This is ticket/contract clearance, not implementation or parent acceptance. Read-only review at integration HEAD `75139d74bd164695c84683a3174b8ec705ec2763`; no repository edits or Cargo/browser run.

Reviewed SHA-256:

- Draft02: `e16a7d30c97014f0f3fd005e267c66f5a7136067b711f2238bb0c79497960879`
- Corrected F7.1 issue: `7a15f3f7aa67f31e6e9334d0bde1a1ebc7215ef45352ccb063deac47433d1b1a`
- DISPATCH: `183613a5ce784f5544411d785a7d805cad4f35963eaee94aa01ee16a934e1109`

Reused the retained final Astra standards review/source-contract packet, then checked current main.rs, renderer_host.rs and CaseCanvas call path. The private binary inclusion of the same host source is coherent: existing library behavior stays intact, new methods/DTOs remain crate-private, and actual page-binary plus library checks must establish reachability. No public widening, engine operation, provider or format is authorized.

The proposed child is demoable through existing Case navigation, current scene projection, real viewer controls and preserved forms/2D route. It explicitly leaves physical-instance handoff with F7.7 and later consumer adoption open. The62-task graph remains unchanged: F7.3 starts only after F7.1; INT.2/BND.1 remain acceptance joins. Five-consumer coverage is retained by incorporation of F7.1 and the explicit non-closure statement.

Identity/lifetime requirements retain full application Scope, independent viewer/projection generation, strictly increasing renderer scene sequence, and current-owner checks for async/status/pick/gesture output. Parts identity reuse is explicit. Context loss stops rendering without inventing restoration/retry UI; provider retry remains its existing behavior.

Ownership is serial: author extends host/private viewer/projection; coordinator registers modules, mounts Case and edits shared CSS/build files. The required dispatch packet must name exact paths before coding. Public paired fixture actions and actual renderer fault/lifecycle probes are appropriate; do not replace them with fake providers or claim AT/performance proof from build success. Downstream-only handle/consumer semantics remain downstream gates.

**Nonblocking precision correction:** the paragraph naming F7.8 lists F3.6/F4.4/F6C.5 but omits its additional canonical acceptance join **F2.3**. Add F2.3 explicitly. The draft already preserves all existing joins, so this is clarification, not a graph change; no new review cycle is needed for that exact addition.

RF: no new refactoring takeaway observed; RF-002/RF-012 remain applicable.
