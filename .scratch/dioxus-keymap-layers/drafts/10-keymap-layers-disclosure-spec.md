# Keymap Layers disclosure and Inspector controls

## Problem statement

In the paired Keymap workspace, React presents “Layers” as an expanded-by-default disclosure. Its summary can collapse or reopen the layer list and its controls. The Dioxus Inspector renders “Layers” as a static heading, so users cannot reclaim that vertical space. Dioxus also displays a Base-layer protection sentence that the reference does not show, and its layer-control spacing differs from the reference.

## Solution

Match the reference Layers section inside the existing Keymap Inspector: use an accessible native disclosure named “Layers”, open on initial mount, and keep its chosen open/closed state through ordinary Keymap updates. Keep the existing layer-selection and layer-operation behavior inside the disclosure. Remove the extra Base protection sentence; continue to omit the Remove layer action for Base. Toggling the disclosure remains view state and does not change the project document, revision, or history.

## User stories

1. As a Keymap user, I want to see the layer list when I first open the Inspector, so I can immediately identify the active layer.
2. As a Keymap user, I want to collapse the Layers section, so the binding or macro controls below it have room in the Inspector.
3. As a keyboard user, I want to open and close Layers with the native disclosure keyboard interaction and hear its expanded state, so the control remains accessible.
4. As a user who selects another layer or receives a routine accepted-document refresh, I want my disclosure choice to remain in effect, so ordinary Keymap updates do not interrupt my work.
5. As a user viewing Base, I want only actions that can be performed to be shown, without an extra explanatory protection sentence, matching the reference.
6. As a user viewing another layer, I want its existing rename and removal controls to remain available when Layers is expanded.
7. As a user collapsing or opening Layers, I want the change to affect only the Inspector presentation, not saved project data, revision, or undo history.

## Implementation decisions

- Keep the feature within the existing private Keymap Inspector component and its local presentation styles. Do not widen public or crate APIs or add document state.
- Render “Layers” as a semantic disclosure with a visible summary and native keyboard behavior. It is expanded on mount. User-toggled state is not reset by ordinary layer selection, accepted projection updates, or rerenders; a fresh panel mount returns to the reference default-open state.
- The disclosure contains the current layer list, Add layer, Layer name, the permitted Remove layer action, feedback associated with layer operations, and the existing precedence/transparency help text. Collapsing hides this content as one section.
- Preserve existing accepted layer identity, operation callbacks, loading/feedback behavior, validation, and edit/history semantics. For Base, omit Remove layer and the Dioxus-only “The first layer cannot be removed.” status paragraph.
- Tune only the Layers disclosure/control spacing needed to match the pinned React Inspector at the paired fixture viewport. Do not alter shared workspace chrome or other Keymap tabs.
- The paired source comparison uses the pinned React build and the actual Dioxus candidate already recorded in the audit. Screenshots are visual evidence, not evidence of behavior that was not exercised.

## Testing decisions

- Exercise the production Keymap Inspector, not a parallel disclosure mock. Verify the default-open state, collapse/reopen, accessible summary/state, and hidden descendants.
- Change the selected layer and refresh the accepted projection while collapsed; assert the section remains collapsed and layer state remains correct.
- Assert that Base omits both the Remove layer button and the extra protection sentence, while a non-base layer retains its existing rename/remove controls.
- Compare before/after project revision and undo history around disclosure toggles; they must be unchanged.
- Run a paired browser check against the same layered archive and compare only the owned Layers Inspector section. Other screenshot differences are recorded as separate work.

## Out of scope

- Workspace context bar, 2D/3D assembly/Footprints view controls, shared Objects navigation/panel controls, responsive shell, and the global footer; these belong to existing shared-shell, viewer, Objects, and footer work.
- Keymap projection, layer-selection semantics, add/rename/remove operation behavior, binding/macro/encoder editors, firmware export, or broader F6K.1/F6K.2/F6K.4 acceptance.
- Changes to task-graph edges, parent status, shared selection architecture, persistent schema, or public APIs.

## Further notes

- Parent: F6K.1. This is a narrow visual/accessibility child based on the paired same-fixture observation. The existing parent and F3.1/shared-selection acceptance joins remain open.
- Pinned React commit: `5a472a9426e6e38993361da402cd4ec730feb369`. Relevant React source hashes: `KeymapPanel.tsx` `60c062945d563db25605b327edf71c43cc5fb339`; `InspectorSection.tsx` `d4645c517d2013730d48fab1838f70fd9dc3d876`; `keymap.css` `1b65a4290b7739cb822b2c29700875b7a69d278b`.
- Screenshot candidate: Dioxus source commit `b9e74e37fc34948be9fee97c918b644200778bca`, build `frontend-layout-case-transport-integrated-20261002`, served at `http://127.0.0.1:34737/` and `/boardstudio/`. Its Keymap panel source hash is `c03740226a058d960858f2027cfea33ad2aa4db96438d0859d92df300a6987dc`; its stylesheet hash is `096f24e191223580878d00d9a34458b0a54f031b2f1ef4ca357413f8ef756977`.
- Same layered fixture SHA-256: `5b17071a819e4cfa28531913685e8fedf84cbdd361cefccb3b16befd6c0776df`.
- Reference screenshot SHA-256: `b0a0f662fd1f163fd65e94ee334252e5ecbfa19649d0c59bf650f607bb1f9f14`; Dioxus screenshot SHA-256: `3f287666ccc38df2373e38b6e0d1cf76d08c7ce820b51155b1c348a83c8237fa`.
- No public/browser behavior acceptance or parent completion is claimed by this spec.
