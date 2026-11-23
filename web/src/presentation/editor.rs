use super::workbench::editor_session::EditorSession;
use super::*;
use boardstudio_web_ui_model::editor::EditorContext;
#[component]
pub(super) fn Editor() -> Element {
    let runtime = use_context::<Rc<Runtime>>();
    let version = use_context::<Signal<u64>>();
    let _ = version();
    let snapshot = runtime.with_model(|model| model.accepted.clone());
    let (Some(scope), Some(snapshot)) = (runtime.scope(), snapshot) else {
        return rsx! {};
    };
    rsx! { EditorBody { scope, snapshot } }
}

#[component]
fn EditorBody(scope: Scope, snapshot: AcceptedSnapshot) -> Element {
    let runtime = use_context::<Rc<Runtime>>();
    let inst_pref = use_signal(|| {
        runtime.scope().and_then(|s| {
            s.instance_id
                .map(|explicit_id| instance_selection::Preference {
                    session_epoch: s.session_epoch,
                    document_id: s.document_id,
                    explicit_id,
                })
        })
    });
    let instance_selection = use_context_provider(|| InstanceSelection(inst_pref));
    let editor = EditorContext {
        runtime: use_hook(|| CopyValue::new(runtime)),
        version: use_context::<Signal<u64>>(),
        workspace: use_context::<WorkspaceState>().0,
        scope_generation: use_context::<SelectionAdapter>().generation,
        instance_selection,
        panels: use_context::<CompactPanelState>(),
        layers: use_context::<LayerVisibility>(),
        return_workspace: use_context::<ExportReturnWorkspace>().0,
    };
    use_context_provider(|| editor);
    rsx! { EditorSession { scope, snapshot } }
}
