# Spec: dioxus-presentation milestone contracts

Status: Phase 1 authorized in advance; technical validation required, 2026-10-01.
Module id: `dioxus-presentation`; base `96dd51d3`. Continue the
[approved map](CAPABILITY-MAP.md) and [ADR 0003](docs/adr/0003-rust-application-ownership.md).

## Assumptions and objective

Use Dioxus **0.7.10**, matching CLI **0.7.10**, Rust 1.98.0 and explicitly
minimal/web/mounted features. Browser bindings stay at the accepted pins.
No native renderer, SSR/server, new design system or React/Dioxus co-writer
is selected. The shared web package separates components from host modules.

Expose the copied-project M1 workflow with existing visual/interaction language:
open, select/drag/numeric preview, Undo/Redo/save/archive/reload, case setting,
exact 3D inspection and committed STEP. Existing desktop/compact layout,
keyboard/focus/labels/theme behavior is the parity reference. Scope omissions
must be truthful; M1 is not complete workspace or full-Rust parity.

## Presentation contracts

| Surface | Responsibility and invariant |
| --- | --- |
| Application root | Holds/subscribes to the application session supplied by host composition. Its lifetime outlasts replaceable panels; session durability does not live in a panel resource. |
| Read-model subscriptions | Render immutable accepted/display snapshots and status. Signals are subscribed/derived presentation handles, never a second writable document/history/session. Granularity and clone/update costs require measurement. |
| Forms and controls | Own text formatting, validation display, focus and panel visibility. Submit typed session intentions for domain preview/commit; session owns their transaction. Show domain/storage/transport outcomes distinctly. |
| SVG/canvas surfaces | Use rendering mappings and host attachment/event ports. Host owns DOM/capture/observers/RAF/GPU attachment; session owns pointer/gesture policy. No component recomputation of domain geometry or readiness. |
| Generation and export status | Distinguish current exact, provisional, paused/stale, blocked, failed and cancelled. Render recovery with retry and explicit abandonment consequences; no export action can bypass durable/readiness guards. |
| Disposal | Unsubscribe/remove local presentation resources on panel unmount. Explicit session/host teardown settles jobs and disposes resources at application close. Framework future cancellation alone does not cancel a dispatched worker job. |

Preserve one final gesture commit and final pointer sample. Escape, pointercancel,
scope replacement, unrelated-pointer guards, snap/Alt and modifier selection
must match characterized reference behavior. Numeric text drafts do not write
a document signal directly. Clipboard/file/navigation behavior stays with host
ports; provider errors retain meaning rather than disappearing into generic UI
success.

## Versioned sources, rationale and tradeoffs

Official [Dioxus 0.7.10 manifest](https://raw.githubusercontent.com/DioxusLabs/dioxus/v0.7.10/packages/dioxus/Cargo.toml)
and [tagged mounted-event implementation](https://raw.githubusercontent.com/DioxusLabs/dioxus/v0.7.10/packages/web/src/events/mod.rs)
show that mounted-element conversion needs the explicit `mounted` feature with
minimal/web. [Tagged signals](https://github.com/DioxusLabs/dioxus/blob/v0.7.10/packages/signals/README.md)
and [resource implementation](https://raw.githubusercontent.com/DioxusLabs/dioxus/v0.7.10/packages/hooks/src/use_resource.rs)
support presentation subscriptions/reactive reruns, not engine/durability
authority or external CPU cancellation. Copying a signal handle does not clone
its document. [Tagged core task lifetime](https://raw.githubusercontent.com/DioxusLabs/dioxus/v0.7.10/packages/core/src/global_context.rs)
supports the explicit distinction between component and application lifetimes.

The observed problem is presentation/reactivity currently coupling to session
authority. A subscribed Rust session separates correctness from Dioxus lifetime
behavior. Keeping all mutable state in signals is convenient but creates a
second authority; a monolithic document signal may also broaden updates/copies.
Fine-grained subscriptions add interface work and are selected only with
measurements. Exact-version source verification justifies the APIs, not runtime
compatibility. 0.8 prerelease is not selected and an older patch has no evidenced
advantage; this turn's official release check retains 0.7.10.

Preserve [existing workbench styles](app/src/ui/workbench.css) and
[shared layout styles](app/src/ui/unified-workbench.css), labels, focus order,
responsive layout and domain gestures. No intentional visual redesign is
bundled with the migration. New save-recovery UI reflects the approved behavior
change and needs an explicit corrected oracle alongside unaffected parity.

## Structure and code style

Components/presentation drafts belong in web presentation modules; host adapters
remain distinct in the same package. Only this layer depends on Dioxus; do not
add Dioxus/web-sys to the headless session/provider crates. Public UI intents
consume session contracts. No application crate or component is created here.

Follow Rust formatting/naming and typed read models. This actual
[provider record excerpt](renderer/src/lib.rs) illustrates explicit optional
data rather than browser handles; it is not a new Dioxus component:

```rust
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelMesh {
    pub positions: Vec<f32>,
    pub normals: Vec<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub colors: Option<Vec<f32>>,
}
```

Use exact-version documented event/mount APIs when P2 implements its isolated
probe. Canvas downcast and pointer event behavior remain compile/runtime tests.

## Commands and testing strategy

Existing reference commands:

```sh
pnpm run build
pnpm run test:e2e
pnpm run test:e2e:dev
pnpm run test:e2e:pages
pnpm --dir app exec playwright test e2e/canvas-interactions.spec.ts e2e/workbench-selection.spec.ts e2e/project-library.spec.ts e2e/startup-recovery.spec.ts
```

After an isolated Dioxus probe/package exists, tagged CLI target/build sources
support these prospective commands from its package directory:

```sh
dx --version
dx build --web --release --base-path / --cargo-args="--locked"
dx build --web --release --base-path /boardstudio/ --cargo-args="--locked"
```

These are not existing application gates or passing builds. Verify installed
help/output and retain deployment-specific artifacts; source references:
[CLI build](https://raw.githubusercontent.com/DioxusLabs/dioxus/v0.7.10/packages/cli/src/cli/build.rs),
[target arguments](https://raw.githubusercontent.com/DioxusLabs/dioxus/v0.7.10/packages/cli/src/cli/target.rs).
P2 needs real SVG/canvas lifecycle tests and P1/P3 supply packaging/durability.
Paired M1 actions/geometry/history/errors, controlled screenshots, keyboard/
responsive checks and frozen performance/resource budgets remain required under
[CONSTRAINTS.md](CONSTRAINTS.md). New visual/a11y/coverage gates are not activated
or assumed to exist; document missing evidence rather than declare parity.

## Success criteria

- U1: Components submit intentions and render session snapshots without a second
  writable document/history/session or loss of supported fixture data.
- U2: Characterized selection/drag/numeric/keyboard gestures, final sample and
  one-step Undo/Redo work through Dioxus with correct save acknowledgment.
- U3: Current/provisional/stale/blocked/failed states and save recovery are truthful;
  readiness/durability cannot be bypassed by controls.
- U4: Panel rerun/unmount cannot cancel a durable transaction or publish stale
  jobs; application disposal cleans subscriptions, frames and GPU resources.
- U5: Existing labels/focus/themes/compact layout and affected visuals satisfy
  paired acceptance; copy/update/paint/resource measurements retain their budgets.
- U6: Root/subpath production initialization, archive/reload and committed STEP
  work with the host/provider contracts; no complete-migration claim follows M1.

## Boundaries and open validation

Always preserve approved scope/styles/contracts and version-match source facts.
Ask before a framework/version/platform change, visual redesign or runtime bridge
exception. Never move saves into replaceable resources, assume Dioxus cancellation
terminates workers, or delete React before replacement gates pass.

P0 is dependency resolution only. CLI installation is a tool result; worker,
canvas, offline, paired behavior and measured production performance remain
separate gates. No prototype code is promoted automatically.
