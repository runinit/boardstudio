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
