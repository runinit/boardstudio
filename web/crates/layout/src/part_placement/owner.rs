//! Project-scoped placement ownership: identity captures and admission checks that
//! keep a placement proposal bound to the snapshot, scope and generation that began it.
pub use super::plan::PendingPart;
use super::plan::{PlacementIntent, next_component_reference, placement_operation_for_intent};
use crate::runtime::Runtime;
use crate::setup_guide::{SetupGuidePreferences, SetupGuideStage};
use boardstudio_application::{
    AcceptedSnapshot, Durability, EditResolver, Lifecycle, Resolution, Scope, SnapshotToken,
};
use boardstudio_model::model::{
    EditOperation, MatrixAssembly, MatrixCell, Part, PartDefinition, PartKind, ProjectDoc, Vec2,
};
use boardstudio_web_ui_model::tree::{ScopedTreeContext, TreeContext};
use std::rc::Rc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlacementWorkflow {
    WiringController,
    PcbController,
    GeneralComponent,
}

impl PlacementWorkflow {
    pub(crate) fn is_controller(self) -> bool {
        self != Self::GeneralComponent
    }
}

#[derive(Clone)]
pub(crate) struct ControllerChooserOwner {
    pub(crate) accepted: AcceptedSnapshot,
    pub(crate) scope: Scope,
    pub(crate) generation: u64,
}

pub use boardstudio_web_ui_model::state::ComponentPlacementAction;

impl ControllerChooserOwner {
    pub(crate) fn is_current(&self, runtime: &Runtime, generation: u64, workspace: &str) -> bool {
        workspace == "Parts"
            && generation == self.generation
            && runtime.scope().as_ref() == Some(&self.scope)
            && accepted_snapshot_is_current(runtime, &self.accepted)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlacementOwner {
    pub token: SnapshotToken,
    pub session_epoch: boardstudio_application::SessionEpoch,
    pub project_id: String,
    pub revision: u64,
    pub scope: Scope,
    pub generation: u64,
    pub(crate) source_workspace: &'static str,
    pub board_id: String,
    pub part_id: String,
    pub definition_id: String,
    pub reference: String,
    pub layout_id: Option<String>,
    pub at: Vec2,
    pub(crate) workflow: PlacementWorkflow,
    pub(crate) intent: PlacementIntent,
}

pub(crate) struct PlacementCapture {
    pub(crate) scope: Scope,
    pub(crate) generation: u64,
    pub(crate) part_id: String,
    pub(crate) definition_id: String,
    pub(crate) kind: PartKind,
    pub(crate) workflow: PlacementWorkflow,
    pub(crate) intent: PlacementIntent,
    pub(crate) source_workspace: &'static str,
    pub(crate) layout_id: Option<String>,
    pub(crate) at: Vec2,
}

impl PlacementOwner {
    pub(crate) fn record_committed_position(&mut self, at: Vec2) {
        self.at = at;
    }

    pub(crate) fn capture(snapshot: &AcceptedSnapshot, request: PlacementCapture) -> Option<Self> {
        let PlacementCapture {
            scope,
            generation,
            part_id,
            definition_id,
            kind,
            workflow,
            intent,
            source_workspace,
            layout_id,
            at,
        } = request;
        if snapshot.document.id != scope.document_id
            || snapshot.session_epoch != scope.session_epoch
            || !snapshot
                .document
                .boards
                .iter()
                .any(|board| board.id == scope.board_id)
        {
            return None;
        }
        let reference = next_component_reference(&snapshot.document, &kind);
        let board_id = scope.board_id.clone();
        Some(Self {
            token: snapshot.token,
            session_epoch: snapshot.session_epoch,
            project_id: snapshot.document.id.clone(),
            revision: snapshot.document.revision,
            scope,
            generation,
            source_workspace,
            board_id,
            part_id,
            definition_id,
            reference,
            layout_id,
            at,
            workflow,
            intent,
        })
    }

    pub fn is_current(
        &self,
        snapshot: &AcceptedSnapshot,
        scope: Option<&Scope>,
        generation: u64,
        guide_open: bool,
        guide_project_id: Option<&str>,
        guide_stage_is_wiring: bool,
    ) -> bool {
        scope == Some(&self.scope)
            && generation == self.generation
            && snapshot.token == self.token
            && snapshot.session_epoch == self.session_epoch
            && snapshot.document.id == self.project_id
            && snapshot.document.revision == self.revision
            && self.board_id == self.scope.board_id
            && (self.workflow != PlacementWorkflow::WiringController
                || (guide_open
                    && guide_project_id == Some(self.project_id.as_str())
                    && guide_stage_is_wiring))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ActivePartPlacement {
    pub owner: PlacementOwner,
    pub pending: PendingPart,
    pub snap_document: Rc<ProjectDoc>,
}

/// An in-flight apply-to-key edit. One-shot: the canvas stays busy while it is held.
#[derive(Clone)]
pub(crate) struct KeyEditOwner {
    pub(crate) scope: Scope,
    pub(crate) generation: u64,
    pub(crate) source_workspace: &'static str,
    pub(crate) selection: ScopedTreeContext,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PlacementPendingKey {
    KeyComponent,
    Placement,
}

pub(crate) fn placement_resolver(
    board_id: String,
    definition: PartDefinition,
    part: Part,
    layout_id: Option<String>,
    intent: PlacementIntent,
) -> EditResolver {
    EditResolver::new(
        "layout-component-placement",
        move |accepted: &AcceptedSnapshot| match placement_operation_for_intent(
            &accepted.document,
            &board_id,
            &definition,
            &part,
            layout_id.as_deref(),
            intent,
        ) {
            Ok(operation) => Resolution::submit(vec![part.id.clone()], operation),
            Err(message) => Resolution::Retire(message),
        },
    )
}

/// Resolve applying a loaded component definition to the selected key.
pub(crate) fn key_component_resolver(
    scope: Scope,
    selection: ScopedTreeContext,
    definition: PartDefinition,
) -> EditResolver {
    EditResolver::new(
        "layout-key-component",
        move |accepted: &AcceptedSnapshot| {
            let operation = match selected_key_component_operation(
                &accepted.document,
                &scope,
                &selection,
                &definition,
            ) {
                Ok(operation) => operation,
                Err(message) => return Resolution::Retire(message),
            };
            let EditOperation::SetMatrix {
                matrix,
                definitions,
            } = &operation
            else {
                return Resolution::Retire("The selected key can no longer be edited.".into());
            };
            let mut target_ids = vec![matrix.id.clone()];
            if definitions.is_some() {
                target_ids.push(definition.id.clone());
            }
            Resolution::submit(target_ids, operation)
        },
    )
}

pub(crate) struct PlacementAdmission {
    scope: Option<Scope>,
    generation: u64,
    workspace: &'static str,
    expected_workspace: &'static str,
    preferences: Option<SetupGuidePreferences>,
}

impl PlacementAdmission {
    pub(crate) fn capture(
        runtime: &Runtime,
        generation: u64,
        workspace: &'static str,
        expected_workspace: &'static str,
        preferences: Option<SetupGuidePreferences>,
    ) -> Self {
        Self {
            scope: runtime.scope(),
            generation,
            workspace,
            expected_workspace,
            preferences,
        }
    }

    pub(crate) fn admits(
        &self,
        owner: &PlacementOwner,
        runtime: &Runtime,
        model: &boardstudio_application::ReadModel,
    ) -> bool {
        let Some(snapshot) = model.accepted.as_ref() else {
            return false;
        };
        owner.is_current(
            snapshot,
            self.scope.as_ref(),
            self.generation,
            self.preferences
                .as_ref()
                .is_some_and(|preferences| preferences.open),
            self.preferences
                .as_ref()
                .map(|preferences| preferences.project_id.as_str()),
            self.preferences
                .as_ref()
                .is_some_and(|preferences| preferences.current_stage == SetupGuideStage::Wiring),
        ) && model.active_board_id == owner.board_id
            && runtime.scope().as_ref() == Some(&owner.scope)
            && self.workspace == self.expected_workspace
    }
}

#[derive(Clone)]
pub(crate) struct ComponentActionOwner {
    pub(crate) accepted: AcceptedSnapshot,
    pub(crate) scope: Scope,
    pub(crate) generation: u64,
    pub(crate) workspace: &'static str,
    pub(crate) selected_context: Option<ScopedTreeContext>,
}

impl ComponentActionOwner {
    pub(crate) fn capture(
        runtime: &Runtime,
        generation: u64,
        workspace: &'static str,
        selected_context: Option<ScopedTreeContext>,
    ) -> Option<Self> {
        let model = runtime.model();
        let accepted = model.accepted?;
        let scope = runtime.scope()?;
        if !accepted_snapshot_is_current(runtime, &accepted)
            || accepted.document.id != scope.document_id
            || accepted.session_epoch != scope.session_epoch
        {
            return None;
        }
        Some(Self {
            accepted,
            scope,
            generation,
            workspace,
            selected_context,
        })
    }

    pub(crate) fn is_current(
        &self,
        runtime: &Runtime,
        generation: u64,
        workspace: &'static str,
        selected_context: Option<ScopedTreeContext>,
        expected_workspace: &'static str,
    ) -> bool {
        self.workspace == expected_workspace
            && workspace == self.workspace
            && self.generation == generation
            && self.selected_context == selected_context
            && runtime.scope().as_ref() == Some(&self.scope)
            && accepted_snapshot_is_current(runtime, &self.accepted)
    }
}

pub(crate) fn accepted_snapshot_is_current(runtime: &Runtime, accepted: &AcceptedSnapshot) -> bool {
    let model = runtime.model();
    model.lifecycle == Lifecycle::Ready
        && model.durability
            == (Durability::Saved {
                revision: accepted.document.revision,
            })
        && model.accepted.as_ref().is_some_and(|current| {
            current.token == accepted.token
                && current.session_epoch == accepted.session_epoch
                && current.document.id == accepted.document.id
                && current.document.revision == accepted.document.revision
        })
}

pub fn selected_key_component_operation(
    accepted: &ProjectDoc,
    scope: &Scope,
    selection: &ScopedTreeContext,
    definition: &PartDefinition,
) -> Result<EditOperation, String> {
    if selection.scope != *scope {
        return Err("The selected key belongs to a different board scope.".into());
    }
    if scope.document_id != accepted.id
        || !accepted
            .boards
            .iter()
            .any(|board| board.id == scope.board_id)
    {
        return Err("The selected board is no longer available.".into());
    }
    let TreeContext::Key {
        matrix_id,
        row,
        column,
    } = &selection.context
    else {
        return Err("Select a matrix key before applying this component.".into());
    };
    let mut matrix = accepted
        .matrices
        .iter()
        .find(|matrix| {
            matrix.id == *matrix_id
                && crate::objects::matrix_visible_on_board(accepted, &scope.board_id, matrix_id)
        })
        .filter(|matrix| {
            matrix
                .board_id
                .as_deref()
                .is_none_or(|board_id| board_id == scope.board_id)
                && *row < matrix.rows
                && *column < matrix.columns
        })
        .cloned()
        .ok_or_else(|| "The selected matrix key is no longer available.".to_string())?;
    let current = matrix
        .cells
        .iter()
        .find(|cell| cell.row == *row && cell.column == *column)
        .cloned();
    let can_drive_matrix = matches!(definition.kind, PartKind::Switch)
        || crate::parts::matrix_input_available(definition);
    let mut cell = current.clone().unwrap_or(MatrixCell {
        shift: None,
        bend: None,
        deleted: false,
        row: *row,
        column: *column,
        enabled: true,
        definition_id: None,
        variant: None,
        offset: None,
        rotation: None,
        assemblies: Vec::new(),
        assemblies_local: None,
    });
    cell.enabled = true;
    if can_drive_matrix {
        cell.definition_id = Some(definition.id.clone());
    } else {
        let safe_definition_id = definition
            .id
            .chars()
            .map(|character| {
                if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                    character
                } else {
                    '-'
                }
            })
            .collect::<String>();
        let mut ordinal = cell.assemblies.len() + 1;
        let id = loop {
            let candidate = format!("library-{safe_definition_id}-{ordinal}");
            if !cell
                .assemblies
                .iter()
                .any(|assembly| assembly.id == candidate)
            {
                break candidate;
            }
            ordinal += 1;
        };
        cell.assemblies.push(MatrixAssembly {
            id,
            definition_id: definition.id.clone(),
            offset: Vec2::default(),
            rotation: None,
            side: None,
        });
    }
    let assemblies_changed = current.as_ref().is_none_or(|previous| {
        previous.definition_id != cell.definition_id
            || previous.variant != cell.variant
            || previous.assemblies != cell.assemblies
    });
    let is_mirror_target = accepted.layouts.iter().any(|layout| {
        layout.matrix_ids.iter().any(|id| id == matrix_id) && layout.mirror_link.is_some()
    });
    if assemblies_changed && is_mirror_target {
        cell.assemblies_local = Some(true);
    }
    matrix
        .cells
        .retain(|existing| existing.row != *row || existing.column != *column);
    matrix.cells.push(cell);
    let definitions = match accepted
        .definitions
        .iter()
        .find(|item| item.id == definition.id)
    {
        Some(existing) if existing != definition => {
            return Err("The selected component definition changed before it was applied.".into());
        }
        Some(_) => None,
        None => Some(vec![definition.clone()]),
    };
    Ok(EditOperation::SetMatrix {
        matrix,
        definitions,
    })
}
