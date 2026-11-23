use crate::{
    Workspace,
    runtime::Runtime,
    state::{CompactPanelState, InstanceSelection, LayerVisibility},
};
use dioxus::prelude::*;
use std::rc::Rc;

/// Signals shared by the Editor and the workspace components it mounts.
#[derive(Clone, Copy)]
pub struct EditorContext {
    pub runtime: CopyValue<Rc<Runtime>>,
    pub version: Signal<u64>,
    pub workspace: Signal<Workspace>,
    pub scope_generation: Signal<u64>,
    pub instance_selection: InstanceSelection,
    pub panels: CompactPanelState,
    pub layers: LayerVisibility,
    pub return_workspace: Signal<Workspace>,
}

impl EditorContext {
    pub fn runtime(self) -> Rc<Runtime> {
        self.runtime.peek().clone()
    }
}

#[derive(Clone, Copy)]
pub struct EditGuard {
    pub editor: EditorContext,
    pub workspace: Workspace,
}

impl EditGuard {
    pub fn workspace_is_current(self) -> bool {
        (self.editor.workspace)() == self.workspace
    }

    pub fn route_is_current(self, generation: u64) -> bool {
        self.workspace_is_current() && (self.editor.scope_generation)() == generation
    }

    pub fn instance_is_current(self, runtime: &Runtime) -> bool {
        runtime.with_model(|model| self.editor.instance_selection.is_current(model))
    }
}

/// Run owner-specific admission after checking the shared workspace and instance route.
/// The caller checks the live snapshot and its durability according to the edit's policy.
pub fn use_guarded_edit<T: 'static>(
    guard: EditGuard,
    mut edit: impl FnMut(&Rc<Runtime>, T) + 'static,
) -> EventHandler<T> {
    let _ = (guard.editor.version)();
    use_callback(move |request| {
        if !guard.workspace_is_current() {
            return;
        }
        let runtime = guard.editor.runtime();
        if guard.instance_is_current(&runtime) {
            edit(&runtime, request);
        }
    })
}
