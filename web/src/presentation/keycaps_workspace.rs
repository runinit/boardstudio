//! Keycaps-owned workspace surface composition.
use super::Runtime;
use super::keycaps_fit::{FindingNavigationRequest, KeycapsFitInspector, KeycapsFitState};
use super::keycaps_scene::{KeycapsCanvas, KeycapsKeyList, KeycapsView};
use super::keycaps_settings::{
    KeycapsBoardSettingsEditor, KeycapsMatrixSettingsEditor, KeycapsSettingsActions,
    KeycapsSettingsEditor, SelectedKeySettings,
};
use super::objects;
use super::workspace_composition::{CanvasEventHandlers, SharedObjectsInput};
use super::{keycaps_finding_marker, keycaps_navigation};
use boardstudio_application::{Scope, SnapshotToken};
use boardstudio_core::model::Contour;
use dioxus::prelude::*;
use std::{collections::BTreeSet, rc::Rc};

pub(super) fn use_projection(
    runtime: Rc<Runtime>,
    accepted_token: Option<SnapshotToken>,
    current_scope: Option<Scope>,
    active_board_id: String,
) -> Option<(Rc<KeycapsView>, Rc<[Contour]>)> {
    use_memo(use_reactive(
        (&accepted_token, &current_scope, &active_board_id),
        {
            move |(token, scope, board_id)| {
                let model = runtime.model();
                let snapshot = model.accepted.as_ref()?;
                if token.as_ref() != Some(&snapshot.token) {
                    return None;
                }
                let view = super::keycaps_scene::project(snapshot, scope.as_ref()?, &board_id)?;
                Some((view, super::accepted_board_contours(snapshot, &board_id)))
            }
        },
    ))
    .read()
    .clone()
}

#[derive(Clone)]
pub(super) struct KeycapsWorkspaceState {
    focused_finding: Signal<Option<keycaps_finding_marker::FocusedFinding>>,
    pending_layout_fit: Signal<Option<keycaps_navigation::PendingLayoutFit>>,
    navigation_alive: Rc<std::cell::Cell<bool>>,
}

pub(super) fn use_keycaps_workspace_state(
    workspace: &'static str,
    scope: Option<Scope>,
    token: Option<SnapshotToken>,
    revision: Option<u64>,
    active_board_id: String,
) -> KeycapsWorkspaceState {
    let focused_finding = use_signal(|| None::<keycaps_finding_marker::FocusedFinding>);
    let pending_layout_fit = use_signal(|| None::<keycaps_navigation::PendingLayoutFit>);
    keycaps_finding_marker::use_retire_stale_finding(
        focused_finding,
        workspace,
        scope,
        token,
        revision,
        active_board_id,
    );
    KeycapsWorkspaceState {
        focused_finding,
        pending_layout_fit,
        navigation_alive: keycaps_navigation::use_navigation_lifetime(),
    }
}

impl KeycapsWorkspaceState {
    pub(super) fn focused_finding(&self) -> Signal<Option<keycaps_finding_marker::FocusedFinding>> {
        self.focused_finding
    }

    pub(super) fn navigation_alive(&self) -> Rc<std::cell::Cell<bool>> {
        self.navigation_alive.clone()
    }

    pub(super) fn set_focused_finding(
        &self,
        finding: Option<keycaps_finding_marker::FocusedFinding>,
    ) {
        let mut focused_finding = self.focused_finding;
        focused_finding.set(finding);
    }

    pub(super) fn focus_finding(
        &self,
        scope: Scope,
        token: SnapshotToken,
        revision: u64,
        finding_id: String,
    ) {
        let navigation_id =
            keycaps_finding_marker::next_navigation_id(self.focused_finding.peek().as_ref());
        self.set_focused_finding(Some(keycaps_finding_marker::FocusedFinding {
            scope,
            token,
            revision,
            finding_id,
            navigation_id,
        }));
    }

    pub(super) fn set_pending_layout_fit(
        &self,
        pending: Option<keycaps_navigation::PendingLayoutFit>,
    ) {
        let mut pending_layout_fit = self.pending_layout_fit;
        pending_layout_fit.set(pending);
    }

    pub(super) fn use_pending_layout_fit<T>(
        &self,
        observed_owner: T,
        current_owner: impl Fn() -> keycaps_navigation::LiveNavigationOwner + 'static,
        resolve_geometry: impl Fn(
            &keycaps_navigation::PendingLayoutFit,
        ) -> Option<keycaps_navigation::DestinationFitGeometry>
        + 'static,
        perform: impl Fn(keycaps_navigation::FitAction, keycaps_navigation::NavigationOwner) + 'static,
    ) where
        T: Clone + PartialEq + 'static,
    {
        keycaps_navigation::use_pending_layout_fit(
            self.pending_layout_fit,
            observed_owner,
            self.navigation_alive.clone(),
            current_owner,
            resolve_geometry,
            perform,
        );
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
        let scope = Scope {
            session_epoch: SessionEpoch(8),
            document_id: "keycaps-state".into(),
            board_id: "board".into(),
            instance_id: None,
        };
        let state = use_keycaps_workspace_state(
            "Layout",
            Some(scope.clone()),
            Some(SnapshotToken(3)),
            Some(2),
            "board".into(),
        );
        let finding = state.focused_finding();
        let navigation_id_label = finding()
            .map(|focused| focused.navigation_id.to_string())
            .unwrap_or_else(|| "none".into());
        rsx! {
            button {
                id: "focus-finding",
                onclick: move |_| state.focus_finding(scope.clone(), SnapshotToken(3), 2, "finding".into()),
                "Focus finding"
            }
            output { id: "focused-navigation-id", "{navigation_id_label}" }
        }
    }

    #[wasm_bindgen_test]
    async fn focusing_the_same_finding_advances_its_navigation_identity() {
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        let dom = VirtualDom::new(state_host);
        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        gloo_timers::future::TimeoutFuture::new(40).await;
        let button = document
            .get_element_by_id("focus-finding")
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap();
        button.click();
        gloo_timers::future::TimeoutFuture::new(40).await;
        assert_eq!(
            document
                .get_element_by_id("focused-navigation-id")
                .unwrap()
                .text_content()
                .as_deref(),
            Some("0")
        );
        button.click();
        gloo_timers::future::TimeoutFuture::new(40).await;
        assert_eq!(
            document
                .get_element_by_id("focused-navigation-id")
                .unwrap()
                .text_content()
                .as_deref(),
            Some("1")
        );
        root.remove();
    }
}

pub(super) struct CanvasInput {
    pub(super) view: Option<Rc<KeycapsView>>,
    pub(super) contours: Rc<[Contour]>,
    pub(super) view_box: String,
    pub(super) selected_ids: BTreeSet<String>,
    pub(super) handlers: CanvasEventHandlers,
    pub(super) on_select_key: EventHandler<String>,
}

pub(super) struct InspectorInput {
    pub(super) view: Option<Rc<KeycapsView>>,
    pub(super) document: Rc<boardstudio_core::model::ProjectDoc>,
    pub(super) selected_key_id: Option<String>,
    pub(super) on_select_key: EventHandler<String>,
    pub(super) settings_editor: Option<(SelectedKeySettings, KeycapsSettingsActions)>,
    pub(super) settings_actions: Option<KeycapsSettingsActions>,
    pub(super) fit_state: Option<KeycapsFitState>,
    pub(super) mechanical_layer_ids: Rc<[String]>,
    pub(super) fit_retry: EventHandler<()>,
    pub(super) fit_navigate: EventHandler<FindingNavigationRequest>,
    pub(super) on_export: EventHandler<()>,
}

pub(super) fn objects(input: SharedObjectsInput) -> Element {
    rsx! {
        objects::Objects {
            selected_context: input.selected_context,
            on_select: input.on_select,
            on_navigate: input.on_navigate,
            on_nudge: input.on_nudge,
            on_open_geometry_scripts: input.on_open_geometry_scripts,
            board_setup: Some(input.board_setup),
            matrix_setup: None,
            mirrored_pair: None,
            pair_created: None,
            on_place_component: None,
            layout_target: None,
            parts_query: None,
            on_browse_parts: None,
            placement_error: None,
            matrix_inspector: None,
        }
    }
}

pub(super) fn toolbar(input: super::shared_viewer::DesignViewToolbarProps) -> Element {
    rsx! {
        super::shared_viewer::DesignViewToolbar {
            label: input.label,
            detail: input.detail,
            assembly_3d: input.assembly_3d,
            footprints_visible: input.footprints_visible,
            on_view_mode: input.on_view_mode,
            on_toggle_footprints: input.on_toggle_footprints,
        }
    }
}

pub(super) fn canvas(input: CanvasInput) -> Element {
    let handlers = input.handlers;
    if let Some(view) = input.view {
        rsx! {
            section { class: "m1-keycaps-workspace", "aria-label": "Keycaps workspace",
                svg {
                    class: "m1-canvas m1-keycaps-canvas",
                    view_box: "{input.view_box}",
                    preserve_aspect_ratio: "xMidYMid meet",
                    tabindex: "0",
                    role: "group",
                    "aria-label": "Physical keycaps; select a key with click, Enter, or Space, hold Space and drag to pan, or use the mouse wheel to zoom",
                    onmounted: handlers.mount,
                    onpointerdown: handlers.start_pan,
                    onpointermove: handlers.move_pointer,
                    onpointerup: handlers.end_pointer,
                    onpointercancel: handlers.cancel_pointer,
                    onlostpointercapture: handlers.cancel_pointer,
                    onkeydown: handlers.keyboard,
                    onkeyup: handlers.key_up,
                    onwheel: handlers.wheel,
                    g { transform: "scale(1,-1)",
                        KeycapsCanvas {
                            view,
                            contours: input.contours,
                            selected_ids: input.selected_ids,
                            on_select_key: input.on_select_key,
                        }
                    }
                }
            }
        }
    } else {
        rsx! {
            p { class: "m1-keycaps-empty-note", role: "status", "Keycaps are unavailable for the current board." }
        }
    }
}

pub(super) fn inspector(input: InspectorInput) -> Element {
    if let Some(view) = input.view {
        let settings_editor = input.settings_editor;
        let settings_actions = input.settings_actions;
        let on_export = input.on_export;
        let selected_title = input
            .selected_key_id
            .as_deref()
            .and_then(|selected_id| view.keys.iter().find(|key| key.id.as_ref() == selected_id))
            .map_or_else(
                || "Select a key".to_owned(),
                |key| format!("{} · key", key.reference),
            );
        let assigned_count = view.assigned_count;
        let key_count = view.keys.len();
        rsx! {
            section { class: "m1-keycaps-inspector", "aria-label": "Keycaps inspector",
                header { class: "m1-keycaps-inspector-header",
                    h2 { "Keycaps" }
                    span { class: "m1-keycaps-inspector-assigned-count", "{assigned_count}/{key_count} assigned" }
                }
                if view.keys.is_empty() {
                    p { class: "m1-keycaps-empty-note", role: "status", "Add switches in Layout to create a keymap." }
                }
                if let Some(actions) = settings_actions.clone() {
                    KeycapsBoardSettingsEditor { settings: view.board_settings.clone(), actions: actions.clone() }
                    details { class: "m1-keycaps-disclosure", open: true,
                        summary { "Matrix profiles" }
                        section { class: "m1-keycaps-settings", "aria-label": "Matrix profiles",
                            for matrix in view.matrices.iter() {
                                KeycapsMatrixSettingsEditor { key: "{matrix.id}", matrix_id: matrix.id.to_string(), matrix_name: matrix.name.to_string(), settings: matrix.settings.clone(), actions: actions.clone() }
                            }
                            if view.matrices.is_empty() { p { class: "m1-keycaps-empty-note", "Standalone switches use their individual profile override." } }
                        }
                    }
                }
                details { class: "m1-keycaps-disclosure m1-keycaps-selected-key", open: true,
                    summary { "{selected_title}" }
                    div { class: "m1-keycaps-selected-key-body",
                        KeycapsKeyList {
                            view: view.clone(),
                            selected_key_id: input.selected_key_id.clone(),
                            on_select_key: input.on_select_key,
                        }
                        if let Some((selected, actions)) = settings_editor {
                            KeycapsSettingsEditor { selected, actions }
                        }
                    }
                }
                KeycapsFitInspector {
                    document: input.document,
                    state: input.fit_state,
                    mechanical_layer_ids: input.mechanical_layer_ids,
                    on_retry: input.fit_retry,
                    on_navigate: input.fit_navigate,
                }
                button { disabled: view.keys.is_empty(), onclick: move |_| on_export.call(()), "Export keycap STEP" }
            }
        }
    } else {
        rsx! {
            section { class: "m1-keycaps-selected-summary", "aria-label": "Selected keycap",
                h2 { "Keycaps unavailable" }
                p { "The current board has no Keycaps projection." }
            }
        }
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
#[path = "keycaps_workspace_tests.rs"]
mod tests;
