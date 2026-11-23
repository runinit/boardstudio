//! Shared browser mounting and Editor fixtures for the page tests.
use super::*;
use boardstudio_core::model::ProjectDoc;
use boardstudio_web_test_runtime::project_name_test_support as support;
use boardstudio_web_ui_model::Workspace;
use futures_util::future::{AbortHandle, Abortable};
use serde_json::json;
use std::{cell::RefCell, ops::Deref};

pub(super) mod canvas;

pub(super) struct MountedRoot {
    root: web_sys::Element,
    renderer: AbortHandle,
}

impl Deref for MountedRoot {
    type Target = web_sys::Element;

    fn deref(&self) -> &Self::Target {
        &self.root
    }
}

impl Drop for MountedRoot {
    fn drop(&mut self) {
        self.renderer.abort();
        self.root.remove();
    }
}

pub(super) fn mount(dom: VirtualDom, id: Option<&str>) -> MountedRoot {
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    if let Some(id) = id {
        root.set_id(id);
    }
    document.body().unwrap().append_child(&root).unwrap();
    let config = dioxus_web::Config::new().rootnode(root.clone().into());
    let (renderer, registration) = AbortHandle::new_pair();
    wasm_bindgen_futures::spawn_local(async move {
        let _ = Abortable::new(dioxus_web::run(dom, config), registration).await;
    });
    MountedRoot { root, renderer }
}

pub(super) async fn open_document(document: ProjectDoc) -> Rc<Runtime> {
    let runtime = support::new_runtime();
    support::open_document(&runtime, document).await;
    runtime
}

pub(super) fn mount_editor(root_id: &str, runtime: Rc<Runtime>) -> (EditorProbe, MountedRoot) {
    let probe = EditorProbe {
        runtime: runtime.clone(),
        adapter: Rc::default(),
        version: Rc::default(),
    };
    let dom = VirtualDom::new(editor_host);
    dom.provide_root_context(runtime);
    dom.provide_root_context(probe.clone());
    (probe, mount(dom, Some(root_id)))
}

#[derive(Clone)]
pub(super) struct EditorProbe {
    pub(super) runtime: Rc<crate::runtime::Runtime>,
    adapter: Rc<RefCell<Option<SelectionAdapter>>>,
    pub(super) version: Rc<RefCell<Option<Signal<u64>>>>,
}

impl EditorProbe {
    pub(super) fn select(&self, context: objects::TreeContext) {
        selection::submit_context(
            &self.runtime,
            &self.adapter.borrow().clone().expect("mounted adapter"),
            selection::ContextRequest {
                scope: self.runtime.scope().expect("opened project scope"),
                context,
                mode: boardstudio_application::SelectionMode::Replace,
            },
        );
    }

    pub(super) fn revision(&self) -> u64 {
        self.runtime
            .model()
            .accepted
            .expect("accepted document")
            .document
            .revision
    }
}

fn editor_host() -> Element {
    let probe = use_context::<EditorProbe>();
    let workspace = use_signal(|| Workspace::Layout);
    let version = use_signal(|| 0u64);
    *probe.version.borrow_mut() = Some(version);
    let selected = use_signal(|| None);
    let anchor = use_signal(|| None);
    let generation = use_signal(|| 0u64);
    let adapter = SelectionAdapter::new(selected, anchor, generation);
    *probe.adapter.borrow_mut() = Some(adapter.clone());
    use_editor_context(workspace, version, adapter);
    rsx! { boardstudio_web_ui_shared::ui::toast::ToastProvider { Editor {} } }
}

pub(super) fn use_editor_context(
    workspace: Signal<Workspace>,
    version: Signal<u64>,
    adapter: SelectionAdapter,
) {
    let _ = layout_input::use_layout_input();
    use_context_provider(|| WorkspaceState(workspace));
    use_context_provider(|| ExportReturnWorkspace(workspace));
    use_context_provider(|| version);
    use_context_provider(|| adapter);
    let created = use_signal(|| None::<SetupGuideRequest>);
    use_context_provider(|| created);
    let objects_open = use_signal(|| true);
    let inspector_open = use_signal(|| true);
    use_context_provider(|| CompactPanelState {
        objects_open,
        inspector_open,
    });
    let warning = use_signal(|| false);
    use_context_provider(|| PreferenceStorageWarning(warning));
    let theme = use_signal(|| "light");
    use_context_provider(|| ThemeState(theme));
    use_context_provider(|| AppearanceState(theme));
    let resolved = use_memo(|| "light");
    use_context_provider(|| ResolvedAppearance(resolved));
    let hidden = use_signal(BTreeSet::new);
    let modules_hidden = use_signal(BTreeSet::new);
    let footprints = use_signal(|| false);
    use_context_provider(|| LayerVisibility {
        hidden,
        modules_hidden,
        footprints,
    });
    boardstudio_web_ui_model::state::use_test_case_generation_state();
}

pub(super) fn matrix_part_definition(
    id: &str,
    kind: &str,
    courtyard: [[f64; 2]; 4],
) -> serde_json::Value {
    json!({
        "id": id, "name": id, "kind": kind,
        "courtyard": courtyard.map(|[x, y]| json!({ "x": x, "y": y })),
        "pads": [{ "id": "p0", "number": "1", "at": { "x": 0.0, "y": 0.0 },
                   "size": { "x": 2.0, "y": 2.0 }, "shape": "rect" }],
    })
}
