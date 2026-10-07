//! Setup guide and instance-selection state shared by Editor's workspace views.
use super::{
    InstanceSelection, PanelSettings, Runtime, WorkspaceState, instance_selection, setup_guide,
};
use dioxus::prelude::*;
use std::{cell::RefCell, collections::BTreeSet, rc::Rc};

#[derive(Clone, Copy)]
pub(super) struct SetupWorkspaceState {
    guide_preferences: Signal<Option<setup_guide::SetupGuidePreferences>>,
    instance_preference: Signal<Option<instance_selection::Preference>>,
    geometry_scripts_open: Signal<bool>,
}

pub(super) fn use_setup_workspace_state(
    runtime: Rc<Runtime>,
    created_request_signal: Signal<Option<setup_guide::SetupGuideRequest>>,
    objects_open: Signal<bool>,
    inspect_open: Signal<bool>,
    objects_panel_settings: Signal<PanelSettings>,
    inspector_panel_settings: Signal<PanelSettings>,
) -> SetupWorkspaceState {
    let mut guide_preferences = use_signal(|| None::<setup_guide::SetupGuidePreferences>);
    let instance_preference = use_signal(|| {
        runtime.scope().and_then(|scope| {
            scope
                .instance_id
                .map(|explicit_id| instance_selection::Preference {
                    session_epoch: scope.session_epoch,
                    document_id: scope.document_id,
                    explicit_id,
                })
        })
    });
    use_context_provider(|| InstanceSelection(instance_preference));
    let geometry_scripts_open = use_signal(|| false);
    let created_request = created_request_signal();
    let accepted_project_id = runtime
        .model()
        .accepted
        .as_ref()
        .map(|snapshot| snapshot.document.id.clone());
    let mut guide_workspace = use_context::<WorkspaceState>().0;
    let consumed_guide_requests = use_hook(|| Rc::new(RefCell::new(BTreeSet::<String>::new())));
    use_effect(use_reactive!(|accepted_project_id, created_request| {
        let mut created_request_signal = created_request_signal;
        let Some(project_id) = accepted_project_id.as_ref() else {
            return;
        };
        if let Some(request) = created_request
            .as_ref()
            .filter(|request| request.project_id == *project_id)
        {
            let already_consumed = consumed_guide_requests
                .borrow()
                .contains(&request.request_id);
            if !already_consumed {
                consumed_guide_requests
                    .borrow_mut()
                    .insert(request.request_id.clone());
                guide_preferences.set(Some(setup_guide::SetupGuidePreferences {
                    project_id: project_id.clone(),
                    open: true,
                    current_stage: if request.start_at_project {
                        setup_guide::SetupGuideStage::Project
                    } else {
                        guide_preferences()
                            .filter(|preferences| preferences.project_id == *project_id)
                            .map(|preferences| preferences.current_stage)
                            .unwrap_or_else(|| {
                                setup_guide::read_preferences(project_id).current_stage
                            })
                    },
                }));
                if request.start_at_project {
                    guide_workspace.set("Layout");
                }
                setup_guide::reveal_panels(
                    crate::setup_guide_state::GuideReveal::Guide,
                    objects_open,
                    inspect_open,
                    objects_panel_settings,
                    inspector_panel_settings,
                );
                created_request_signal.set(None);
            }
        } else if guide_preferences()
            .as_ref()
            .is_none_or(|preferences| preferences.project_id != *project_id)
        {
            let preferences = setup_guide::read_preferences(project_id);
            if preferences.open {
                setup_guide::reveal_panels(
                    crate::setup_guide_state::GuideReveal::Guide,
                    objects_open,
                    inspect_open,
                    objects_panel_settings,
                    inspector_panel_settings,
                );
            }
            guide_preferences.set(Some(preferences));
        }
    }));
    let preferences_to_persist = guide_preferences();
    use_effect(use_reactive!(|preferences_to_persist| {
        if let Some(preferences) = preferences_to_persist.as_ref() {
            setup_guide::write_preferences(preferences);
        }
    }));
    SetupWorkspaceState {
        guide_preferences,
        instance_preference,
        geometry_scripts_open,
    }
}

impl SetupWorkspaceState {
    pub(super) fn guide_preferences(self) -> Signal<Option<setup_guide::SetupGuidePreferences>> {
        self.guide_preferences
    }

    pub(super) fn instance_preference(self) -> Signal<Option<instance_selection::Preference>> {
        self.instance_preference
    }

    pub(super) fn geometry_scripts_open(self) -> Signal<bool> {
        self.geometry_scripts_open
    }

    pub(super) fn set_geometry_scripts_open(&self, open: bool) {
        let mut geometry_scripts_open = self.geometry_scripts_open;
        geometry_scripts_open.set(open);
    }

    pub(super) fn set_guide_preferences(
        &self,
        preferences: Option<setup_guide::SetupGuidePreferences>,
    ) {
        let mut guide_preferences = self.guide_preferences;
        guide_preferences.set(preferences);
    }

    pub(super) fn update_guide_stage(
        &self,
        project_id: &str,
        stage: setup_guide::SetupGuideStage,
    ) -> bool {
        let mut preferences = self.guide_preferences().read().clone();
        let Some(preferences) = preferences.as_mut().filter(|p| p.project_id == project_id) else {
            return false;
        };
        preferences.current_stage = stage;
        preferences.open = true;
        self.set_guide_preferences(Some(preferences.clone()));
        true
    }

    pub(super) fn dismiss_guide(&self, project_id: &str) {
        let mut preferences = self.guide_preferences().read().clone();
        let Some(preferences) = preferences.as_mut().filter(|p| p.project_id == project_id) else {
            return;
        };
        preferences.open = false;
        self.set_guide_preferences(Some(preferences.clone()));
    }

    pub(super) fn set_instance_preference(
        &self,
        preference: Option<instance_selection::Preference>,
    ) {
        let mut instance_preference = self.instance_preference;
        instance_preference.set(preference);
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod workspace_state_tests {
    use super::*;
    use wasm_bindgen::JsCast;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[component]
    fn state_host() -> Element {
        let guide_preferences = use_signal(|| {
            Some(setup_guide::SetupGuidePreferences {
                project_id: "setup-state".into(),
                open: true,
                current_stage: setup_guide::SetupGuideStage::Project,
            })
        });
        let geometry_scripts_open = use_signal(|| false);
        let instance_preference = use_signal(|| None::<instance_selection::Preference>);
        let state = SetupWorkspaceState {
            guide_preferences,
            geometry_scripts_open,
            instance_preference,
        };
        let status = guide_preferences()
            .map(|preferences| format!("{:?}:{}", preferences.current_stage, preferences.open))
            .unwrap_or_else(|| "none".into());
        rsx! {
            button {
                id: "setup-state-stage",
                onclick: move |_| { state.update_guide_stage("setup-state", setup_guide::SetupGuideStage::Layout); },
                "Change stage"
            }
            button { id: "setup-state-dismiss", onclick: move |_| state.dismiss_guide("setup-state"), "Dismiss" }
            output { id: "setup-state-status", "{status}" }
        }
    }

    #[wasm_bindgen_test]
    async fn setup_handle_updates_and_dismisses_only_its_project_guide() {
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        let dom = VirtualDom::new(state_host);
        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        gloo_timers::future::TimeoutFuture::new(40).await;
        let stage_button = document
            .get_element_by_id("setup-state-stage")
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap();
        stage_button.click();
        gloo_timers::future::TimeoutFuture::new(40).await;
        assert_eq!(
            document
                .get_element_by_id("setup-state-status")
                .unwrap()
                .text_content()
                .as_deref(),
            Some("Layout:true")
        );
        document
            .get_element_by_id("setup-state-dismiss")
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap()
            .click();
        gloo_timers::future::TimeoutFuture::new(40).await;
        assert_eq!(
            document
                .get_element_by_id("setup-state-status")
                .unwrap()
                .text_content()
                .as_deref(),
            Some("Layout:false")
        );
        root.remove();
    }
}
