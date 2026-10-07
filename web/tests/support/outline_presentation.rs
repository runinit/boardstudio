pub mod objects {
    use boardstudio_application::Scope;
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum TreeContext {
        Component {
            part_id: String,
        },
        Outline {
            board_id: String,
        },
        OutlineVersion {
            board_id: String,
            version_id: Option<String>,
        },
    }
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct ScopedTreeContext {
        pub scope: Scope,
        pub context: TreeContext,
    }
}
pub mod selection {
    use super::objects::TreeContext;
    pub fn context_is_current(
        model: &boardstudio_application::ReadModel,
        scope: &boardstudio_application::Scope,
        context: &TreeContext,
    ) -> bool {
        let board = match context {
            TreeContext::Component { .. } => return false,
            TreeContext::Outline { board_id } | TreeContext::OutlineVersion { board_id, .. } => {
                board_id
            }
        };
        model.active_board_id == scope.board_id && board == &scope.board_id
    }
}
#[path = "../../src/presentation/outline_lifecycle.rs"]
mod outline_lifecycle;
