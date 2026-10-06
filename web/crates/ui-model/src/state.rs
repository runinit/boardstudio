//! Workspace, view and layout state that the page shell provides as Dioxus context and
//! feature crates read.
use crate::case_generation_lifecycle::AutomaticCaseGeneration;
use crate::instance_selection;
use boardstudio_application::{Scope, SnapshotToken};
use boardstudio_core::model::{Position, Vec2};
use boardstudio_web_runtime::operation_outcomes::OutcomeSlot;
use dioxus::prelude::*;
use std::collections::BTreeSet;

#[derive(Clone)]
pub struct Drag {
    pub pointer: i64,
    pub scope: Scope,
    pub generation: u64,
    pub gesture_generation: Option<u64>,
    pub origin: Vec2,
    pub client_x: f64,
    pub client_y: f64,
    pub positions: Vec<Position>,
    pub active: bool,
    pub pan: bool,
    pub camera: Vec2,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayoutOwnerIdentity {
    pub scope: Option<Scope>,
    pub token: Option<SnapshotToken>,
    pub revision: Option<u64>,
    pub generation: u64,
    pub workspace: &'static str,
}

#[derive(Clone, Copy)]
pub struct WorkspaceState(pub Signal<&'static str>);
#[derive(Clone, Copy)]
pub struct CompactPanelState {
    pub objects_open: Signal<bool>,
    pub inspector_open: Signal<bool>,
}
/// The explicit UI preference is separate from Session's effective instance.
#[derive(Clone, Copy)]
pub struct InstanceSelection(pub Signal<Option<instance_selection::Preference>>);

#[derive(Clone, Copy)]
pub struct CaseGenerationState {
    pub live_preview: Signal<bool>,
    pub automatic: Signal<AutomaticCaseGeneration>,
}

impl InstanceSelection {
    pub fn is_current(self, model: &boardstudio_application::ReadModel) -> bool {
        instance_selection::is_current(model, self.0.read().as_ref())
    }

    pub fn reconcile(
        mut self,
        session_epoch: boardstudio_application::SessionEpoch,
        document_id: String,
        explicit_id: String,
    ) {
        self.0.set(Some(instance_selection::Preference {
            session_epoch,
            document_id,
            explicit_id,
        }));
    }
}

#[cfg(all(any(test, feature = "test-support"), target_arch = "wasm32"))]
pub fn use_empty_test_instance_selection() {
    let preference = use_signal(|| None);
    use_context_provider(|| InstanceSelection(preference));
}

#[cfg(all(any(test, feature = "test-support"), target_arch = "wasm32"))]
pub fn use_test_case_generation_state() {
    let live_preview = use_signal(|| true);
    let automatic = use_signal(AutomaticCaseGeneration::new);
    use_context_provider(|| CaseGenerationState {
        live_preview,
        automatic,
    });
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ProjectMenuPage {
    Project,
    Settings,
}

#[derive(Clone, Copy)]
pub struct ResolvedTheme(pub Memo<&'static str>);

#[derive(Clone, Copy)]
pub struct LayerVisibility {
    pub hidden: Signal<BTreeSet<String>>,
    pub modules_hidden: Signal<BTreeSet<String>>,
    pub footprints: Signal<bool>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LayoutSnapSettings {
    /// Zero disables the grid, positive values are fractions of pitch, negative values are mm.
    pub snap_fraction: f64,
    pub geometry_snap: bool,
    pub gap_snap: bool,
    /// Kept as an editable draft; blank or invalid input uses the current context's fallback.
    pub gap_override: String,
}

impl Default for LayoutSnapSettings {
    fn default() -> Self {
        Self {
            snap_fraction: 0.25,
            geometry_snap: true,
            gap_snap: true,
            gap_override: String::new(),
        }
    }
}

/// Retained cell coordinates let mode changes preserve the full matrix scope after Row/Column
/// contexts intentionally omit one coordinate. The root owns and clears this with its live owner.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreeCellAnchor {
    pub matrix_id: String,
    pub row: u32,
    pub column: u32,
}

#[derive(Clone, Copy)]
pub struct ThemeState(pub Signal<&'static str>);
#[derive(Clone, Copy)]
pub struct PreferenceStorageWarning(pub Signal<bool>);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SetupGuideRequest {
    pub project_id: String,
    pub request_id: String,
    pub start_at_project: bool,
}

#[derive(Clone)]
pub struct PendingNewKeyboard {
    pub project_id: String,
    pub outcome: OutcomeSlot,
}
