use super::*;

#[derive(Clone, Copy)]
pub(crate) struct WorkspaceRoutingSlots {
    pub(crate) select_tree: EventHandler<objects::TreeSelectRequest>,
    pub(crate) navigate: EventHandler<(Scope, String, Option<String>)>,
}
