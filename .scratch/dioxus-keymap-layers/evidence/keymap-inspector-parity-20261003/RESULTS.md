# Keymap Inspector placement/spacing paired journey — 2026-10-03

**Scope:** only the F6K.4c Inspector composition change integrated in source `3a080e0e99d0638ddc1b562b3fc5a8ac91f3fefd`: editor-before-export ordering, full-width primary action, local-source note and Inspector spacing. This does not repeat binding, macro-history, firmware-delivery or F6K parent qualification.

**Oracle/candidate:** React `5a472a9426e6e38993361da402cd4ec730feb369` at `http://127.0.0.1:5173/`; packaged Dioxus `http://127.0.0.1:34767/boardstudio/`, reported source `3a080e0e99d0638ddc1b562b3fc5a8ac91f3fefd`, provenance SHA-256 `96d7bc6289aedaadd8198bb9038c67d24fc659fde847362b24c7504313cbc733`. Separate named `agent-browser` worktree sessions were used for React and Dioxus, both at 1280×940, Light. Both imported the exact same saved archive, SHA-256 `5b17071a819e4cfa28531913685e8fedf84cbdd361cefccb3b16befd6c0776df`.

## Results

| Active editor | Result |
| --- | --- |
| Keys | In both apps, Layers and the key editor precede the full-width blue primary “Export ZMK source” button and the local-source helper. Candidate ordering and button treatment match. |
| Macros | In both apps, macro guidance and Add macro precede Export and the helper. Candidate preserves ordering and action; the screenshot shows it remains visible after the editor. |
| Encoders | In both apps, the encoder editor is before the same Export action in document order. Its longer content pushes Export below the 940 px viewport in both apps, so this pass did not claim the button was visually in-frame for this state. |

The candidate keeps the intended content order and primary-action treatment, but vertical rhythm still differs from React in the inspected Keys/Macros views. In the 1280×940 Macros screenshots, approximate top coordinates are: Layers summary 131 px candidate / 148 px React; Layer name 346 / 373; editor tabs 484 / 516; Macros editor header 548 / 589; Add macro 638 / 676; Export 698 / 740; helper note 742 / 784. The candidate stacks this Inspector content about 17–42 px higher. Panel left inset is effectively aligned (984–985 px). This is a remaining visual refinement in the existing F6K.4c composition ticket, not a missing control or behavior. No new ticket is proposed here.

I switched between Keys, Macros, Encoders and Layout only. No export was invoked and no project edit was made. Both archive uploads started from the same SHA; the screenshots show the loaded Sofle v2 project and the same 29-key Keymap projection. This is focused visual evidence, not full F6K acceptance or a saved-document round trip.

## Captures

- Keys: [React](screenshots/react-keys.png) / [Dioxus](screenshots/dioxus-keys.png)
- Macros: [React](screenshots/react-macros.png) / [Dioxus](screenshots/dioxus-macros.png)
- Encoders: [React](screenshots/react-encoders.png) / [Dioxus](screenshots/dioxus-encoders.png)

The screenshots are direct 1280×940 captures. SHA-256: React Keys `88b5b6affd470833bec0826c407bcdc2023a0327c2e44b5d6b2f2814d6f634c8`; Dioxus Keys `82f62027d48cbe04ef7769d4defd8c4a98840d2fc1e0eb2254b179782aa45036`; React Macros `5c8eb5975cbe415f158fd9530c5c20e5dd74befdd9d7bd4bb14e574ebdd41607`; Dioxus Macros `99d2d974225a4ae4b399aa6b33a820c693bcefe70561250ad1ee7f7e11136a23`; React Encoders `64379aba7b7ff1c66238f41b266bd0048e95f0a82ca6a3c9e3bca49c965ffc24`; Dioxus Encoders `1bd831a8d2ee42944a49e10dfbe0d87c7178372892596272d37a2ed2a596153c`.
