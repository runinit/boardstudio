//! Parts-owned Objects, canvas placeholder and Inspector composition.
use super::parts::{PartsInspectorPanel, PartsLibraryPanel, PartsPreviewWorkspace};
use super::parts::{PartsQuery, PartsSelection};
use super::workspace_composition::PlaceholderInput;
use boardstudio_application::{AcceptedSnapshot, Scope};
use dioxus::prelude::*;

/// State shared by the Parts library, preview, placement and assembly editors.
/// Editor keeps the handle to compose these views; catalogue state and its
/// selection epochs remain owned here.
#[derive(Clone, Copy)]
pub(super) struct PartsWorkspaceState {
    query: PartsQuery,
    selected: PartsSelection,
    assembly_selection: Signal<Option<super::parts::MatrixPresetId>>,
    assembly_orientation: Signal<super::parts::SwitchOrientation>,
    selection_generation: Signal<u64>,
    preview_activation: Signal<u64>,
    generator_draft: Signal<Option<super::parts::GeneratorPreviewDraft>>,
}

pub(super) fn use_parts_workspace_state() -> PartsWorkspaceState {
    let state = PartsWorkspaceState {
        query: use_signal(String::new),
        selected: use_signal(|| None),
        assembly_selection: use_signal(|| None),
        assembly_orientation: use_signal(|| super::parts::SwitchOrientation::South),
        selection_generation: use_signal(|| 0),
        preview_activation: use_signal(|| 0),
        generator_draft: use_signal(|| None),
    };
    use_context_provider(|| super::parts::PartsAssemblySelection(state.assembly_selection));
    use_context_provider(|| super::parts::PartsAssemblyOrientation(state.assembly_orientation));
    use_context_provider(|| super::parts::PartsSelectionGeneration(state.selection_generation));
    use_context_provider(|| super::parts::PartsPreviewActivation(state.preview_activation));
    use_context_provider(|| super::parts::GeneratorDraftStore(state.generator_draft));
    state
}

impl PartsWorkspaceState {
    pub(super) fn query(self) -> PartsQuery {
        self.query
    }

    pub(super) fn selected(self) -> PartsSelection {
        self.selected
    }

    pub(super) fn assembly_orientation(self) -> Signal<super::parts::SwitchOrientation> {
        self.assembly_orientation
    }

    pub(super) fn selection_generation(self) -> Signal<u64> {
        self.selection_generation
    }

    /// Record explicit selection of a mounted module and expire callbacks or
    /// previews tied to the prior Parts selection.
    pub(super) fn select_mounted_module(mut self, scope: Scope, definition_id: &str) {
        self.selected
            .set(Some((Some(scope), format!("module:{definition_id}"))));
        self.selection_generation
            .with_mut(|generation| *generation = generation.wrapping_add(1));
        self.preview_activation
            .with_mut(|activation| *activation = activation.wrapping_add(1));
    }
}

pub(super) struct ObjectsInput {
    pub(super) snapshot: AcceptedSnapshot,
    pub(super) scope: Option<Scope>,
    pub(super) scope_generation: Signal<u64>,
    pub(super) workspace: Signal<&'static str>,
    pub(super) query: PartsQuery,
    pub(super) selected: PartsSelection,
    pub(super) on_select: EventHandler<()>,
}

pub(super) struct InspectorInput {
    pub(super) snapshot: AcceptedSnapshot,
    pub(super) scope: Option<Scope>,
    pub(super) query: PartsQuery,
    pub(super) selected: PartsSelection,
    pub(super) selected_context: Signal<Option<super::objects::ScopedTreeContext>>,
    pub(super) on_place_controller: EventHandler<String>,
    pub(super) on_place_component: EventHandler<super::part_placement::ComponentPlacementAction>,
    pub(super) controller_placement_enabled: bool,
    pub(super) placement_busy: bool,
    pub(super) placement_error: Option<String>,
    pub(super) layout_target: Signal<Option<String>>,
    pub(super) on_place_assembly: EventHandler<super::objects::MatrixPlacementSource>,
    pub(super) on_board_placed: EventHandler<()>,
    pub(super) on_open_module_placement: EventHandler<String>,
    pub(super) on_module_attached: EventHandler<super::parts::AttachedModuleNavigation>,
}

pub(super) struct CanvasInput {
    pub(super) controller_back: Option<EventHandler<()>>,
    pub(super) snapshot: AcceptedSnapshot,
    pub(super) scope: Option<Scope>,
    pub(super) query: PartsQuery,
    pub(super) selected: PartsSelection,
}

pub(super) fn objects(input: ObjectsInput) -> Element {
    rsx! {
        PartsLibraryPanel {
            snapshot: input.snapshot,
            scope: input.scope,
            scope_generation: input.scope_generation,
            workspace: input.workspace,
            query: input.query,
            selected: input.selected,
            on_select: input.on_select,
        }
    }
}

pub(super) fn toolbar() -> Element {
    rsx! {}
}

pub(super) fn canvas(mut input: PlaceholderInput) -> Element {
    rsx! {
        section { class: "m1-placeholder-workspace",
            h1 { "{input.name}" }
            p { "{input.message}" }
            button { onclick: move |_| input.workspace.set("Layout"), "Back to Layout" }
        }
    }
}

pub(super) fn preview(input: CanvasInput) -> Element {
    rsx! {
        if let Some(on_back) = input.controller_back {
            button { class: "m1-parts-controller-back", type: "button", onclick: move |_| on_back.call(()), "Back to PCB" }
        }
        PartsPreviewWorkspace {
            snapshot: input.snapshot,
            scope: input.scope,
            query: input.query,
            selected: input.selected,
        }
    }
}

pub(super) fn inspector(input: InspectorInput) -> Element {
    rsx! {
        PartsInspectorPanel {
            snapshot: input.snapshot,
            scope: input.scope,
            query: input.query,
            selected: input.selected,
            selected_context: input.selected_context,
            on_place_controller: input.on_place_controller,
            on_place_component: input.on_place_component,
            controller_placement_enabled: input.controller_placement_enabled,
            placement_busy: input.placement_busy,
            placement_error: input.placement_error,
            layout_target: input.layout_target,
            on_place_assembly: input.on_place_assembly,
            on_board_placed: input.on_board_placed,
            on_open_module_placement: input.on_open_module_placement,
            on_module_attached: input.on_module_attached,
        }
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod workspace_state_tests {
    use super::*;
    use boardstudio_application::SessionEpoch;
    use wasm_bindgen::JsCast;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[component]
    fn state_host() -> Element {
        let state = use_parts_workspace_state();
        let selected = state.selected();
        let generation = state.selection_generation();
        let activation = state.preview_activation;
        let scope = Scope {
            session_epoch: SessionEpoch(7),
            document_id: "parts-state".into(),
            board_id: "board".into(),
            instance_id: None,
        };
        let selected_label = selected()
            .as_ref()
            .map(|(_, id)| id.clone())
            .unwrap_or_else(|| "none".into());
        rsx! {
            button {
                id: "select-mounted-module",
                onclick: move |_| state.select_mounted_module(scope.clone(), "module-definition"),
                "Select module"
            }
            output { id: "selected-module", "{selected_label}" }
            output { id: "selection-generation", "{generation()}" }
            output { id: "preview-activation", "{activation()}" }
        }
    }

    #[wasm_bindgen_test]
    async fn mounted_module_selection_expires_prior_parts_work() {
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        let dom = VirtualDom::new(state_host);
        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        gloo_timers::future::TimeoutFuture::new(40).await;

        document
            .get_element_by_id("select-mounted-module")
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap()
            .click();
        gloo_timers::future::TimeoutFuture::new(40).await;

        assert_eq!(
            document
                .get_element_by_id("selected-module")
                .unwrap()
                .text_content()
                .as_deref(),
            Some("module:module-definition")
        );
        assert_eq!(
            document
                .get_element_by_id("selection-generation")
                .unwrap()
                .text_content()
                .as_deref(),
            Some("1")
        );
        assert_eq!(
            document
                .get_element_by_id("preview-activation")
                .unwrap()
                .text_content()
                .as_deref(),
            Some("1")
        );
        root.remove();
    }
}
