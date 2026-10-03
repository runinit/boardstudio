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

The measured summary-spacing difference is addressed in isolated source commit `55692acc5c17dbd5c863a5a898bb1550372771d9`: the Dioxus Keymap Layers/editor disclosures now use the same `.6rem` vertical summary margin as React's `app/src/ui/keymap.css`. That source commit has only a whitespace/rhythm CSS change and `git diff --check` passed. It is not yet in the captured 34767 package; rerun only this changed Inspector spacing journey on the next packaged candidate before marking the refinement verified.

I switched between Keys, Macros, Encoders and Layout only. No export was invoked and no project edit was made. Both archive uploads started from the same SHA; the screenshots show the loaded Sofle v2 project and the same 29-key Keymap projection. This is focused visual evidence, not full F6K acceptance or a saved-document round trip.

## Captures

- Keys: [React](screenshots/react-keys.png) / [Dioxus](screenshots/dioxus-keys.png)
- Macros: [React](screenshots/react-macros.png) / [Dioxus](screenshots/dioxus-macros.png)
- Encoders: [React](screenshots/react-encoders.png) / [Dioxus](screenshots/dioxus-encoders.png)

The screenshots are direct 1280×940 captures. SHA-256: React Keys `88b5b6affd470833bec0826c407bcdc2023a0327c2e44b5d6b2f2814d6f634c8`; Dioxus Keys `82f62027d48cbe04ef7769d4defd8c4a98840d2fc1e0eb2254b179782aa45036`; React Macros `5c8eb5975cbe415f158fd9530c5c20e5dd74befdd9d7bd4bb14e574ebdd41607`; Dioxus Macros `99d2d974225a4ae4b399aa6b33a820c693bcefe70561250ad1ee7f7e11136a23`; React Encoders `64379aba7b7ff1c66238f41b266bd0048e95f0a82ca6a3c9e3bca49c965ffc24`; Dioxus Encoders `1bd831a8d2ee42944a49e10dfbe0d87c7178372892596272d37a2ed2a596153c`.

## Packaged spacing retest — 34768

The same isolated React profile and 1280×940 Light browser capture were repeated against Dioxus package `frontend-contextual-panes-followup-20261003`, source `9e6f9b62e33d7dc22412cf5793a3f4b857e219c3`, provenance SHA-256 `0d04905a8dbd66e961595fbae3702124e2f7d2784cfbe5bc3e8dab85dc81d290`. Both apps used the same original layered Sofle fixture, SHA-256 `5b17071a819e4cfa28531913685e8fedf84cbdd361cefccb3b16befd6c0776df`.

In the Keys and Macros captures, the former 17–42 px cumulative vertical offset is gone: layer heading/rows, editor tabs, selection controls, Add macro and Export now sit at the same approximate vertical positions. The same editor-before-action order remains. The Encoders captures retain the editor before Export in both apps, with the action below the viewport because the expanded content is taller. Dioxus encoder disclosures still use bordered cards and wider vertical padding where React uses flatter disclosure rows. That is a remaining presentation detail outside the `.6rem` summary-margin correction; this focused pass did not change it.

This supersedes the previous paragraph's statement that the spacing was still unverified on a built package. Captures: `34768-react-keys.png`, `34768-dioxus-keys.png`, `34768-react-macros.png`, `34768-dioxus-macros.png`, `34768-react-encoders.png`, `34768-dioxus-encoders.png`. Screenshots remain direct 1280×940 browser captures.
