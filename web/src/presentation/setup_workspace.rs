//! Setup guide and instance-selection state shared by Editor's workspace views.
use super::{PanelSettings, Runtime, WorkspaceState, setup_guide};
use boardstudio_web_ui_model::Workspace;
use dioxus::prelude::*;
use std::{cell::RefCell, collections::BTreeSet, rc::Rc};

#[derive(Clone, Copy)]
pub(super) struct SetupWorkspaceState {
    guide_preferences: Signal<Option<setup_guide::SetupGuidePreferences>>,
}

pub(super) fn use_setup_workspace_state(
    runtime: Rc<Runtime>,
    mut created_req: Signal<Option<setup_guide::SetupGuideRequest>>,
    objects_open: Signal<bool>,
    inspect_open: Signal<bool>,
    objects_settings: Signal<PanelSettings>,
    inspector_settings: Signal<PanelSettings>,
) -> SetupWorkspaceState {
    let mut guide_prefs = use_signal(|| None::<setup_guide::SetupGuidePreferences>);
    let created = created_req();
    let project_id = runtime
        .model()
        .accepted
        .as_ref()
        .map(|s| s.document.id.clone());
    let mut workspace = use_context::<WorkspaceState>().0;
    let consumed = use_hook(|| Rc::new(RefCell::new(BTreeSet::<String>::new())));
    use_effect(use_reactive!(|project_id, created| {
        let Some(pid) = project_id.as_ref() else {
            return;
        };
        if let Some(req) = created.as_ref().filter(|r| r.project_id == *pid) {
            if consumed.borrow_mut().insert(req.request_id.clone()) {
                let stage = if req.start_at_project {
                    setup_guide::SetupGuideStage::Project
                } else {
                    guide_prefs()
                        .filter(|p| p.project_id == *pid)
                        .map(|p| p.current_stage)
                        .unwrap_or_else(|| setup_guide::read_preferences(pid).current_stage)
                };
                guide_prefs.set(Some(setup_guide::SetupGuidePreferences {
                    project_id: pid.clone(),
                    open: true,
                    current_stage: stage,
                }));
                if req.start_at_project {
                    workspace.set(Workspace::Layout);
                }
                setup_guide::reveal_panels(
                    crate::setup_guide_state::GuideReveal::Guide,
                    objects_open,
                    inspect_open,
                    objects_settings,
                    inspector_settings,
                );
                created_req.set(None);
            }
        } else if guide_prefs().as_ref().is_none_or(|p| p.project_id != *pid) {
            let prefs = setup_guide::read_preferences(pid);
            if prefs.open {
                setup_guide::reveal_panels(
                    crate::setup_guide_state::GuideReveal::Guide,
                    objects_open,
                    inspect_open,
                    objects_settings,
                    inspector_settings,
                );
            }
            guide_prefs.set(Some(prefs));
        }
    }));
    let to_persist = guide_prefs();
    use_effect(use_reactive!(
        |to_persist| if let Some(p) = to_persist.as_ref() {
            setup_guide::write_preferences(p);
        }
    ));
    SetupWorkspaceState {
        guide_preferences: guide_prefs,
    }
}

impl SetupWorkspaceState {
    pub(super) fn guide_preferences(self) -> Signal<Option<setup_guide::SetupGuidePreferences>> {
        self.guide_preferences
    }
    pub(super) fn set_guide_preferences(&self, p: Option<setup_guide::SetupGuidePreferences>) {
        let mut s = self.guide_preferences;
        s.set(p);
    }
    pub(super) fn update_guide_stage(
        &self,
        project_id: &str,
        stage: setup_guide::SetupGuideStage,
    ) -> bool {
        let Some(mut p) = self
            .guide_preferences
            .read()
            .clone()
            .filter(|p| p.project_id == project_id)
        else {
            return false;
        };
        p.current_stage = stage;
        p.open = true;
        self.set_guide_preferences(Some(p));
        true
    }
    pub(super) fn dismiss_guide(&self, project_id: &str) {
        let Some(mut p) = self
            .guide_preferences
            .peek()
            .as_ref()
            .filter(|p| p.project_id == project_id)
            .cloned()
        else {
            return;
        };
        p.open = false;
        self.set_guide_preferences(Some(p));
    }
}
