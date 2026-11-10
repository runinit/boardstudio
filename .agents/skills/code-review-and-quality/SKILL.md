---
name: code-review-and-quality
description: Five-axis review lens (correctness, readability, architecture, security, performance) with severity labels. Use when the user asks for a quality review beyond the two-axis `code-review`.
disable-model-invocation: true
---

# Code review lens

A reference lens for **one pass** over a fixed diff. `code-review` is this project's default review. When a review happens, who runs it, what is reviewed again and which mutations run all come from the [slice boundary](../../../docs/agents/issue-tracker.md#slice-boundary).

**Approval standard:** approve a change that improves overall code health and follows project conventions, even when you would have written it differently.

## One pass

1. Read the spec or task, then the tests, then the implementation.
2. Read the recorded evidence (RED/GREEN logs, mutation results). Run nothing yourself.
3. Walk the five axes, report findings by severity with the highest leverage first, give the verdict, and stop.

## Five axes

- **Correctness**: the change does what the spec asks; edge and error paths are handled; each test checks behaviour against an independent oracle.
- **Readability**: names reveal intent; control flow is straightforward; the change is as short as the problem allows.
- **Architecture**: the change follows existing patterns and module boundaries, reuses the canonical helper, and keeps feature logic in its owning module.
- **Security**: untrusted input is validated at the boundary; secrets stay out of code and logs.
- **Performance**: work in hot paths stays bounded as input grows.

For a structural finding, name the remedy: collapse duplicate branches, replace a conditional chain with a typed model, move logic to its owning module, delete a pass-through wrapper, or split a file.

## Severity

| Label | Meaning |
| --- | --- |
| **Blocks** | A spec requirement, proof row or mandatory criterion is unmet, a documented standard is broken, or behaviour, data or security is at risk. |
| **Follow-up** | A real issue outside the slice boundary. Listed for a later ticket; this change proceeds. |
| **Nit** / **FYI** | Optional or informational. |

A few high-conviction findings beat a long list. Give the evidence for each finding and quantify it where you can.

**Verdict:** `Approve` when nothing blocks; otherwise `Request changes`, listing the blocking findings.
