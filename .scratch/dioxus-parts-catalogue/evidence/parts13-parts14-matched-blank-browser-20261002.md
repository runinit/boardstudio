# Parts13/Parts14 matching blank-project browser journey — 2026-10-02

This bounded cross-frontend journey used the same newly created blank-project fixture, named `Parts13 Browser Proof Native`, in both clients. It is a matched fixture, not a shared imported archive: each app has an independent local store. It does not claim public acceptance or Parts15 parity.

## Builds and sessions

- Candidate Dioxus app: `http://127.0.0.1:34732`, integrated source build `9d34f367` (Parts13 and Parts14 source; predates Parts15).
- React app: `http://127.0.0.1:5173`.
- Browser sessions: `parts13-15-native` and `parts13-15-react`.

## Parts13 create/name/history/reopen

In both apps, created a blank project with the same name, opened Parts, created a custom component, and renamed it to `Parts13 paired custom component`. Undo restored the generated name (`Custom component 1`), Redo restored the chosen name, and reload/reopen retained the final name in the catalogue.

## Parts14 accepted profile/history/reopen

In both apps, selected the Choc V1/V2 switch, opened Define profile, edited the profile, and saved. Undo reverted the accepted profile to Define profile, Redo restored Edit profile, and reload/reselection retained Edit profile.

The operations were bounded to what the integrated builds offered: Dioxus saved a manual 2.2 mm gap plus one cutout, while React selected Choc v1 and loaded its standard cutout before saving. The Dioxus build predates issue15, so the different standard-family workflow is not a parity claim. Both paths exercised accepted edit, history, and persistence on the same blank-project fixture.

## Reviewed context references

- Shared reviewed workflow spec: `/home/chris/.local/share/boardstudio/worktrees/parts-standard-profile15-mounted-regressions-20261002/.scratch/dioxus-parts-catalogue/specs/contextual-workflows-reviewed.md`, SHA-256 `f739b30e6e4fb5aa743ccfd0be3b069a81abd701d96297857a0251bfa48f9884`.
- Parts13 current status-only ticket: `/home/chris/.local/share/boardstudio/worktrees/parts-definition-name-20261002/.scratch/dioxus-parts-catalogue/drafts/13-create-custom-component.md`, SHA-256 `c50934a967017604ad57a3144d43f2c2cf4be3af028ebf1892fa23a4a85673c3`; reviewed ticket predecessor SHA-256 `81b7df372a7d0b0db907ece02748986604e12b0632a94f4e428d8984d5e9483b`.
- Parts14 issue: `/home/chris/.local/share/boardstudio/worktrees/parts-fit-profile-14-20261002/.scratch/dioxus-parts-catalogue/drafts/14-edit-manual-fit-profile.md`, SHA-256 `ac1a6bad76207504be0a90f32da894f25b84acf229ca24bc09d9f0ad605a5c41`.
- Parts13 integrated-start capability receipt: `.scratch/dioxus-parts-catalogue/evidence/parts13-integrated-start-capability-20261002.json`, SHA-256 `48f5cba3393681fe818b4b6c2a61590d667d145c593d1ed2f3b42b4511e8e114`.
- Parts14 start capability receipt: `.scratch/dioxus-parts-catalogue/evidence/parts14-start-capability-20261002.md`, SHA-256 `384b052c7a4a4c7f9b20cdd10eec1a0f926bba65ceb0afb6c4fce9a6650620c6`.
