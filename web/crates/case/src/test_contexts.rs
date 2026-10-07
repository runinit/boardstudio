//! Context providers that mounted Case tests, here and in the page, wrap their components in.
use crate::runtime::Runtime;
use crate::{
    InstanceSelection, ResolvedTheme, WorkspaceState, case_viewer, mechanical_settings_mount,
    objects,
};
use boardstudio_application::Scope;
use boardstudio_web_ui_model::selection::SelectionAdapter;
use dioxus::prelude::*;
use std::rc::Rc;

pub fn use_case_viewer_test_contexts() {
    let workspace = use_signal(|| "Case");
    use_context_provider(|| WorkspaceState(workspace));

    let selected_context = use_signal(|| None::<objects::ScopedTreeContext>);
    let anchor_scope = use_signal(|| None::<Scope>);
    let scope_generation = use_signal(|| 0u64);
    let adapter =
        use_hook(|| SelectionAdapter::new(selected_context, anchor_scope, scope_generation));
    use_context_provider(|| adapter);

    let case_selection = case_viewer::CaseSelection {
        body: use_signal(|| None::<case_viewer::BodySelection>),
        layer: use_signal(|| None::<case_viewer::LayerSelection>),
        display: use_signal(std::collections::BTreeMap::new),
        body_edit_portal: case_viewer::CaseBodyEditPortal {
            dispatch: use_signal(|| None::<case_viewer::CaseBodyEditDispatch>),
            editable: use_signal(|| false),
        },
    };
    use_context_provider(|| case_selection);

    let theme = use_memo(|| "light");
    use_context_provider(|| ResolvedTheme(theme));
}

// Assemble the real mechanical owner hook's contexts for mounted Case tests.
// The readiness predicate remains owned by mechanical_settings_mount.
pub fn use_case_generation_readiness_test_bridge(runtime: Rc<Runtime>) -> bool {
    let adapter = use_context::<SelectionAdapter>();
    let WorkspaceState(workspace) = use_context::<WorkspaceState>();
    let instance_selection = use_context::<InstanceSelection>();
    let case_selection = use_context::<case_viewer::CaseSelection>();
    mechanical_settings_mount::use_mechanical_settings_mount(
        runtime,
        adapter.generation,
        workspace,
        instance_selection,
        case_selection,
        EventHandler::new(|_| {}),
    )
    .generation_ready
}
