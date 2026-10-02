# F6K.4 encoder/firmware ticket split publication

Published local child files after exact-hash independent Spec and Standards clearance:

| Published issue | Final SHA-256 | Status |
| --- | --- | --- |
| `issues/06-encoder-rotation-editor.md` | `9fb4f14b207f547cd87de7fa52ccfb3537dd6c46fcacff3b5011289621092aa4` | Published; implementation dispatch held pending independent review of the separately authored private binding-target contract. |
| `issues/07-pcb-firmware-position-editor.md` | `d86689bf058b1fb8dd6db4bbda476569c7d759aa19ab69a4ec4a25e690ace857` | Published; existing F6K.1/F6K.2 start gates and F5.2/F8.2 parent joins remain. |
| `issues/08-zmk-firmware-export-delivery.md` | `3376e546389ca4eb010e30949c38cb30a8cfa78959a27df619fb93e4d42029cb` | Published; real private firmware/provider delivery route remains an implementation gate. |
| `issues/05-encoder-firmware-handoff.md` | `576eedfd4f39501268aaaab6b219f043705cb740e11db3d84a5b1567d5d596f0` | Aggregate superseded for dispatch; original criteria and history retained in place. |

The reviewed draft hashes are recorded in [`spec-review.md`](spec-review.md) and [`standards-review.md`](standards-review.md). Source mapping and actual current boundaries are in [`source-contract.md`](source-contract.md). The encoder shared-target contract is deliberately separate and remains an unreviewed draft: [`private-binding-target-contract.md`](../../drafts/encoder-target-port/private-binding-target-contract.md), SHA-256 `621216338d84e127d129fa07a1855e75552c68d71fe2c568e6e27523c3bba196`.

## Count method and canonical graph

The portfolio now has **44 published bounded ticket records**: this is a cumulative count of all published local issue files, including the retained superseded issue05 record. There are **43 non-superseded published ticket records** after excluding that one aggregate. “Non-superseded” does not mean unblocked, ready for dispatch, or accepted: issue06 has a contract-review dispatch gate, and other child status/parent gates remain their own.

The canonical graph still has **62 parent work packages**. F6K.4 still starts after F6K.1 and F6K.2 and retains F5.2 and F8.2 as its exact acceptance joins. No parent, edge, source code, Core API, schema, or visibility changed.
