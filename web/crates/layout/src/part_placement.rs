//! Private project-scoped placement planning for catalogue components.
//!
//! The browser controller owns the interaction lifetime; this module keeps the
//! identity checks and atomic document proposal independent of the canvas DOM.
use super::{
    canvas_interaction::{CanvasInteractionArbiter, CanvasInteractionOwner},
    objects::{self, LayoutSnapSettings},
    objects::{ScopedTreeContext, TreeContext},
    parts::{PartsQuery, PartsSelection},
    selection::SelectionAdapter,
    setup_guide::{SetupGuidePreferences, SetupGuideStage},
};
use crate::runtime::Runtime;
use boardstudio_application::{
    AcceptedSnapshot, Durability, EditResolver, Event, Lifecycle, Resolution, Scope, SelectionMode,
    SnapshotToken,
};
use boardstudio_core::model::{
    EditOperation, EditPhase, MatrixAssembly, MatrixCell, OutlineFeature, Part,
    PartDefinition, PartKind, Pose2, ProjectDoc, Side, Vec2,
};
use boardstudio_web_runtime::edit_ticket::{EditTicket, Settlement};
use dioxus::prelude::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::spawn_local;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlacementWorkflow {
    WiringController,
    PcbController,
    GeneralComponent,
}

impl PlacementWorkflow {
    fn is_controller(self) -> bool {
        self != Self::GeneralComponent
    }
}

#[derive(Clone)]
struct ControllerChooserOwner {
    accepted: AcceptedSnapshot,
    scope: Scope,
    generation: u64,
}

pub use boardstudio_web_ui_model::state::ComponentPlacementAction;

impl ControllerChooserOwner {
    fn is_current(&self, runtime: &dyn PlacementRuntime, generation: u64, workspace: &str) -> bool {
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
    source_workspace: &'static str,
    pub board_id: String,
    pub part_id: String,
    pub definition_id: String,
    pub reference: String,
    pub layout_id: Option<String>,
    pub at: Vec2,
    workflow: PlacementWorkflow,
}

struct PlacementCapture {
    scope: Scope,
    generation: u64,
    part_id: String,
    definition_id: String,
    kind: PartKind,
    workflow: PlacementWorkflow,
    source_workspace: &'static str,
    layout_id: Option<String>,
    at: Vec2,
}

impl PlacementOwner {
    fn record_committed_position(&mut self, at: Vec2) {
        self.at = at;
    }

    fn capture(snapshot: &AcceptedSnapshot, request: PlacementCapture) -> Option<Self> {
        let PlacementCapture {
            scope,
            generation,
            part_id,
            definition_id,
            kind,
            workflow,
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
pub struct PendingPart {
    pub definition: PartDefinition,
    pub part: Part,
    pub at: Vec2,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ActivePartPlacement {
    pub owner: PlacementOwner,
    pub pending: PendingPart,
    pub snap_document: Rc<ProjectDoc>,
}

/// An in-flight apply-to-key edit. One-shot: the canvas stays busy while it is held.
#[derive(Clone)]
struct PendingKeyEdit {
    scope: Scope,
    generation: u64,
    source_workspace: &'static str,
    selection: ScopedTreeContext,
}

/// Resolve a placement commit against the accepted document at execution time: the
/// operation is built from the document the resolver is handed, so a placement queued
/// behind other edits lands on top of them.
fn placement_resolver(
    board_id: String,
    definition: PartDefinition,
    part: Part,
    layout_id: Option<String>,
) -> EditResolver {
    EditResolver::new(
        "layout-component-placement",
        move |accepted: &AcceptedSnapshot| {
            match placement_operation(
                &accepted.document,
                &board_id,
                &definition,
                &part,
                layout_id.as_deref(),
            ) {
                Ok(operation) => Resolution::Submit(boardstudio_core::model::EditCommand {
                    base_revision: 0,
                    transaction_id: String::new(),
                    phase: EditPhase::Commit,
                    target_ids: vec![part.id.clone()],
                    operation,
                }),
                Err(message) => Resolution::Retire(message),
            }
        },
    )
}

/// Resolve applying a loaded component definition to the selected key.
fn key_component_resolver(
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
            Resolution::Submit(boardstudio_core::model::EditCommand {
                base_revision: 0,
                transaction_id: String::new(),
                phase: EditPhase::Commit,
                target_ids,
                operation,
            })
        },
    )
}

#[derive(Clone)]
pub struct PartPlacementMount {
    pub projection: Option<ActivePartPlacement>,
    pub busy: bool,
    pub error: Option<String>,
    pub on_choose_controller: EventHandler<()>,
    pub on_place_controller: EventHandler<String>,
    pub controller_placement_enabled: bool,
    pub controller_back: Option<EventHandler<()>>,
    pub on_place_component: EventHandler<ComponentPlacementAction>,
    pub on_move: EventHandler<Vec2>,
    pub on_commit: EventHandler<Vec2>,
    pub on_cancel: EventHandler<()>,
}

impl PartPlacementMount {
    #[cfg(test)]
    pub fn owns_canvas(&self) -> bool {
        self.busy || self.projection.is_some()
    }
}

pub struct PartPlacementHost {
    pub runtime: Rc<dyn PlacementRuntime>,
    pub load_definition: DefinitionLoader,
    pub workspace: Signal<&'static str>,
    pub generation: Signal<u64>,
    pub version: Signal<u64>,
    pub adapter: SelectionAdapter,
    pub layout_selection_kind: Signal<objects::LayoutSelectionKind>,
    pub guide_preferences: Signal<Option<SetupGuidePreferences>>,
    pub parts_query: PartsQuery,
    pub parts_selection: PartsSelection,
    pub snap_settings: Signal<LayoutSnapSettings>,
    pub layout_target: Signal<Option<String>>,
    pub canvas_center: Vec2,
    pub objects_open: Signal<bool>,
    pub inspect_open: Signal<bool>,
    pub canvas_interaction: CanvasInteractionArbiter,
}

pub type DefinitionLoader = Rc<
    dyn Fn(
        ProjectDoc,
        String,
    )
        -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<PartDefinition, String>>>>,
>;

pub trait PlacementRuntime {
    fn model(&self) -> boardstudio_application::ReadModel;
    fn scope(&self) -> Option<Scope>;
    fn operation(&self) -> boardstudio_application::OperationId;
    fn observe_operation(
        &self,
        operation: boardstudio_application::OperationId,
    ) -> crate::operation_outcomes::OutcomeSlot;
    fn submit(&self, event: Event);
    /// Begin a pending edit through the shared edit ticket.
    fn begin_edit(&self, label: &str, feature: Option<String>, resolver: EditResolver)
    -> EditTicket;
}

impl<T: PlacementRuntime + ?Sized> PlacementRuntime for Rc<T> {
    fn model(&self) -> boardstudio_application::ReadModel {
        self.as_ref().model()
    }

    fn scope(&self) -> Option<Scope> {
        self.as_ref().scope()
    }

    fn operation(&self) -> boardstudio_application::OperationId {
        self.as_ref().operation()
    }

    fn observe_operation(
        &self,
        operation: boardstudio_application::OperationId,
    ) -> crate::operation_outcomes::OutcomeSlot {
        self.as_ref().observe_operation(operation)
    }

    fn submit(&self, event: Event) {
        self.as_ref().submit(event);
    }

    fn begin_edit(
        &self,
        label: &str,
        feature: Option<String>,
        resolver: EditResolver,
    ) -> EditTicket {
        self.as_ref().begin_edit(label, feature, resolver)
    }
}

struct RuntimePlacementAdapter(Rc<Runtime>);

impl PlacementRuntime for RuntimePlacementAdapter {
    fn model(&self) -> boardstudio_application::ReadModel {
        self.0.model()
    }

    fn scope(&self) -> Option<Scope> {
        self.0.scope()
    }

    fn operation(&self) -> boardstudio_application::OperationId {
        self.0.operation()
    }

    fn observe_operation(
        &self,
        operation: boardstudio_application::OperationId,
    ) -> crate::operation_outcomes::OutcomeSlot {
        self.0.observe_operation(operation)
    }

    fn submit(&self, event: Event) {
        self.0.submit(event);
    }

    fn begin_edit(
        &self,
        label: &str,
        feature: Option<String>,
        resolver: EditResolver,
    ) -> EditTicket {
        EditTicket::begin(&self.0, label, feature, resolver)
    }
}

pub fn runtime_adapter(runtime: Rc<Runtime>) -> Rc<dyn PlacementRuntime> {
    Rc::new(RuntimePlacementAdapter(runtime))
}

struct PlacementAdmission {
    scope: Option<Scope>,
    generation: u64,
    workspace: &'static str,
    expected_workspace: &'static str,
    preferences: Option<SetupGuidePreferences>,
}

impl PlacementAdmission {
    fn capture(
        runtime: &dyn PlacementRuntime,
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
}

#[derive(Clone)]
struct ComponentActionOwner {
    accepted: AcceptedSnapshot,
    scope: Scope,
    generation: u64,
    workspace: &'static str,
    selected_context: Option<ScopedTreeContext>,
}

impl ComponentActionOwner {
    fn capture(
        runtime: &dyn PlacementRuntime,
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

    fn is_current(
        &self,
        runtime: &dyn PlacementRuntime,
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

fn accepted_snapshot_is_current(
    runtime: &dyn PlacementRuntime,
    accepted: &AcceptedSnapshot,
) -> bool {
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

pub fn use_controller_placement(host: PartPlacementHost) -> PartPlacementMount {
    let PartPlacementHost {
        runtime,
        load_definition,
        workspace,
        generation,
        version,
        adapter,
        mut layout_selection_kind,
        guide_preferences,
        parts_query,
        parts_selection,
        snap_settings,
        layout_target,
        canvas_center,
        mut objects_open,
        mut inspect_open,
        canvas_interaction,
    } = host;
    let chooser = use_signal(|| None::<ControllerChooserOwner>);
    let active = use_signal(|| None::<ActivePartPlacement>);
    let preparing = use_signal(|| None::<PlacementOwner>);
    let committing = use_signal(|| None::<EditTicket>);
    let mut key_edit = use_signal(|| None::<PendingKeyEdit>);
    let error = use_signal(|| None::<String>);
    let alive = use_hook(|| Rc::new(Cell::new(true)));
    use_drop({
        let alive = alive.clone();
        move || alive.set(false)
    });

    let action_owner = ComponentActionOwner::capture(
        runtime.as_ref(),
        generation(),
        workspace(),
        (adapter.selected_context)(),
    );
    let on_choose_controller = {
        let runtime = runtime.clone();
        let mut workspace = workspace;
        let mut query = parts_query;
        let mut selected = parts_selection;
        let guide = guide_preferences;
        let entry_owner = action_owner.clone();
        let mut chooser = chooser;
        let mut selected_context = adapter.selected_context;
        let mut anchor_scope = adapter.anchor_scope;
        let canvas_interaction = canvas_interaction.clone();
        move |_| {
            if canvas_interaction.current().is_some() {
                return;
            }
            let model = runtime.model();
            let Some(project_id) = model
                .accepted
                .as_ref()
                .map(|snapshot| snapshot.document.id.clone())
            else {
                return;
            };
            let guided = guide().as_ref().is_some_and(|preferences| {
                preferences.project_id == project_id
                    && preferences.open
                    && preferences.current_stage == SetupGuideStage::Wiring
            });
            let Some(owner) = entry_owner.as_ref().filter(|owner| {
                owner.is_current(
                    runtime.as_ref(),
                    generation(),
                    workspace(),
                    selected_context(),
                    owner.workspace,
                )
            }) else {
                return;
            };
            if !guided && owner.workspace != "PCB" {
                return;
            }
            chooser.set((!guided).then(|| ControllerChooserOwner {
                accepted: owner.accepted.clone(),
                scope: owner.scope.clone(),
                generation: owner.generation,
            }));
            workspace.set("Parts");
            query.set("controller".into());
            selected.set(None);
            selected_context.set(None);
            anchor_scope.set(None);
            objects_open.set(true);
            inspect_open.set(!guided);
            runtime.submit(Event::SelectParts {
                operation_id: runtime.operation(),
                part_ids: Vec::new(),
                range_part_ids: Vec::new(),
                mode: SelectionMode::Replace,
            });
        }
    };

    let start_placement = {
        let runtime = runtime.clone();
        let mut active = active;
        let query = parts_query;
        let load_definition = load_definition.clone();
        let mut preparing = preparing;
        let mut error = error;
        let alive = alive.clone();
        let adapter_for_async = adapter.clone();
        let canvas_interaction = canvas_interaction.clone();
        Rc::new(RefCell::new(
            move |definition_id: String,
                  kind: PartKind,
                  workflow: PlacementWorkflow,
                  apply_to_key: bool,
                  source_definition: Option<(String, PartDefinition)>| {
                let source_workspace = workspace();
                if match workflow {
                    PlacementWorkflow::WiringController | PlacementWorkflow::PcbController => {
                        source_workspace != "Parts"
                    }
                    PlacementWorkflow::GeneralComponent => {
                        !matches!(source_workspace, "Parts" | "Layout" | "PCB")
                    }
                } {
                    return;
                }
                let current_model = runtime.model();
                let admission = PlacementAdmission::capture(
                    &runtime,
                    generation(),
                    workspace(),
                    "Layout",
                    guide_preferences(),
                );
                if active.read().as_ref().is_some_and(|placement| {
                    !owner_is_live(&placement.owner, &runtime, &current_model, admission)
                }) {
                    active.set(None);
                }
                let current_model = runtime.model();
                let admission = PlacementAdmission::capture(
                    &runtime,
                    generation(),
                    workspace(),
                    preparing
                        .read()
                        .as_ref()
                        .map_or("Parts", |owner| owner.source_workspace),
                    guide_preferences(),
                );
                if preparing
                    .read()
                    .as_ref()
                    .is_some_and(|owner| !owner_is_live(owner, &runtime, &current_model, admission))
                {
                    preparing.set(None);
                }
                if active.read().is_some()
                    || preparing.read().is_some()
                    || committing.read().is_some()
                    || key_edit.read().is_some()
                    || workspace() != source_workspace
                {
                    return;
                }
                let model = runtime.model();
                let Some(snapshot) = model.accepted.as_ref() else {
                    return;
                };
                let Some(scope) = runtime.scope() else {
                    return;
                };
                if apply_to_key && source_definition.is_some() {
                    error.set(Some(
                        "A module source component cannot be applied to a matrix key.".into(),
                    ));
                    return;
                }
                let source_definition = source_definition.map(|(module_id, definition)| {
                    placement_source_definition(&snapshot.document, &module_id, definition)
                });
                let definition_id = source_definition
                    .as_ref()
                    .map(|definition| definition.id.clone())
                    .unwrap_or(definition_id);
                let kind = source_definition
                    .as_ref()
                    .map(|definition| definition.kind.clone())
                    .unwrap_or(kind);
                let guide_is_live = guide_preferences().as_ref().is_some_and(|preferences| {
                    preferences.open
                        && preferences.current_stage == SetupGuideStage::Wiring
                        && preferences.project_id == snapshot.document.id
                });
                let pcb_chooser_is_live = chooser.read().as_ref().is_some_and(|owner| {
                    owner.is_current(runtime.as_ref(), generation(), source_workspace)
                });
                if (workflow == PlacementWorkflow::WiringController && !guide_is_live)
                    || (workflow == PlacementWorkflow::PcbController && !pcb_chooser_is_live)
                    || model.lifecycle != Lifecycle::Ready
                    || model.durability
                        != (Durability::Saved {
                            revision: snapshot.document.revision,
                        })
                    || model.display_preview.is_some()
                    || model.gesture.is_some()
                    || model.active_board_id != scope.board_id
                {
                    return;
                }
                let selected_context = (adapter_for_async.selected_context)();
                if apply_to_key
                    && selected_context
                        .as_ref()
                        .is_some_and(|selected| matches!(selected.context, TreeContext::Key { .. }))
                {
                    let Some(selected) = selected_context else {
                        return;
                    };
                    if selected.scope != scope
                        || !super::selection::context_is_current(&model, &scope, &selected.context)
                    {
                        return;
                    }
                    let accepted = snapshot.clone();
                    let accepted_generation = generation();
                    let reversible_document = (*snapshot.document).clone();
                    let runtime = runtime.clone();
                    let load_definition = load_definition.clone();
                    let mut error = error;
                    let current_context = adapter_for_async.selected_context;
                    let alive = alive.clone();
                    spawn_local(async move {
                        let definition =
                            match load_definition(reversible_document, definition_id).await {
                                Ok(definition) => definition,
                                Err(message) => {
                                    if alive.get()
                                        && accepted_snapshot_is_current(runtime.as_ref(), &accepted)
                                        && runtime.scope().as_ref() == Some(&scope)
                                        && workspace() == source_workspace
                                        && generation() == accepted_generation
                                        && current_context() == Some(selected.clone())
                                    {
                                        error.set(Some(message));
                                    }
                                    return;
                                }
                            };
                        if !alive.get() {
                            return;
                        }
                        if definition.kind != kind {
                            if accepted_snapshot_is_current(runtime.as_ref(), &accepted)
                                && runtime.scope().as_ref() == Some(&scope)
                                && workspace() == source_workspace
                                && generation() == accepted_generation
                                && current_context() == Some(selected.clone())
                            {
                                error.set(Some(
                                    "The selected component changed before it was applied.".into(),
                                ));
                            }
                            return;
                        }
                        if !alive.get()
                            || runtime.scope().as_ref() != Some(&scope)
                            || current_context() != Some(selected.clone())
                            || workspace() != source_workspace
                            || generation() != accepted_generation
                        {
                            return;
                        }
                        let ticket = runtime.begin_edit(
                            "layout-key-component",
                            Some("component".into()),
                            key_component_resolver(scope.clone(), selected.clone(), definition),
                        );
                        let pending = PendingKeyEdit {
                            scope: scope.clone(),
                            generation: accepted_generation,
                            source_workspace,
                            selection: selected.clone(),
                        };
                        key_edit.set(Some(pending.clone()));
                        error.set(None);
                        let runtime = runtime.clone();
                        let mut key_edit = key_edit;
                        let mut error = error;
                        let current_context = current_context;
                        let alive = alive.clone();
                        spawn_local(async move {
                            while ticket.is_pending() {
                                gloo_timers::future::TimeoutFuture::new(16).await;
                            }
                            if !alive.get() {
                                return;
                            }
                            let still_current = runtime.scope().as_ref() == Some(&pending.scope)
                                && workspace() == pending.source_workspace
                                && generation() == pending.generation
                                && current_context() == Some(pending.selection.clone());
                            key_edit.set(None);
                            if let Settlement::Failed { message } =
                                ticket.settlement(still_current)
                            {
                                error.set(Some(message));
                            }
                        });
                    });
                    return;
                }
                let part_id = match browser_uuid() {
                    Ok(id) => format!("ui-{id}"),
                    Err(message) => {
                        error.set(Some(message));
                        return;
                    }
                };
                let at = grid_snap_point(canvas_center, snap_settings.read().snap_fraction);
                let layout_id =
                    layout_target().filter(|layout_id| {
                        snapshot.document.layouts.iter().any(|layout| {
                            layout.id == *layout_id && layout.board_id == scope.board_id
                        })
                    });
                let Some(owner) = PlacementOwner::capture(
                    snapshot,
                    PlacementCapture {
                        scope: scope.clone(),
                        generation: generation(),
                        part_id,
                        definition_id: definition_id.clone(),
                        kind: kind.clone(),
                        workflow,
                        source_workspace,
                        layout_id,
                        at,
                    },
                ) else {
                    return;
                };
                if !canvas_interaction.try_acquire(CanvasInteractionOwner::PartPlacement) {
                    return;
                }
                preparing.set(Some(owner.clone()));
                error.set(None);
                let accepted = snapshot.clone();
                let reversible_document = (*snapshot.document).clone();
                let runtime = runtime.clone();
                let mut workspace = workspace;
                let mut active = active;
                let mut error = error;
                let alive = alive.clone();
                let mut adapter = adapter_for_async.clone();
                let guide = guide_preferences;
                let mut query = query;
                let mut preparing = preparing;
                let load_definition = load_definition.clone();
                let source_definition = source_definition.clone();
                spawn_local(async move {
                    let definition = match source_definition {
                        Some(definition) => Ok(definition),
                        None => load_definition(reversible_document, definition_id).await,
                    };
                    if !alive.get() {
                        return;
                    }
                    if preparing.read().as_ref() != Some(&owner) {
                        return;
                    }
                    let model = runtime.model();
                    let still_owned = owner_is_live(
                        &owner,
                        &runtime,
                        &model,
                        PlacementAdmission::capture(
                            &runtime,
                            generation(),
                            workspace(),
                            owner.source_workspace,
                            guide(),
                        ),
                    );
                    if !still_owned
                        || workspace() != owner.source_workspace
                        || model.lifecycle != Lifecycle::Ready
                        || model.durability
                            != (Durability::Saved {
                                revision: accepted.document.revision,
                            })
                    {
                        preparing.set(None);
                        return;
                    }
                    let definition = match definition {
                        Ok(definition) => definition,
                        Err(message) => {
                            preparing.set(None);
                            error.set(Some(message));
                            return;
                        }
                    };
                    if definition.kind != kind {
                        preparing.set(None);
                        error.set(Some(
                            "The selected component changed before placement.".into(),
                        ));
                        return;
                    }
                    if workflow.is_controller() && !matches!(definition.kind, PartKind::Controller)
                    {
                        preparing.set(None);
                        error.set(Some(
                            "The selected catalogue item is not a controller.".into(),
                        ));
                        return;
                    }
                    let pending = if workflow.is_controller() {
                        controller_part(
                            definition,
                            owner.part_id.clone(),
                            owner.reference.clone(),
                            owner.at,
                        )
                    } else {
                        Some(component_part(
                            definition,
                            owner.part_id.clone(),
                            owner.reference.clone(),
                            owner.at,
                        ))
                    };
                    let Some(pending) = pending else {
                        preparing.set(None);
                        error.set(Some(
                            "The selected catalogue item is not a controller.".into(),
                        ));
                        return;
                    };
                    if !owner_is_live(
                        &owner,
                        &runtime,
                        &model,
                        PlacementAdmission::capture(
                            &runtime,
                            generation(),
                            workspace(),
                            owner.source_workspace,
                            guide(),
                        ),
                    ) {
                        preparing.set(None);
                        return;
                    }
                    active.set(Some(ActivePartPlacement {
                        owner,
                        snap_document: Rc::new(document_with_pending_definition(
                            &accepted.document,
                            &pending.definition,
                        )),
                        pending,
                    }));
                    preparing.set(None);
                    if workflow.is_controller() {
                        query.set("controller".into());
                    }
                    workspace.set("Layout");
                    adapter.selected_context.set(None);
                    adapter.anchor_scope.set(None);
                    runtime.submit(Event::SelectParts {
                        operation_id: runtime.operation(),
                        part_ids: Vec::new(),
                        range_part_ids: Vec::new(),
                        mode: SelectionMode::Replace,
                    });
                });
            },
        ))
    };
    let ordinary_controller = chooser
        .read()
        .as_ref()
        .is_some_and(|owner| owner.is_current(runtime.as_ref(), generation(), workspace()));
    let guided_controller = guide_preferences().as_ref().is_some_and(|preferences| {
        preferences.open
            && preferences.current_stage == SetupGuideStage::Wiring
            && runtime
                .model()
                .accepted
                .as_ref()
                .is_some_and(|accepted| preferences.project_id == accepted.document.id)
    });
    let on_place_controller = {
        let start = start_placement.clone();
        let runtime = runtime.clone();
        let controller_action_owner = action_owner.clone();
        let selected_context = adapter.selected_context;
        let workflow = if ordinary_controller {
            PlacementWorkflow::PcbController
        } else {
            PlacementWorkflow::WiringController
        };
        move |definition_id: String| {
            if workflow == PlacementWorkflow::PcbController
                && !controller_action_owner.as_ref().is_some_and(|owner| {
                    owner.is_current(
                        runtime.as_ref(),
                        generation(),
                        workspace(),
                        selected_context(),
                        "Parts",
                    )
                })
            {
                return;
            }
            start.borrow_mut()(definition_id, PartKind::Controller, workflow, false, None);
        }
    };
    let controller_back =
        ordinary_controller.then(|| {
            let runtime = runtime.clone();
            let mut workspace = workspace;
            let mut chooser = chooser;
            let canvas_interaction = canvas_interaction.clone();
            EventHandler::new(move |_| {
                if chooser.read().as_ref().is_some_and(|owner| {
                    owner.is_current(runtime.as_ref(), generation(), workspace())
                }) && canvas_interaction.current().is_none()
                {
                    chooser.set(None);
                    workspace.set("PCB");
                }
            })
        });
    let on_place_component = {
        let start = start_placement;
        let runtime = runtime.clone();
        let selected_context = adapter.selected_context;
        move |action: ComponentPlacementAction| {
            let expected_workspace = match action {
                ComponentPlacementAction::AddObject { .. }
                | ComponentPlacementAction::AddSourceObject { .. } => {
                    let Some(owner) = action_owner
                        .as_ref()
                        .filter(|owner| matches!(owner.workspace, "Layout" | "PCB"))
                    else {
                        return;
                    };
                    owner.workspace
                }
                ComponentPlacementAction::PartsInspector { .. } => "Parts",
            };
            if !action_owner.as_ref().is_some_and(|owner| {
                owner.is_current(
                    runtime.as_ref(),
                    generation(),
                    workspace(),
                    selected_context(),
                    expected_workspace,
                )
            }) {
                return;
            }
            match action {
                ComponentPlacementAction::AddObject {
                    definition_id,
                    kind,
                } => start.borrow_mut()(
                    definition_id,
                    kind,
                    PlacementWorkflow::GeneralComponent,
                    false,
                    None,
                ),
                ComponentPlacementAction::AddSourceObject {
                    module_definition_id,
                    definition,
                } => start.borrow_mut()(
                    definition.id.clone(),
                    definition.kind.clone(),
                    PlacementWorkflow::GeneralComponent,
                    false,
                    Some((module_definition_id, definition)),
                ),
                ComponentPlacementAction::PartsInspector {
                    definition_id,
                    kind,
                } => start.borrow_mut()(
                    definition_id,
                    kind,
                    PlacementWorkflow::GeneralComponent,
                    true,
                    None,
                ),
            }
        }
    };

    let observed_version = version();
    let observed_generation = generation();
    let observed_workspace = workspace();
    let observed_guide = guide_preferences();
    let cleanup_runtime = runtime.clone();
    let mut cleanup_active = active;
    let mut cleanup_preparing = preparing;
    let mut cleanup_chooser = chooser;
    use_effect(use_reactive!(
        |observed_version, observed_generation, observed_workspace, observed_guide| {
            let _ = observed_version;
            let stale_chooser = cleanup_chooser.read().as_ref().is_some_and(|owner| {
                !owner.is_current(
                    cleanup_runtime.as_ref(),
                    observed_generation,
                    observed_workspace,
                )
            });
            if stale_chooser {
                cleanup_chooser.set(None);
            }
            let model = cleanup_runtime.model();
            let stale_active = cleanup_active.read().as_ref().is_some_and(|placement| {
                !owner_is_live(
                    &placement.owner,
                    &cleanup_runtime,
                    &model,
                    PlacementAdmission::capture(
                        &cleanup_runtime,
                        observed_generation,
                        observed_workspace,
                        "Layout",
                        observed_guide.clone(),
                    ),
                )
            });
            if stale_active {
                cleanup_active.set(None);
            }
            let stale_preparing = cleanup_preparing.read().as_ref().is_some_and(|owner| {
                !owner_is_live(
                    owner,
                    &cleanup_runtime,
                    &model,
                    PlacementAdmission::capture(
                        &cleanup_runtime,
                        observed_generation,
                        observed_workspace,
                        owner.source_workspace,
                        observed_guide.clone(),
                    ),
                )
            });
            if stale_preparing {
                cleanup_preparing.set(None);
            }
        }
    ));
    let on_move = {
        let mut active = active;
        let canvas_interaction = canvas_interaction.clone();
        move |at: Vec2| {
            if !canvas_interaction.is_owner(CanvasInteractionOwner::PartPlacement) {
                return;
            }
            if let Some(mut placement) = active() {
                update_pending_part(&mut placement.pending, at);
                active.set(Some(placement));
            }
        }
    };
    let on_commit = {
        let runtime = runtime.clone();
        let mut active = active;
        let mut committing = committing;
        let mut error = error;
        let alive = alive.clone();
        let canvas_interaction = canvas_interaction.clone();
        move |at: Vec2| {
            if !canvas_interaction.is_owner(CanvasInteractionOwner::PartPlacement) {
                return;
            }
            let Some(mut placement) = active() else {
                return;
            };
            if committing.read().is_some() {
                return;
            }
            update_pending_part(&mut placement.pending, at);
            placement
                .owner
                .record_committed_position(placement.pending.at);
            let model = runtime.model();
            if model.accepted.is_none() {
                return;
            }
            if !owner_is_live(
                &placement.owner,
                &runtime,
                &model,
                PlacementAdmission::capture(
                    &runtime,
                    generation(),
                    workspace(),
                    "Layout",
                    guide_preferences(),
                ),
            ) || model.lifecycle != Lifecycle::Ready
                || model.durability
                    != (Durability::Saved {
                        revision: placement.owner.revision,
                    })
            {
                return;
            }
            let owner = placement.owner.clone();
            let ticket = runtime.begin_edit(
                "layout-component-placement",
                Some("placement".into()),
                placement_resolver(
                    owner.board_id.clone(),
                    placement.pending.definition.clone(),
                    placement.pending.part.clone(),
                    owner.layout_id.clone(),
                ),
            );
            // One-shot: the canvas stays busy while the placement ticket is held.
            committing.set(Some(ticket.clone()));
            active.set(None);
            error.set(None);
            let runtime = runtime.clone();
            let mut committing = committing;
            let mut error = error;
            let mut workspace = workspace;
            let guide_preferences = guide_preferences;
            let mut selected_context = adapter.selected_context;
            let mut anchor_scope = adapter.anchor_scope;
            let canvas_interaction = canvas_interaction.clone();
            let alive = alive.clone();
            spawn_local(async move {
                while ticket.is_pending() {
                    gloo_timers::future::TimeoutFuture::new(16).await;
                }
                if !alive.get() {
                    return;
                }
                let model = runtime.model();
                let route_live = placement_route_is_current(
                    &runtime,
                    &model,
                    &owner,
                    generation(),
                    workspace(),
                    guide_preferences(),
                );
                let settlement = ticket.settlement(route_live);
                committing.set(None);
                match settlement {
                    Settlement::Landed { .. } => {
                        // Select from the accepted document at the landing; nothing when the
                        // placed part is no longer there.
                        let placed = model.accepted.as_ref().is_some_and(|accepted| {
                            accepted
                                .document
                                .parts
                                .iter()
                                .any(|part| part.id == owner.part_id)
                        });
                        let next_context = if owner.workflow == PlacementWorkflow::GeneralComponent
                            && placed
                        {
                            objects::context_for_part(&model, &owner.part_id).map(|context| {
                                ScopedTreeContext {
                                    scope: owner.scope.clone(),
                                    context,
                                }
                            })
                        } else {
                            None
                        };
                        selected_context.set(next_context);
                        anchor_scope.set(None);
                        workspace.set(if owner.workflow.is_controller() {
                            "PCB"
                        } else {
                            "Layout"
                        });
                        if owner.workflow == PlacementWorkflow::GeneralComponent {
                            layout_selection_kind.set(objects::LayoutSelectionKind::Part);
                        }
                        runtime.submit(Event::SelectParts {
                            operation_id: runtime.operation(),
                            part_ids: if owner.workflow == PlacementWorkflow::GeneralComponent
                                && placed
                            {
                                vec![owner.part_id.clone()]
                            } else {
                                Vec::new()
                            },
                            range_part_ids: Vec::new(),
                            mode: SelectionMode::Replace,
                        });
                    }
                    Settlement::Failed { message } => {
                        workspace.set(if owner.workflow.is_controller() {
                            "Parts"
                        } else {
                            "Layout"
                        });
                        error.set(Some(message));
                    }
                    Settlement::Retired | Settlement::Pending => {}
                }
                canvas_interaction.release(CanvasInteractionOwner::PartPlacement);
            });
        }
    };
    let on_cancel = {
        let runtime = runtime.clone();
        let mut active = active;
        let mut preparing = preparing;
        let mut workspace = workspace;
        let guide = guide_preferences;
        let mut selected_context = adapter.selected_context;
        let mut anchor_scope = adapter.anchor_scope;
        let canvas_interaction = canvas_interaction.clone();
        move |_| {
            if !canvas_interaction.is_owner(CanvasInteractionOwner::PartPlacement) {
                return;
            }
            if committing.read().is_some() {
                return;
            }
            let owner_and_workspace = active
                .read()
                .as_ref()
                .map(|placement| (placement.owner.clone(), "Layout"))
                .or_else(|| {
                    preparing
                        .read()
                        .clone()
                        .map(|owner| (owner.clone(), owner.source_workspace))
                });
            let Some((owner, owner_workspace)) = owner_and_workspace else {
                return;
            };
            let model = runtime.model();
            let current = model.accepted.as_ref().is_some_and(|_snapshot| {
                owner_is_live(
                    &owner,
                    &runtime,
                    &model,
                    PlacementAdmission::capture(
                        &runtime,
                        generation(),
                        workspace(),
                        owner_workspace,
                        guide(),
                    ),
                )
            });
            active.set(None);
            preparing.set(None);
            canvas_interaction.release(CanvasInteractionOwner::PartPlacement);
            if current {
                selected_context.set(None);
                anchor_scope.set(None);
                workspace.set(if owner.workflow.is_controller() {
                    "PCB"
                } else {
                    "Layout"
                });
                runtime.submit(Event::SelectParts {
                    operation_id: runtime.operation(),
                    part_ids: Vec::new(),
                    range_part_ids: Vec::new(),
                    mode: SelectionMode::Replace,
                });
            }
        }
    };
    let on_move = EventHandler::new(on_move);
    let on_commit = EventHandler::new(on_commit);
    let projection = active().filter(|placement| {
        let model = runtime.model();
        owner_is_live(
            &placement.owner,
            &runtime,
            &model,
            PlacementAdmission::capture(
                &runtime,
                generation(),
                workspace(),
                "Layout",
                guide_preferences(),
            ),
        )
    });
    let busy =
        preparing.read().is_some() || committing.read().is_some() || key_edit.read().is_some();
    let owns_canvas = busy || projection.is_some();
    use_effect(use_reactive(
        (&version(), &workspace(), &generation(), &owns_canvas),
        {
            let canvas_interaction = canvas_interaction.clone();
            move |(_, _, _, owns_canvas)| {
                if !owns_canvas {
                    canvas_interaction.release(CanvasInteractionOwner::PartPlacement);
                }
            }
        },
    ));
    PartPlacementMount {
        projection,
        busy,
        error: error(),
        on_choose_controller: EventHandler::new(on_choose_controller),
        on_place_controller: EventHandler::new(on_place_controller),
        controller_placement_enabled: ordinary_controller || guided_controller,
        controller_back,
        on_place_component: EventHandler::new(on_place_component),
        on_move,
        on_commit,
        on_cancel: EventHandler::new(on_cancel),
    }
}

fn owner_is_live(
    owner: &PlacementOwner,
    runtime: &dyn PlacementRuntime,
    model: &boardstudio_application::ReadModel,
    admission: PlacementAdmission,
) -> bool {
    let Some(snapshot) = model.accepted.as_ref() else {
        return false;
    };
    owner.is_current(
        snapshot,
        admission.scope.as_ref(),
        admission.generation,
        admission
            .preferences
            .as_ref()
            .is_some_and(|preferences| preferences.open),
        admission
            .preferences
            .as_ref()
            .map(|preferences| preferences.project_id.as_str()),
        admission
            .preferences
            .as_ref()
            .is_some_and(|preferences| preferences.current_stage == SetupGuideStage::Wiring),
    ) && model.active_board_id == owner.board_id
        && runtime.scope().as_ref() == Some(&owner.scope)
        && admission.workspace == admission.expected_workspace
}

fn placement_route_is_current(
    runtime: &dyn PlacementRuntime,
    model: &boardstudio_application::ReadModel,
    owner: &PlacementOwner,
    generation: u64,
    workspace: &'static str,
    preferences: Option<SetupGuidePreferences>,
) -> bool {
    model.accepted.as_ref().is_some_and(|accepted| {
        accepted.session_epoch == owner.session_epoch
            && accepted.document.id == owner.project_id
            && model.active_board_id == owner.board_id
            && model.active_instance_id == owner.scope.instance_id
            && runtime.scope().as_ref() == Some(&owner.scope)
            && generation == owner.generation
            && workspace == "Layout"
    }) && (owner.workflow != PlacementWorkflow::WiringController
        || preferences.as_ref().is_some_and(|preferences| {
            preferences.open
                && preferences.project_id == owner.project_id
                && preferences.current_stage == SetupGuideStage::Wiring
        }))
}

fn browser_uuid() -> Result<String, String> {
    let crypto = js_sys::Reflect::get(&js_sys::global(), &JsValue::from_str("crypto"))
        .map_err(|error| format!("Could not access browser identity service: {error:?}"))?;
    let random_uuid = js_sys::Reflect::get(&crypto, &JsValue::from_str("randomUUID"))
        .map_err(|error| format!("Could not access browser identity service: {error:?}"))?
        .dyn_into::<js_sys::Function>()
        .map_err(|_| "The browser cannot generate a safe component identity.".to_string())?;
    random_uuid
        .call0(&crypto)
        .map_err(|error| format!("Could not generate a component identity: {error:?}"))?
        .as_string()
        .ok_or_else(|| "Could not generate a component identity.".into())
}

pub fn controller_part(
    definition: PartDefinition,
    part_id: String,
    reference: String,
    at: Vec2,
) -> Option<PendingPart> {
    matches!(definition.kind, PartKind::Controller)
        .then(|| component_part(definition, part_id, reference, at))
}

pub fn component_part(
    definition: PartDefinition,
    part_id: String,
    reference: String,
    at: Vec2,
) -> PendingPart {
    PendingPart {
        part: Part {
            keycap: None,
            outline: None,
            id: part_id,
            definition_id: definition.id.clone(),
            reference,
            pose: Pose2 { at, rotation: 0.0 },
            side: Side::Front,
            locked: None,
            properties: None,
            generator_parameters: definition
                .generator
                .as_ref()
                .map(|generator| generator.parameters.clone()),
        },
        definition,
        at,
    }
}

fn placement_source_definition(
    accepted: &ProjectDoc,
    module_definition_id: &str,
    source: PartDefinition,
) -> PartDefinition {
    if !accepted
        .definitions
        .iter()
        .any(|definition| definition.id == source.id && definition != &source)
    {
        return source;
    }

    let base_id = format!(
        "module-source/{}/{}:{}/{}",
        module_definition_id.len(),
        module_definition_id,
        source.id.len(),
        source.id
    );
    for suffix in 0u32.. {
        let id = if suffix == 0 {
            base_id.clone()
        } else {
            format!("{base_id}/{suffix}")
        };
        let mut candidate = source.clone();
        candidate.id = id;
        if let Some(profile) = &mut candidate.mechanical_profile {
            profile.definition_id = candidate.id.clone();
        }
        match accepted
            .definitions
            .iter()
            .find(|definition| definition.id == candidate.id)
        {
            Some(existing) if existing == &candidate => return candidate,
            Some(_) => continue,
            None => return candidate,
        }
    }
    unreachable!("A finite project cannot exhaust component definition identities")
}

pub fn update_pending_part(pending: &mut PendingPart, at: Vec2) {
    pending.at = at;
    pending.part.pose.at = at;
}

pub fn canvas_world_center(
    min_x: f64,
    max_x: f64,
    min_y: f64,
    max_y: f64,
    camera_pan: Vec2,
) -> Vec2 {
    Vec2 {
        x: (min_x + max_x) * 0.5 + camera_pan.x,
        y: (min_y + max_y) * 0.5 + camera_pan.y,
    }
}

pub fn pointer_release_commits(button: i16) -> bool {
    button == 0
}

fn document_with_pending_definition(
    document: &ProjectDoc,
    definition: &PartDefinition,
) -> ProjectDoc {
    let mut snap_document = document.clone();
    if !snap_document
        .definitions
        .iter()
        .any(|existing| existing.id == definition.id)
    {
        snap_document.definitions.push(definition.clone());
    }
    snap_document
}

fn grid_snap_point(at: Vec2, snap_fraction: f64) -> Vec2 {
    if snap_fraction == 0.0 {
        return at;
    }
    let step = if snap_fraction.is_finite() {
        if snap_fraction < 0.0 {
            -snap_fraction
        } else {
            19.05 * snap_fraction
        }
    } else {
        19.05 * 0.25
    }
    .max(0.001);
    Vec2 {
        x: (at.x / step).round() * step,
        y: (at.y / step).round() * step,
    }
}

pub struct PlacementSnapOptions {
    pub snap_fraction: f64,
    pub geometry_snap: bool,
    pub gap: Option<f64>,
    pub free: bool,
}

pub fn placement_gap(settings: &LayoutSnapSettings) -> Option<f64> {
    objects::gesture_snap_inputs(settings, None, None).gap
}

pub fn snap_placement_at(
    document: &ProjectDoc,
    board_id: &str,
    pending: &PendingPart,
    at: Vec2,
    options: PlacementSnapOptions,
) -> Vec2 {
    if options.free {
        return at;
    }
    let grid = grid_snap_point(at, options.snap_fraction);
    if !options.geometry_snap {
        return grid;
    }
    let mut moving = pending.part.clone();
    moving.pose.at = at;
    boardstudio_application::interactions::snap_part_in_board(
        document,
        board_id,
        &moving,
        2.0,
        options.gap,
    )
    .map_or(grid, |guide| guide.at)
}

pub fn placement_operation(
    accepted: &ProjectDoc,
    board_id: &str,
    definition: &PartDefinition,
    part: &Part,
    layout_id: Option<&str>,
) -> Result<EditOperation, String> {
    if part.definition_id != definition.id || part.id.is_empty() || part.reference.is_empty() {
        return Err("The prepared component placement is incomplete.".into());
    }
    if accepted.parts.iter().any(|existing| existing.id == part.id) {
        return Err("The component placement identity already exists.".into());
    }
    let board_index = accepted
        .boards
        .iter()
        .position(|board| board.id == board_id)
        .ok_or_else(|| "The selected board is no longer available.".to_string())?;
    if accepted
        .parts
        .iter()
        .any(|existing| existing.reference == part.reference)
    {
        return Err("The component reference is already in use.".into());
    }

    let mut proposed = accepted.clone();
    match proposed
        .definitions
        .iter()
        .find(|existing| existing.id == definition.id)
    {
        Some(existing) if existing != definition => {
            return Err("The selected component definition changed before placement.".into());
        }
        Some(_) => {}
        None => proposed.definitions.push(definition.clone()),
    }
    proposed.parts.push(part.clone());
    let board = &mut proposed.boards[board_index];
    if !board.part_ids.contains(&part.id) {
        board.part_ids.push(part.id.clone());
    }
    let board_outlines = board
        .outline_ids
        .iter()
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    for outline in &mut proposed.outline {
        if let OutlineFeature::PartEnvelope { id, part_ids, .. } = outline
            && board_outlines.contains(id)
            && !part_ids.contains(&part.id)
        {
            part_ids.push(part.id.clone());
        }
    }
    if let Some(layout_id) = layout_id {
        let layout = proposed
            .layouts
            .iter_mut()
            .find(|layout| layout.id == layout_id && layout.board_id == board_id)
            .ok_or_else(|| {
                "The selected layout is no longer available on this board.".to_string()
            })?;
        if !layout.part_ids.contains(&part.id) {
            layout.part_ids.push(part.id.clone());
        }
    }
    Ok(EditOperation::ReplaceDocument {
        document: Box::new(proposed),
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
                && super::objects::matrix_visible_on_board(accepted, &scope.board_id, matrix_id)
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
        || super::parts::matrix_input_available(definition);
    let mut cell = current.clone().unwrap_or(MatrixCell {
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
        let id = format!("library-{}-{}", definition.id, cell.assemblies.len() + 1);
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
    let is_mirror_target = accepted
        .layouts
        .iter()
        .any(|layout| layout.matrix_id == *matrix_id && layout.mirror_link.is_some());
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

pub fn next_component_reference(document: &ProjectDoc, kind: &PartKind) -> String {
    let prefix = match kind {
        PartKind::Switch => 'S',
        PartKind::Controller => 'U',
        PartKind::Encoder => 'E',
        PartKind::Connector | PartKind::Passive | PartKind::Custom | PartKind::Utility => 'J',
    };
    let occupied = document
        .parts
        .iter()
        .filter_map(|part| part.reference.strip_prefix(prefix))
        .filter_map(|number| number.parse::<u32>().ok())
        .collect::<std::collections::BTreeSet<_>>();
    let next = (1..=u32::MAX)
        .find(|candidate| !occupied.contains(candidate))
        .unwrap_or(1);
    format!("{prefix}{next}")
}

#[cfg(test)]
mod tests {
    wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

    use super::*;
    use boardstudio_application::{
        Event as SessionEvent, OperationId, ReadModel, SessionEpoch, SnapshotToken,
        TerminalOutcome,
    };
    use boardstudio_core::model::{Board, EditCommand, OutlineSettings};
    use std::{
        cell::{Cell, RefCell},
        sync::Arc,
        task::{Context, Waker},
    };

    fn controller_definition(id: &str) -> PartDefinition {
        serde_json::from_value(serde_json::json!({
            "id": id,
            "name": "Controller",
            "kind": "controller",
            "courtyard": [{"x": -5.0, "y": -5.0}, {"x": 5.0, "y": -5.0}, {"x": 5.0, "y": 5.0}],
            "pads": []
        }))
        .unwrap()
    }

    fn passive_definition(id: &str) -> PartDefinition {
        serde_json::from_value(serde_json::json!({
            "id": id,
            "name": "Reset switch",
            "kind": "passive",
            "courtyard": [{"x": -3.0, "y": -3.0}, {"x": 3.0, "y": -3.0}, {"x": 3.0, "y": 3.0}],
            "pads": []
        }))
        .unwrap()
    }

    fn switch_definition(id: &str) -> PartDefinition {
        serde_json::from_value(serde_json::json!({
            "id": id,
            "name": "MX switch",
            "kind": "switch",
            "courtyard": [{"x": -3.0, "y": -3.0}, {"x": 3.0, "y": -3.0}, {"x": 3.0, "y": 3.0}],
            "pads": []
        }))
        .unwrap()
    }

    fn fixture() -> ProjectDoc {
        let mut document = ProjectDoc::empty("project", "Keyboard");
        document.outline.push(OutlineFeature::PartEnvelope {
            connections: vec![],
            settings: OutlineSettings::default(),
            id: "envelope-main".into(),
            part_ids: vec![],
            margin: 4.0,
            operation: boardstudio_core::model::Operation::Add,
        });
        document.boards.push(Board {
            id: "board-main".into(),
            name: "Main".into(),
            outline_ids: vec!["envelope-main".into()],
            part_ids: vec![],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        document
    }

    fn matrix_fixture() -> boardstudio_core::model::Matrix {
        serde_json::from_value(serde_json::json!({
            "id": "matrix-main",
            "rows": 2,
            "columns": 3,
            "pitch": {"x": 19.05, "y": 19.05},
            "origin": {"x": 0.0, "y": 0.0},
            "definitionId": "switch:base",
            "partIds": [],
            "boardId": "board-main",
            "cells": []
        }))
        .unwrap()
    }

    fn accepted(document: ProjectDoc, token: u64) -> AcceptedSnapshot {
        let revision = document.revision;
        AcceptedSnapshot {
            token: SnapshotToken(token),
            session_epoch: SessionEpoch(7),
            document: Arc::new(document),
            scene: Arc::new(boardstudio_core::model::SceneDelta {
                module_scenes: vec![],
                revision,
                transaction_id: "controller-placement-test".into(),
                changed_ids: vec![],
                transforms: vec![],
                matrix_scenes: vec![],
                contours: vec![],
                board_contours: vec![],
                board_readiness: vec![],
                board_outline_scenes: vec![],
                finding_markers: vec![],
                findings: vec![],
                readiness: boardstudio_core::model::Readiness {
                    layout: false,
                    outline: false,
                    pcb: false,
                    case_ready: false,
                },
            }),
        }
    }

    #[derive(Default)]
    struct HookRuntime {
        model: RefCell<ReadModel>,
        model_reads: Cell<u64>,
        outcomes: crate::operation_outcomes::OperationOutcomes,
        events: RefCell<Vec<SessionEvent>>,
        next_operation: Cell<u64>,
        settle_edit_on_submit: Cell<bool>,
        registered_before_submit: Cell<Option<bool>>,
    }

    impl PlacementRuntime for HookRuntime {
        fn model(&self) -> ReadModel {
            self.model_reads.set(self.model_reads.get() + 1);
            self.model.borrow().clone()
        }

        fn scope(&self) -> Option<Scope> {
            let model = self.model.borrow();
            let snapshot = model.accepted.as_ref()?;
            Some(Scope {
                session_epoch: snapshot.session_epoch,
                document_id: snapshot.document.id.clone(),
                board_id: model.active_board_id.clone(),
                instance_id: model.active_instance_id.clone(),
            })
        }

        fn operation(&self) -> OperationId {
            let next = self.next_operation.get() + 1;
            self.next_operation.set(next);
            OperationId(next)
        }

        fn observe_operation(
            &self,
            operation: OperationId,
        ) -> crate::operation_outcomes::OutcomeSlot {
            self.outcomes.observe(operation)
        }

        fn submit(&self, event: SessionEvent) {
            if let SessionEvent::Edit { operation_id, .. } = &event
                && self.settle_edit_on_submit.get()
            {
                self.registered_before_submit
                    .set(Some(self.settle(*operation_id, TerminalOutcome::Completed)));
            }
            self.events.borrow_mut().push(event);
        }

        fn begin_edit(
            &self,
            label: &str,
            feature: Option<String>,
            resolver: boardstudio_application::EditResolver,
        ) -> EditTicket {
            EditTicket::begin(&HookPort(self), label, feature, resolver)
        }
    }

    impl HookRuntime {
        /// Settle an operation the way the Session does: a completed edit lands at the
        /// accepted revision.
        fn settle(&self, operation: OperationId, outcome: TerminalOutcome) -> bool {
            let landing = (outcome == TerminalOutcome::Completed)
                .then(|| {
                    self.model.borrow().accepted.as_ref().map(|accepted| {
                        boardstudio_application::Landing {
                            revision: accepted.document.revision,
                            token: accepted.token,
                        }
                    })
                })
                .flatten();
            self.outcomes
                .settle_with_landing(operation, outcome, landing)
        }
    }

    /// The edit-ticket port over the fake: a submitted resolver runs against the model's
    /// accepted snapshot straight away, recording the edit it resolves to.
    struct HookPort<'a>(&'a HookRuntime);

    impl boardstudio_web_runtime::edit_ticket::EditTicketPort for HookPort<'_> {
        fn allocate_operation(&self) -> OperationId {
            self.0.operation()
        }

        fn observe(
            &self,
            operation: OperationId,
        ) -> (
            crate::operation_outcomes::OutcomeSlot,
            crate::operation_outcomes::LandingSlot,
        ) {
            self.0.outcomes.observe_with_landing(operation)
        }

        fn submit(&self, event: SessionEvent) {
            let SessionEvent::ResolveEdit {
                operation_id,
                resolver,
                ..
            } = event
            else {
                self.0.submit(event);
                return;
            };
            let accepted = self
                .0
                .model
                .borrow()
                .accepted
                .clone()
                .expect("a resolver runs against an accepted snapshot");
            match resolver.resolve(&accepted) {
                Resolution::Submit(mut command) => {
                    command.base_revision = accepted.document.revision;
                    command.transaction_id = format!("test-edit-{}", operation_id.0);
                    self.0.submit(SessionEvent::Edit {
                        operation_id,
                        command,
                    });
                }
                Resolution::Unchanged => {
                    self.0.settle(operation_id, TerminalOutcome::Completed);
                }
                Resolution::Retire(reason) => {
                    self.0
                        .settle(operation_id, TerminalOutcome::Rejected(reason));
                }
            }
        }
    }

    #[derive(Clone)]
    struct HookProbe {
        runtime: Rc<HookRuntime>,
        canvas_interaction: CanvasInteractionArbiter,
        mounted: Rc<Cell<bool>>,
        unmounted: Rc<Cell<bool>>,
        latest: Rc<RefCell<Option<PartPlacementMount>>>,
        assembly_3d: Rc<RefCell<Option<Signal<bool>>>>,
        view_mode: Rc<RefCell<Option<EventHandler<bool>>>>,
        workspace: Rc<RefCell<Option<Signal<&'static str>>>>,
        selected_context: Rc<RefCell<Option<Signal<Option<ScopedTreeContext>>>>>,
        selection_kind: Rc<RefCell<Option<Signal<objects::LayoutSelectionKind>>>>,
        guide: Rc<RefCell<Option<Signal<Option<SetupGuidePreferences>>>>>,
        version: Rc<Cell<u64>>,
        loader_reply: Rc<RefCell<Option<Result<PartDefinition, String>>>>,
        loader_waker: Rc<RefCell<Option<Waker>>>,
    }

    fn hook_host() -> Element {
        let probe = use_context::<HookProbe>();
        if probe.mounted.get() {
            rsx! { HookMounted {} }
        } else {
            rsx! { div {} }
        }
    }

    #[component]
    fn HookMounted() -> Element {
        let probe = use_context::<HookProbe>();
        use_drop({
            let unmounted = probe.unmounted.clone();
            move || unmounted.set(true)
        });
        let workspace = use_signal(|| "Layout");
        let guide = use_signal(|| {
            Some(SetupGuidePreferences {
                project_id: "project".into(),
                open: true,
                current_stage: SetupGuideStage::Wiring,
            })
        });
        let mut version = use_signal(|| 0u64);
        let current_version = probe.version.get();
        if *version.peek() != current_version {
            version.set(current_version);
        }
        *probe.workspace.borrow_mut() = Some(workspace);
        *probe.guide.borrow_mut() = Some(guide);
        let selected_context = use_signal(|| None);
        *probe.selected_context.borrow_mut() = Some(selected_context);
        let layout_selection_kind = use_signal(objects::LayoutSelectionKind::default);
        *probe.selection_kind.borrow_mut() = Some(layout_selection_kind);
        let anchor_scope = use_signal(|| None);
        let generation = use_signal(|| 4u64);
        let adapter = SelectionAdapter::new(selected_context, anchor_scope, generation);
        let loader_reply = probe.loader_reply.clone();
        let loader_waker = probe.loader_waker.clone();
        let mount = use_controller_placement(PartPlacementHost {
            runtime: probe.runtime.clone(),
            load_definition: Rc::new(move |_, _| {
                let reply = loader_reply.clone();
                let waker = loader_waker.clone();
                Box::pin(std::future::poll_fn(move |context| {
                    match reply.borrow_mut().take() {
                        Some(value) => std::task::Poll::Ready(value),
                        None => {
                            *waker.borrow_mut() = Some(context.waker().clone());
                            std::task::Poll::Pending
                        }
                    }
                }))
            }),
            workspace,
            generation,
            version,
            adapter,
            layout_selection_kind,
            guide_preferences: guide,
            parts_query: use_signal(String::new),
            parts_selection: use_signal(|| None),
            snap_settings: use_signal(LayoutSnapSettings::default),
            layout_target: use_signal(|| None),
            canvas_center: Vec2::default(),
            objects_open: use_signal(|| false),
            inspect_open: use_signal(|| false),
            canvas_interaction: probe.canvas_interaction.clone(),
        });
        *probe.latest.borrow_mut() = Some(mount.clone());
        let assembly_3d = use_signal(|| false);
        let assembly_3d_read = assembly_3d;
        let mut assembly_3d_write = assembly_3d;
        *probe.assembly_3d.borrow_mut() = Some(assembly_3d);
        let matrix_placement = objects::MatrixPlacementMount {
            cancel_owner: None,
            placement: None,
            busy: false,
            error: None,
            on_place: EventHandler::new(|_| {}),
            on_move: EventHandler::new(|_| {}),
            on_commit: EventHandler::new(|_| {}),
            on_cancel: EventHandler::new(|_| {}),
        };
        *probe.view_mode.borrow_mut() = Some(crate::presentation::layout_view_mode_handler(
            || true,
            assembly_3d_read,
            move |value| assembly_3d_write.set(value),
            crate::presentation::LayoutPlacementCancellation {
                parts: mount,
                matrices: matrix_placement,
                interactions: probe.canvas_interaction.clone(),
            },
            || {},
            || {},
        ));
        rsx! { div {} }
    }

    fn hook_mounted() -> (HookProbe, VirtualDom) {
        let document = fixture();
        let snapshot = accepted(document.clone(), 11);
        let runtime = Rc::new(HookRuntime::default());
        *runtime.model.borrow_mut() = ReadModel {
            lifecycle: Lifecycle::Ready,
            durability: Durability::Saved {
                revision: document.revision,
            },
            accepted: Some(snapshot),
            active_board_id: "board-main".into(),
            ..ReadModel::default()
        };
        let probe = HookProbe {
            runtime,
            canvas_interaction: CanvasInteractionArbiter::default(),
            mounted: Rc::new(Cell::new(true)),
            unmounted: Rc::new(Cell::new(false)),
            latest: Rc::default(),
            assembly_3d: Rc::default(),
            view_mode: Rc::default(),
            workspace: Rc::default(),
            selected_context: Rc::default(),
            selection_kind: Rc::default(),
            guide: Rc::default(),
            version: Rc::new(Cell::new(0)),
            loader_reply: Rc::new(RefCell::new(None)),
            loader_waker: Rc::default(),
        };
        let mut dom = VirtualDom::new(hook_host);
        dom.provide_root_context(probe.clone());
        dom.rebuild_to_vec();
        flush_hook(&mut dom);
        (probe, dom)
    }

    fn flush_hook(dom: &mut VirtualDom) {
        dom.mark_all_dirty();
        for _ in 0..5 {
            dom.render_immediate_to_vec();
            let mut work = std::pin::pin!(dom.wait_for_work());
            let _ = work.as_mut().poll(&mut Context::from_waker(Waker::noop()));
        }
    }

    fn workspace(probe: &HookProbe) -> &'static str {
        probe.workspace.borrow().as_ref().unwrap()()
    }

    async fn let_hook_tasks_run() {
        gloo_timers::future::TimeoutFuture::new(20).await;
    }

    fn resolve_loader(probe: &HookProbe, value: Result<PartDefinition, String>) {
        *probe.loader_reply.borrow_mut() = Some(value);
        if let Some(waker) = probe.loader_waker.borrow_mut().take() {
            waker.wake();
        }
    }

    async fn start_hook_placement(probe: &HookProbe, dom: &mut VirtualDom) -> PartPlacementMount {
        resolve_loader(probe, Ok(controller_definition("catalog:controller")));
        probe
            .latest
            .borrow()
            .as_ref()
            .unwrap()
            .on_choose_controller
            .call(());
        flush_hook(dom);
        probe
            .latest
            .borrow()
            .as_ref()
            .unwrap()
            .on_place_controller
            .call("catalog:controller".into());
        let_hook_tasks_run().await;
        flush_hook(dom);
        let mount = probe.latest.borrow().as_ref().unwrap().clone();
        assert!(mount.projection.is_some());
        mount
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    async fn general_component_placement_starts_without_the_setup_guide_and_uses_kind_reference() {
        let (probe, mut dom) = hook_mounted();
        let mut guide = *probe.guide.borrow().as_ref().unwrap();
        guide.set(None);
        flush_hook(&mut dom);

        probe
            .latest
            .borrow()
            .as_ref()
            .unwrap()
            .on_place_component
            .call(ComponentPlacementAction::AddObject {
                definition_id: "catalog:reset-switch".into(),
                kind: PartKind::Passive,
            });
        assert!(
            probe
                .canvas_interaction
                .is_owner(CanvasInteractionOwner::PartPlacement)
        );
        resolve_loader(&probe, Ok(passive_definition("catalog:reset-switch")));
        let_hook_tasks_run().await;
        flush_hook(&mut dom);

        let mount = probe.latest.borrow().as_ref().unwrap().clone();
        let placement = mount
            .projection
            .expect("generic component enters canvas placement");
        assert_eq!(placement.pending.part.reference, "J1");
        assert_eq!(placement.pending.definition.kind, PartKind::Passive);
        assert_eq!(workspace(&probe), "Layout");
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    async fn accepted_general_placement_selects_the_component_and_part_tool_context() {
        let (probe, mut dom) = hook_mounted();
        let mut guide = *probe.guide.borrow().as_ref().unwrap();
        guide.set(None);
        flush_hook(&mut dom);
        probe
            .latest
            .borrow()
            .as_ref()
            .unwrap()
            .on_place_component
            .call(ComponentPlacementAction::AddObject {
                definition_id: "catalog:reset-switch".into(),
                kind: PartKind::Passive,
            });
        resolve_loader(&probe, Ok(passive_definition("catalog:reset-switch")));
        let_hook_tasks_run().await;
        flush_hook(&mut dom);
        let active = probe.latest.borrow().as_ref().unwrap().clone();
        active.on_commit.call(Vec2 { x: 5.0, y: -2.0 });
        let (operation_id, edit) = submitted_edit(&probe);
        let document = replacement(edit);
        let part_id = document.parts.last().unwrap().id.clone();
        let original = probe
            .runtime
            .model
            .borrow()
            .accepted
            .as_ref()
            .unwrap()
            .clone();
        let mut scene = (*original.scene).clone();
        scene.revision = document.revision + 1;
        *probe.runtime.model.borrow_mut() = ReadModel {
            lifecycle: Lifecycle::Ready,
            durability: Durability::Saved {
                revision: document.revision + 1,
            },
            accepted: Some(AcceptedSnapshot {
                token: SnapshotToken(12),
                session_epoch: original.session_epoch,
                document: Arc::new(ProjectDoc {
                    revision: document.revision + 1,
                    ..document
                }),
                scene: Arc::new(scene),
            }),
            active_board_id: "board-main".into(),
            ..ReadModel::default()
        };
        assert!(
            probe
                .runtime
                .settle(operation_id, TerminalOutcome::Completed)
        );
        let_hook_tasks_run().await;
        flush_hook(&mut dom);

        assert_eq!(workspace(&probe), "Layout");
        assert_eq!(
            probe.selection_kind.borrow().as_ref().unwrap()(),
            objects::LayoutSelectionKind::Part
        );
        let selected = probe.selected_context.borrow().as_ref().unwrap()();
        assert!(matches!(
            selected,
            Some(ScopedTreeContext {
                context: TreeContext::Component { part_id: Some(id), .. },
                ..
            }) if id == part_id
        ));
        assert!(probe.runtime.events.borrow().iter().any(|event| matches!(
            event,
            SessionEvent::SelectParts { part_ids, .. }
                if part_ids.len() == 1 && part_ids.first() == Some(&part_id)
        )));
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    async fn parts_inspector_action_applies_selected_key_through_production_edit_handler() {
        let (probe, mut dom) = hook_mounted();
        let mut document = (*probe
            .runtime
            .model
            .borrow()
            .accepted
            .as_ref()
            .unwrap()
            .document)
            .clone();
        document.matrices.push(matrix_fixture());
        {
            let mut model = probe.runtime.model.borrow_mut();
            let accepted = model.accepted.as_mut().unwrap();
            accepted.document = Arc::new(document);
            Arc::make_mut(&mut accepted.scene).matrix_scenes.push(
                boardstudio_core::model::MatrixScene {
                    matrix_id: "matrix-main".into(),
                    cells: Vec::new(),
                    columns: Vec::new(),
                },
            );
        }
        let scope = probe.runtime.scope().unwrap();
        let selected = ScopedTreeContext {
            scope: scope.clone(),
            context: TreeContext::Key {
                matrix_id: "matrix-main".into(),
                row: 1,
                column: 2,
            },
        };
        assert!(super::super::selection::context_is_current(
            &probe.runtime.model(),
            &scope,
            &selected.context,
        ));
        let mut selected_context = *probe.selected_context.borrow().as_ref().unwrap();
        selected_context.set(Some(selected.clone()));
        let mut workspace_signal = *probe.workspace.borrow().as_ref().unwrap();
        workspace_signal.set("Parts");
        probe.runtime.settle_edit_on_submit.set(true);
        flush_hook(&mut dom);

        probe
            .latest
            .borrow()
            .as_ref()
            .unwrap()
            .on_place_component
            .call(ComponentPlacementAction::PartsInspector {
                definition_id: "imported:reset-switch".into(),
                kind: PartKind::Passive,
            });
        resolve_loader(&probe, Ok(passive_definition("imported:reset-switch")));
        let_hook_tasks_run().await;
        flush_hook(&mut dom);

        let edit = probe
            .runtime
            .events
            .borrow()
            .iter()
            .find_map(|event| match event {
                SessionEvent::Edit { command, .. } => Some(command.clone()),
                _ => None,
            })
            .expect("selected-key action submits a real edit");
        let EditOperation::SetMatrix {
            matrix,
            definitions,
        } = &edit.operation
        else {
            panic!("selected-key action commits SetMatrix");
        };
        assert_eq!(
            matrix.cells[0].assemblies[0].definition_id,
            "imported:reset-switch"
        );
        assert_eq!(definitions.as_ref().unwrap()[0].id, "imported:reset-switch");
        assert_eq!(edit.base_revision, 0);
        assert_eq!(workspace(&probe), "Parts");
        assert_eq!(selected_context(), Some(selected));
        assert!(
            !probe
                .canvas_interaction
                .is_owner(CanvasInteractionOwner::PartPlacement)
        );
        let_hook_tasks_run().await;
        flush_hook(&mut dom);
        let mount = probe.latest.borrow().as_ref().unwrap().clone();
        assert!(!mount.busy);
        assert!(mount.error.is_none());
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    async fn delayed_selected_key_load_error_is_ignored_after_accepted_snapshot_refresh() {
        let (probe, mut dom) = hook_mounted();
        let mut document = (*probe
            .runtime
            .model
            .borrow()
            .accepted
            .as_ref()
            .unwrap()
            .document)
            .clone();
        document.matrices.push(matrix_fixture());
        {
            let mut model = probe.runtime.model.borrow_mut();
            let accepted = model.accepted.as_mut().unwrap();
            accepted.document = Arc::new(document);
            Arc::make_mut(&mut accepted.scene).matrix_scenes.push(
                boardstudio_core::model::MatrixScene {
                    matrix_id: "matrix-main".into(),
                    cells: Vec::new(),
                    columns: Vec::new(),
                },
            );
        }
        let selected = ScopedTreeContext {
            scope: probe.runtime.scope().unwrap(),
            context: TreeContext::Key {
                matrix_id: "matrix-main".into(),
                row: 0,
                column: 0,
            },
        };
        let mut selected_context = *probe.selected_context.borrow().as_ref().unwrap();
        selected_context.set(Some(selected));
        let mut workspace_signal = *probe.workspace.borrow().as_ref().unwrap();
        workspace_signal.set("Parts");
        flush_hook(&mut dom);

        probe
            .latest
            .borrow()
            .as_ref()
            .unwrap()
            .on_place_component
            .call(ComponentPlacementAction::PartsInspector {
                definition_id: "imported:delayed-error".into(),
                kind: PartKind::Passive,
            });
        let_hook_tasks_run().await;
        assert!(probe.loader_waker.borrow().is_some());

        let mut refreshed = (*probe
            .runtime
            .model
            .borrow()
            .accepted
            .as_ref()
            .unwrap()
            .document)
            .clone();
        refreshed.revision += 1;
        {
            let mut model = probe.runtime.model.borrow_mut();
            model.accepted = Some(accepted(refreshed.clone(), 12));
            model.durability = Durability::Saved {
                revision: refreshed.revision,
            };
        }

        resolve_loader(&probe, Err("stale catalogue failure".into()));
        let_hook_tasks_run().await;
        flush_hook(&mut dom);
        assert_eq!(probe.latest.borrow().as_ref().unwrap().error, None);
        assert!(probe.runtime.events.borrow().is_empty());
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    async fn retained_component_action_cannot_retarget_a_new_owner_or_wrong_workspace() {
        let (probe, mut dom) = hook_mounted();
        let retained = probe.latest.borrow().as_ref().unwrap().on_place_component;

        retained.call(ComponentPlacementAction::PartsInspector {
            definition_id: "imported:wrong-workspace".into(),
            kind: PartKind::Passive,
        });
        flush_hook(&mut dom);
        let_hook_tasks_run().await;
        assert!(probe.loader_waker.borrow().is_none());

        let mut refreshed = (*probe
            .runtime
            .model
            .borrow()
            .accepted
            .as_ref()
            .unwrap()
            .document)
            .clone();
        refreshed.revision += 1;
        {
            let mut model = probe.runtime.model.borrow_mut();
            model.accepted = Some(accepted(refreshed.clone(), 12));
            model.durability = Durability::Saved {
                revision: refreshed.revision,
            };
        }
        retained.call(ComponentPlacementAction::AddObject {
            definition_id: "imported:stale-owner".into(),
            kind: PartKind::Passive,
        });
        flush_hook(&mut dom);
        let_hook_tasks_run().await;
        assert!(probe.loader_waker.borrow().is_none());
        assert!(!probe.latest.borrow().as_ref().unwrap().busy);
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    async fn retained_object_action_cannot_start_for_a_refreshed_snapshot() {
        let (probe, mut dom) = hook_mounted();
        let retained = probe.latest.borrow().as_ref().unwrap().on_place_component;
        let mut refreshed = (*probe
            .runtime
            .model
            .borrow()
            .accepted
            .as_ref()
            .unwrap()
            .document)
            .clone();
        refreshed.revision += 1;
        {
            let mut model = probe.runtime.model.borrow_mut();
            model.accepted = Some(accepted(refreshed.clone(), 12));
            model.durability = Durability::Saved {
                revision: refreshed.revision,
            };
        }

        retained.call(ComponentPlacementAction::AddObject {
            definition_id: "imported:stale-owner".into(),
            kind: PartKind::Passive,
        });
        let_hook_tasks_run().await;
        flush_hook(&mut dom);
        assert!(probe.loader_waker.borrow().is_none());
        assert!(!probe.latest.borrow().as_ref().unwrap().busy);
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    async fn parts_inspector_action_does_not_apply_to_a_key_selected_after_loading_started() {
        let (probe, mut dom) = hook_mounted();
        let mut document = (*probe
            .runtime
            .model
            .borrow()
            .accepted
            .as_ref()
            .unwrap()
            .document)
            .clone();
        document.matrices.push(matrix_fixture());
        {
            let mut model = probe.runtime.model.borrow_mut();
            let accepted = model.accepted.as_mut().unwrap();
            accepted.document = Arc::new(document);
            Arc::make_mut(&mut accepted.scene).matrix_scenes.push(
                boardstudio_core::model::MatrixScene {
                    matrix_id: "matrix-main".into(),
                    cells: Vec::new(),
                    columns: Vec::new(),
                },
            );
        }
        let scope = probe.runtime.scope().unwrap();
        let selected = ScopedTreeContext {
            scope: scope.clone(),
            context: TreeContext::Key {
                matrix_id: "matrix-main".into(),
                row: 1,
                column: 2,
            },
        };
        let mut selected_context = *probe.selected_context.borrow().as_ref().unwrap();
        selected_context.set(Some(selected));
        let mut workspace_signal = *probe.workspace.borrow().as_ref().unwrap();
        workspace_signal.set("Parts");
        flush_hook(&mut dom);

        probe
            .latest
            .borrow()
            .as_ref()
            .unwrap()
            .on_place_component
            .call(ComponentPlacementAction::PartsInspector {
                definition_id: "imported:reset-switch".into(),
                kind: PartKind::Passive,
            });
        let mut selected_context = *probe.selected_context.borrow().as_ref().unwrap();
        selected_context.set(None);
        flush_hook(&mut dom);
        resolve_loader(&probe, Ok(passive_definition("imported:reset-switch")));
        let_hook_tasks_run().await;
        flush_hook(&mut dom);

        assert!(
            probe
                .runtime
                .events
                .borrow()
                .iter()
                .all(|event| { !matches!(event, SessionEvent::Edit { .. }) })
        );
        assert_eq!(workspace(&probe), "Parts");
        assert!(
            !probe
                .canvas_interaction
                .is_owner(CanvasInteractionOwner::PartPlacement)
        );
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    async fn selected_key_edit_rejection_is_reported_to_the_current_parts_owner() {
        let (probe, mut dom) = hook_mounted();
        let mut document = (*probe
            .runtime
            .model
            .borrow()
            .accepted
            .as_ref()
            .unwrap()
            .document)
            .clone();
        document.matrices.push(matrix_fixture());
        {
            let mut model = probe.runtime.model.borrow_mut();
            let accepted = model.accepted.as_mut().unwrap();
            accepted.document = Arc::new(document);
            Arc::make_mut(&mut accepted.scene).matrix_scenes.push(
                boardstudio_core::model::MatrixScene {
                    matrix_id: "matrix-main".into(),
                    cells: Vec::new(),
                    columns: Vec::new(),
                },
            );
        }
        let selected = ScopedTreeContext {
            scope: probe.runtime.scope().unwrap(),
            context: TreeContext::Key {
                matrix_id: "matrix-main".into(),
                row: 1,
                column: 2,
            },
        };
        let mut selected_context = *probe.selected_context.borrow().as_ref().unwrap();
        selected_context.set(Some(selected));
        let mut workspace_signal = *probe.workspace.borrow().as_ref().unwrap();
        workspace_signal.set("Parts");
        flush_hook(&mut dom);

        probe
            .latest
            .borrow()
            .as_ref()
            .unwrap()
            .on_place_component
            .call(ComponentPlacementAction::PartsInspector {
                definition_id: "imported:reset-switch".into(),
                kind: PartKind::Passive,
            });
        resolve_loader(&probe, Ok(passive_definition("imported:reset-switch")));
        let_hook_tasks_run().await;
        flush_hook(&mut dom);
        let operation_id = probe
            .runtime
            .events
            .borrow()
            .iter()
            .find_map(|event| match event {
                SessionEvent::Edit { operation_id, .. } => Some(*operation_id),
                _ => None,
            })
            .expect("selected-key action observes its production edit");
        assert!(probe.runtime.settle(
            operation_id,
            TerminalOutcome::Rejected("The matrix input is no longer valid.".into()),
        ));
        let_hook_tasks_run().await;
        flush_hook(&mut dom);

        let mount = probe.latest.borrow().as_ref().unwrap().clone();
        assert!(!mount.busy);
        assert!(
            mount
                .error
                .as_deref()
                .is_some_and(|message| message.contains("The matrix input is no longer valid.")),
            "the standard failure wording carries the reason: {:?}",
            mount.error
        );
        assert_eq!(workspace(&probe), "Parts");
    }

    fn submitted_edit(probe: &HookProbe) -> (OperationId, EditOperation) {
        probe
            .runtime
            .events
            .borrow()
            .iter()
            .find_map(|event| match event {
                SessionEvent::Edit {
                    operation_id,
                    command,
                } => Some((*operation_id, command.operation.clone())),
                _ => None,
            })
            .expect("placement submits an Edit")
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    async fn production_hook_owns_canvas_during_definition_preparation_and_escape_restores_guide() {
        let (probe, mut dom) = hook_mounted();
        let mount = probe.latest.borrow().as_ref().unwrap().clone();
        mount.on_choose_controller.call(());
        flush_hook(&mut dom);
        assert_eq!(workspace(&probe), "Parts");
        probe
            .latest
            .borrow()
            .as_ref()
            .unwrap()
            .on_place_controller
            .call("catalog:controller".into());
        let_hook_tasks_run().await;
        flush_hook(&mut dom);
        let preparing = probe.latest.borrow().as_ref().unwrap().clone();
        assert!(preparing.busy);
        assert!(preparing.owns_canvas());
        assert!(
            probe
                .canvas_interaction
                .is_owner(CanvasInteractionOwner::PartPlacement)
        );
        assert!(
            !probe
                .canvas_interaction
                .try_acquire(CanvasInteractionOwner::MirroredPair)
        );
        assert_eq!(
            probe.canvas_interaction.current(),
            Some(CanvasInteractionOwner::PartPlacement)
        );
        assert!(preparing.projection.is_none());
        assert!(
            !probe
                .runtime
                .events
                .borrow()
                .iter()
                .any(|event| matches!(event, SessionEvent::Edit { .. }))
        );

        resolve_loader(&probe, Ok(controller_definition("catalog:controller")));
        let_hook_tasks_run().await;
        flush_hook(&mut dom);
        let active = probe.latest.borrow().as_ref().unwrap().clone();
        assert!(
            active.projection.is_some(),
            "busy={} error={:?} workspace={}",
            active.busy,
            active.error,
            workspace(&probe)
        );
        assert!(active.owns_canvas());
        active.on_cancel.call(());
        flush_hook(&mut dom);
        assert_eq!(workspace(&probe), "PCB");
        assert!(probe.latest.borrow().as_ref().unwrap().projection.is_none());
        assert_eq!(probe.canvas_interaction.current(), None);
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    async fn layout_3d_transition_cancels_suspended_component_preparation_without_ghost() {
        let (probe, mut dom) = hook_mounted();
        probe
            .latest
            .borrow()
            .as_ref()
            .unwrap()
            .on_place_component
            .call(ComponentPlacementAction::AddObject {
                definition_id: "catalog:reset-switch".into(),
                kind: PartKind::Passive,
            });
        let_hook_tasks_run().await;
        flush_hook(&mut dom);

        let preparing = probe.latest.borrow().as_ref().unwrap().clone();
        assert!(preparing.busy);
        assert!(preparing.projection.is_none());
        assert!(probe.loader_waker.borrow().is_some());
        assert_eq!(
            probe.canvas_interaction.current(),
            Some(CanvasInteractionOwner::PartPlacement)
        );

        probe.view_mode.borrow().as_ref().unwrap().call(true);
        flush_hook(&mut dom);
        assert!(probe.assembly_3d.borrow().as_ref().unwrap()());
        assert!(!probe.latest.borrow().as_ref().unwrap().busy);
        assert_eq!(probe.canvas_interaction.current(), None);
        let cancellation_events = probe.runtime.events.borrow().len();
        assert_eq!(cancellation_events, 1);
        assert!(matches!(
            probe.runtime.events.borrow().first(),
            Some(SessionEvent::SelectParts { part_ids, range_part_ids, .. })
                if part_ids.is_empty() && range_part_ids.is_empty()
        ));

        resolve_loader(&probe, Ok(passive_definition("catalog:reset-switch")));
        let_hook_tasks_run().await;
        flush_hook(&mut dom);
        let settled = probe.latest.borrow().as_ref().unwrap().clone();
        assert!(!settled.busy);
        assert!(settled.projection.is_none());
        assert_eq!(probe.runtime.events.borrow().len(), cancellation_events);
        assert!(
            !probe
                .runtime
                .events
                .borrow()
                .iter()
                .any(|event| matches!(event, SessionEvent::Edit { .. }))
        );
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    async fn mounted_controller_start_is_rejected_while_mirrored_pair_owns_the_canvas() {
        let (probe, mut dom) = hook_mounted();
        probe
            .canvas_interaction
            .try_acquire(CanvasInteractionOwner::MirroredPair);
        let mut workspace = *probe.workspace.borrow().as_ref().unwrap();
        workspace.set("Parts");
        resolve_loader(&probe, Ok(controller_definition("catalog:controller")));
        probe
            .latest
            .borrow()
            .as_ref()
            .unwrap()
            .on_place_controller
            .call("catalog:controller".into());
        let_hook_tasks_run().await;
        flush_hook(&mut dom);
        let mount = probe.latest.borrow().as_ref().unwrap().clone();
        assert!(!mount.busy);
        assert!(mount.projection.is_none());
        assert!(mount.error.is_none());
        assert_eq!(
            probe.canvas_interaction.current(),
            Some(CanvasInteractionOwner::MirroredPair)
        );
        assert!(
            !probe
                .runtime
                .events
                .borrow()
                .iter()
                .any(|event| matches!(event, SessionEvent::Edit { .. }))
        );
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    async fn production_hook_registers_before_submit_and_retains_outcome_after_unmount() {
        let (probe, mut dom) = hook_mounted();
        resolve_loader(&probe, Ok(controller_definition("catalog:controller")));
        let mount = probe.latest.borrow().as_ref().unwrap().clone();
        mount.on_choose_controller.call(());
        flush_hook(&mut dom);
        probe
            .latest
            .borrow()
            .as_ref()
            .unwrap()
            .on_place_controller
            .call("catalog:controller".into());
        let_hook_tasks_run().await;
        flush_hook(&mut dom);
        let active = probe.latest.borrow().as_ref().unwrap().clone();
        assert!(active.projection.is_some());
        assert!(
            probe
                .canvas_interaction
                .is_owner(CanvasInteractionOwner::PartPlacement)
        );
        assert!(
            !probe
                .canvas_interaction
                .try_acquire(CanvasInteractionOwner::MirroredPair)
        );
        active.on_commit.call(Vec2 { x: 4.0, y: -3.0 });
        let operation = probe
            .runtime
            .events
            .borrow()
            .iter()
            .find_map(|event| match event {
                SessionEvent::Edit { operation_id, .. } => Some(*operation_id),
                _ => None,
            })
            .expect("placement submits an Edit");
        probe.latest.borrow_mut().take();
        drop(dom);
        assert!(
            probe
                .runtime
                .outcomes
                .settle(operation, TerminalOutcome::Completed)
        );
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    async fn production_hook_registers_the_observer_before_synchronous_submit_settlement() {
        let (probe, mut dom) = hook_mounted();
        probe.runtime.settle_edit_on_submit.set(true);
        resolve_loader(&probe, Ok(controller_definition("catalog:controller")));
        probe
            .latest
            .borrow()
            .as_ref()
            .unwrap()
            .on_choose_controller
            .call(());
        flush_hook(&mut dom);
        probe
            .latest
            .borrow()
            .as_ref()
            .unwrap()
            .on_place_controller
            .call("catalog:controller".into());
        let_hook_tasks_run().await;
        flush_hook(&mut dom);
        probe
            .latest
            .borrow()
            .as_ref()
            .unwrap()
            .on_commit
            .call(Vec2 { x: 2.0, y: 1.0 });
        assert_eq!(probe.runtime.registered_before_submit.get(), Some(true));
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    async fn production_hook_returns_to_wiring_only_after_matching_ready_saved_commit() {
        let (probe, mut dom) = hook_mounted();
        let active = start_hook_placement(&probe, &mut dom).await;
        assert!(
            probe
                .canvas_interaction
                .is_owner(CanvasInteractionOwner::PartPlacement)
        );
        assert!(
            !probe
                .canvas_interaction
                .try_acquire(CanvasInteractionOwner::MirroredPair)
        );
        active.on_commit.call(Vec2 { x: 5.0, y: -2.0 });
        let (operation_id, edit) = submitted_edit(&probe);
        let mut document = replacement(edit);
        document.revision += 1;
        let scene = probe
            .runtime
            .model
            .borrow()
            .accepted
            .as_ref()
            .unwrap()
            .scene
            .clone();
        *probe.runtime.model.borrow_mut() = ReadModel {
            lifecycle: Lifecycle::Ready,
            durability: Durability::Saved {
                revision: document.revision,
            },
            accepted: Some(AcceptedSnapshot {
                token: SnapshotToken(12),
                session_epoch: SessionEpoch(7),
                scene,
                document: Arc::new(document),
            }),
            active_board_id: "board-main".into(),
            ..ReadModel::default()
        };
        assert!(
            probe
                .runtime
                .settle(operation_id, TerminalOutcome::Completed)
        );
        let_hook_tasks_run().await;
        flush_hook(&mut dom);
        assert_eq!(workspace(&probe), "PCB");
        assert!(probe.latest.borrow().as_ref().unwrap().projection.is_none());
        assert_eq!(probe.canvas_interaction.current(), None);
        assert!(
            probe
                .runtime
                .events
                .borrow()
                .iter()
                .any(|event| matches!(event, SessionEvent::SelectParts { .. }))
        );
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    async fn production_hook_routes_persistence_failure_back_but_terminal_cancel_does_not_redirect()
    {
        for (outcome, expected_workspace, expected_error) in [
            (
                TerminalOutcome::PersistenceFailed("disk full".into()),
                "Parts",
                Some("disk full"),
            ),
            (TerminalOutcome::Cancelled, "Layout", None),
        ] {
            let (probe, mut dom) = hook_mounted();
            let active = start_hook_placement(&probe, &mut dom).await;
            active.on_commit.call(Vec2 { x: 5.0, y: -2.0 });
            let (operation_id, _) = submitted_edit(&probe);
            assert!(probe.runtime.settle(operation_id, outcome));
            let_hook_tasks_run().await;
            flush_hook(&mut dom);
            assert_eq!(workspace(&probe), expected_workspace);
            let error = probe.latest.borrow().as_ref().unwrap().error.clone();
            assert_eq!(
                error.is_some(),
                expected_error.is_some(),
                "failure message presence: {error:?}"
            );
            if let (Some(error), Some(expected)) = (error, expected_error) {
                assert!(error.contains(expected), "{error} should carry {expected}");
            }
        }
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    async fn production_hook_never_redirects_a_stale_board_or_project_owner() {
        for stale_identity in ["board", "project"] {
            let (probe, mut dom) = hook_mounted();
            let active = start_hook_placement(&probe, &mut dom).await;
            active.on_commit.call(Vec2 { x: 5.0, y: -2.0 });
            let (operation_id, edit) = submitted_edit(&probe);
            let select_count_before = probe
                .runtime
                .events
                .borrow()
                .iter()
                .filter(|event| matches!(event, SessionEvent::SelectParts { .. }))
                .count();
            {
                let mut model = probe.runtime.model.borrow_mut();
                if stale_identity == "board" {
                    let original = model.accepted.as_ref().unwrap().clone();
                    let mut document = replacement(edit.clone());
                    document.revision = original.document.revision + 1;
                    let mut scene = (*original.scene).clone();
                    scene.revision = document.revision;
                    model.lifecycle = Lifecycle::Ready;
                    model.durability = Durability::Saved {
                        revision: document.revision,
                    };
                    model.accepted = Some(AcceptedSnapshot {
                        token: SnapshotToken(original.token.0 + 1),
                        session_epoch: original.session_epoch,
                        document: Arc::new(document),
                        scene: Arc::new(scene),
                    });
                    model.active_board_id = "board-other".into();
                } else {
                    let accepted = model.accepted.as_mut().unwrap();
                    let mut document = (*accepted.document).clone();
                    document.id = "replacement-project".into();
                    accepted.document = Arc::new(document);
                    accepted.token = SnapshotToken(12);
                }
            }
            assert!(
                probe
                    .runtime
                    .outcomes
                    .settle(operation_id, TerminalOutcome::Completed)
            );
            let_hook_tasks_run().await;
            flush_hook(&mut dom);
            assert_eq!(
                workspace(&probe),
                "Layout",
                "stale {stale_identity} route redirected"
            );
            assert_eq!(
                probe
                    .runtime
                    .events
                    .borrow()
                    .iter()
                    .filter(|event| matches!(event, SessionEvent::SelectParts { .. }))
                    .count(),
                select_count_before,
                "stale {stale_identity} completion selected a new owner"
            );
        }
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    async fn production_hook_does_not_read_signals_after_unmount_while_waiting_for_saved_state() {
        let (probe, mut dom) = hook_mounted();
        let active = start_hook_placement(&probe, &mut dom).await;
        active.on_commit.call(Vec2 { x: 5.0, y: -2.0 });
        let (operation_id, _) = submitted_edit(&probe);
        let accepted = probe
            .runtime
            .model
            .borrow()
            .accepted
            .as_ref()
            .unwrap()
            .clone();
        *probe.runtime.model.borrow_mut() = ReadModel {
            lifecycle: Lifecycle::Saving,
            durability: Durability::Saving { revision: 0 },
            accepted: Some(accepted),
            active_board_id: "board-main".into(),
            ..ReadModel::default()
        };
        assert!(
            probe
                .runtime
                .settle(operation_id, TerminalOutcome::Completed)
        );
        // Let the production observer see Completed and enter its Ready/Saved wait.
        let_hook_tasks_run().await;
        probe.mounted.set(false);
        flush_hook(&mut dom);
        assert!(
            probe.unmounted.get(),
            "mounted hook component was not dropped"
        );
        let model_reads_after_unmount = probe.runtime.model_reads.get();
        drop(dom);
        // Resume the observer after the component-owned Signals have been dropped.
        let_hook_tasks_run().await;
        assert_eq!(
            probe.runtime.model_reads.get(),
            model_reads_after_unmount,
            "completed placement observer read runtime state after unmount"
        );
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    async fn production_hook_suppresses_actionable_failure_after_board_or_project_switch() {
        for (stale_identity, outcome) in [
            ("board", TerminalOutcome::Rejected("stale board".into())),
            (
                "project",
                TerminalOutcome::PersistenceFailed("stale project".into()),
            ),
        ] {
            let (probe, mut dom) = hook_mounted();
            let active = start_hook_placement(&probe, &mut dom).await;
            active.on_commit.call(Vec2 { x: 5.0, y: -2.0 });
            let (operation_id, _) = submitted_edit(&probe);
            let select_count_before = probe
                .runtime
                .events
                .borrow()
                .iter()
                .filter(|event| matches!(event, SessionEvent::SelectParts { .. }))
                .count();
            {
                let mut model = probe.runtime.model.borrow_mut();
                if stale_identity == "board" {
                    model.active_board_id = "board-other".into();
                } else {
                    let accepted = model.accepted.as_mut().unwrap();
                    let mut document = (*accepted.document).clone();
                    document.id = "replacement-project".into();
                    accepted.document = Arc::new(document);
                    accepted.token = SnapshotToken(12);
                }
            }
            assert!(probe.runtime.settle(operation_id, outcome));
            let_hook_tasks_run().await;
            flush_hook(&mut dom);
            assert_eq!(workspace(&probe), "Layout");
            assert_eq!(probe.latest.borrow().as_ref().unwrap().error, None);
            assert!(
                probe
                    .runtime
                    .events
                    .borrow()
                    .iter()
                    .filter(|event| matches!(event, SessionEvent::SelectParts { .. }))
                    .count()
                    == select_count_before,
                "stale {stale_identity} failure submitted a new selection"
            );
        }
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    async fn production_hook_retires_stale_preparation_and_accepts_a_fresh_place_request() {
        let (probe, mut dom) = hook_mounted();
        probe
            .latest
            .borrow()
            .as_ref()
            .unwrap()
            .on_choose_controller
            .call(());
        flush_hook(&mut dom);
        probe
            .latest
            .borrow()
            .as_ref()
            .unwrap()
            .on_place_controller
            .call("catalog:controller".into());
        let_hook_tasks_run().await;
        flush_hook(&mut dom);
        assert!(probe.latest.borrow().as_ref().unwrap().owns_canvas());

        {
            let mut model = probe.runtime.model.borrow_mut();
            let accepted = model.accepted.as_mut().unwrap();
            let mut next = (*accepted.document).clone();
            next.revision += 1;
            accepted.document = Arc::new(next.clone());
            accepted.token = SnapshotToken(12);
            model.durability = Durability::Saved {
                revision: next.revision,
            };
        }
        probe.version.set(probe.version.get() + 1);
        flush_hook(&mut dom);
        assert!(!probe.latest.borrow().as_ref().unwrap().owns_canvas());

        resolve_loader(&probe, Ok(controller_definition("catalog:controller")));
        let_hook_tasks_run().await;
        flush_hook(&mut dom);
        assert!(probe.latest.borrow().as_ref().unwrap().projection.is_none());

        probe
            .latest
            .borrow()
            .as_ref()
            .unwrap()
            .on_place_controller
            .call("catalog:controller".into());
        let_hook_tasks_run().await;
        resolve_loader(&probe, Ok(controller_definition("catalog:controller")));
        let_hook_tasks_run().await;
        flush_hook(&mut dom);
        assert!(probe.latest.borrow().as_ref().unwrap().projection.is_some());
    }

    fn part(definition_id: &str, id: &str, reference: &str) -> Part {
        Part {
            keycap: None,
            outline: None,
            id: id.into(),
            definition_id: definition_id.into(),
            reference: reference.into(),
            pose: Pose2 {
                at: Vec2 { x: 12.0, y: -8.0 },
                rotation: 0.0,
            },
            side: Side::Front,
            locked: None,
            properties: None,
            generator_parameters: None,
        }
    }

    fn replacement(operation: EditOperation) -> ProjectDoc {
        let EditOperation::ReplaceDocument { document } = operation else {
            panic!("placement must be one ReplaceDocument edit");
        };
        *document
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    fn controller_creation_is_one_document_edit_with_board_and_envelope_membership() {
        let accepted = fixture();
        let definition = controller_definition("catalog:controller");
        let placed = part(&definition.id, "part-controller", "U1");

        let proposed = replacement(
            placement_operation(&accepted, "board-main", &definition, &placed, None).unwrap(),
        );

        assert_eq!(proposed.revision, accepted.revision);
        assert_eq!(proposed.definitions, vec![definition]);
        assert_eq!(proposed.parts, vec![placed]);
        assert_eq!(proposed.boards[0].part_ids, vec!["part-controller"]);
        let OutlineFeature::PartEnvelope { part_ids, .. } = &proposed.outline[0] else {
            panic!("fixture outline is an envelope");
        };
        assert_eq!(part_ids, &["part-controller"]);
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    fn controller_creation_adds_selected_layout_membership_and_keeps_other_layouts_untouched() {
        let mut accepted = fixture();
        accepted.layouts = vec![
            boardstudio_core::model::Layout {
                id: "layout-main".into(),
                name: "Main".into(),
                board_id: "board-main".into(),
                matrix_id: "matrix-main".into(),
                part_ids: vec![],
                mirror_link: None,
            },
            boardstudio_core::model::Layout {
                id: "layout-other".into(),
                name: "Other".into(),
                board_id: "board-main".into(),
                matrix_id: "matrix-other".into(),
                part_ids: vec![],
                mirror_link: None,
            },
        ];
        let definition = controller_definition("catalog:controller");
        let placed = part(&definition.id, "part-controller", "U1");

        let proposed = replacement(
            placement_operation(
                &accepted,
                "board-main",
                &definition,
                &placed,
                Some("layout-main"),
            )
            .unwrap(),
        );

        assert_eq!(proposed.layouts[0].part_ids, vec!["part-controller"]);
        assert!(proposed.layouts[1].part_ids.is_empty());
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    fn standalone_placement_accepts_a_non_controller_and_updates_memberships_atomically() {
        let mut accepted = fixture();
        accepted.layouts = vec![
            boardstudio_core::model::Layout {
                id: "layout-main".into(),
                name: "Main".into(),
                board_id: "board-main".into(),
                matrix_id: "matrix-main".into(),
                part_ids: vec![],
                mirror_link: None,
            },
            boardstudio_core::model::Layout {
                id: "layout-other".into(),
                name: "Other".into(),
                board_id: "board-main".into(),
                matrix_id: "matrix-other".into(),
                part_ids: vec![],
                mirror_link: None,
            },
        ];
        let original = accepted.clone();
        let definition = passive_definition("imported:reset-switch");
        let placed = part(&definition.id, "part-reset-switch", "J1");

        let proposed = replacement(
            placement_operation(
                &accepted,
                "board-main",
                &definition,
                &placed,
                Some("layout-main"),
            )
            .expect("the ordinary component action accepts non-controller definitions"),
        );

        assert_eq!(proposed.definitions, vec![definition]);
        assert_eq!(proposed.parts, vec![placed]);
        assert_eq!(proposed.boards[0].part_ids, vec!["part-reset-switch"]);
        let OutlineFeature::PartEnvelope { part_ids, .. } = &proposed.outline[0] else {
            panic!("fixture outline is an envelope");
        };
        assert_eq!(part_ids, &["part-reset-switch"]);
        assert_eq!(proposed.layouts[0].part_ids, vec!["part-reset-switch"]);
        assert!(proposed.layouts[1].part_ids.is_empty());
        assert_eq!(
            accepted, original,
            "proposal construction does not mutate accepted state"
        );
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    fn selected_key_application_uses_set_matrix_and_preserves_other_cell_fields() {
        let mut accepted = fixture();
        accepted.matrices.push(boardstudio_core::model::Matrix {
            id: "matrix-main".into(),
            name: None,
            rows: 2,
            columns: 3,
            pitch: Vec2 { x: 19.05, y: 19.05 },
            origin: Vec2::default(),
            definition_id: "switch:base".into(),
            part_ids: vec![],
            board_id: Some("board-main".into()),
            mirror: None,
            rotation: None,
            edge_gap: None,
            diode_direction: None,
            row_offsets: vec![],
            column_offsets: vec![],
            column_staggers: vec![],
            column_splays: vec![],
            column_origins: vec![],
            cells: vec![MatrixCell {
                row: 1,
                column: 2,
                enabled: false,
                definition_id: None,
                variant: Some("existing-variant".into()),
                offset: Some(Vec2 { x: 0.5, y: -0.5 }),
                rotation: Some(12.0),
                assemblies: vec![],
                assemblies_local: Some(true),
            }],
        });
        let scope = Scope {
            session_epoch: SessionEpoch(7),
            document_id: "project".into(),
            board_id: "board-main".into(),
            instance_id: None,
        };
        let selection = ScopedTreeContext {
            scope: scope.clone(),
            context: TreeContext::Key {
                matrix_id: "matrix-main".into(),
                row: 1,
                column: 2,
            },
        };
        let original = accepted.clone();
        let operation = selected_key_component_operation(
            &accepted,
            &scope,
            &selection,
            &passive_definition("imported:reset-switch"),
        )
        .unwrap();
        let EditOperation::SetMatrix {
            matrix,
            definitions,
        } = operation
        else {
            panic!("applying to a selected key uses SetMatrix");
        };
        assert_eq!(definitions.unwrap()[0].id, "imported:reset-switch");
        assert_eq!(matrix.cells.len(), 1);
        let cell = &matrix.cells[0];
        assert!(cell.enabled);
        assert_eq!(cell.variant.as_deref(), Some("existing-variant"));
        assert_eq!(cell.offset, Some(Vec2 { x: 0.5, y: -0.5 }));
        assert_eq!(cell.rotation, Some(12.0));
        assert_eq!(cell.assemblies_local, Some(true));
        assert_eq!(cell.assemblies[0].id, "library-imported:reset-switch-1");
        assert_eq!(accepted, original);

        let operation = selected_key_component_operation(
            &accepted,
            &scope,
            &selection,
            &switch_definition("switch:mx"),
        )
        .unwrap();
        let EditOperation::SetMatrix {
            matrix,
            definitions,
        } = operation
        else {
            panic!("switch applies through SetMatrix");
        };
        assert_eq!(matrix.cells[0].definition_id.as_deref(), Some("switch:mx"));
        assert!(matrix.cells[0].assemblies.is_empty());
        assert_eq!(definitions.unwrap()[0].id, "switch:mx");
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    fn linked_target_key_replacement_stays_local_through_core_sync_and_history() {
        let mut document = fixture();
        document.definitions.push(switch_definition("switch:base"));
        let mut source = matrix_fixture();
        source.id = "matrix-source".into();
        source.cells.push(MatrixCell {
            row: 0,
            column: 0,
            enabled: true,
            definition_id: Some("switch:base".into()),
            variant: None,
            offset: None,
            rotation: None,
            assemblies: Vec::new(),
            assemblies_local: None,
        });
        let mut target = matrix_fixture();
        target.id = "matrix-target".into();
        document.matrices = vec![source, target];
        document.layouts = vec![
            boardstudio_core::model::Layout {
                id: "layout-source".into(),
                name: "Left".into(),
                board_id: "board-main".into(),
                matrix_id: "matrix-source".into(),
                part_ids: Vec::new(),
                mirror_link: None,
            },
            boardstudio_core::model::Layout {
                id: "layout-target".into(),
                name: "Right".into(),
                board_id: "board-main".into(),
                matrix_id: "matrix-target".into(),
                part_ids: Vec::new(),
                mirror_link: Some(boardstudio_core::model::LayoutMirrorLink {
                    source_id: "layout-source".into(),
                    axis_x: 0.0,
                }),
            },
        ];
        let mut engine = boardstudio_core::CoreEngine::new();
        let opened = engine.handle(boardstudio_core::model::CoreRequest::Open {
            id: "open-linked".into(),
            document,
        });
        let boardstudio_core::model::CoreReply::Scene {
            document: opened_document,
            ..
        } = opened
        else {
            panic!("the linked source fixture opens in Core");
        };
        let accepted = *opened_document;
        let target_before = accepted
            .matrices
            .iter()
            .find(|matrix| matrix.id == "matrix-target")
            .unwrap();
        assert_eq!(
            target_before.cells[0].definition_id.as_deref(),
            Some("switch:base")
        );
        assert_eq!(target_before.cells[0].assemblies_local, None);

        let scope = Scope {
            session_epoch: SessionEpoch(7),
            document_id: accepted.id.clone(),
            board_id: "board-main".into(),
            instance_id: None,
        };
        let selection = ScopedTreeContext {
            scope,
            context: TreeContext::Key {
                matrix_id: "matrix-target".into(),
                row: 0,
                column: 0,
            },
        };
        let EditOperation::SetMatrix {
            matrix,
            definitions,
        } = selected_key_component_operation(
            &accepted,
            &selection.scope,
            &selection,
            &switch_definition("switch:replacement"),
        )
        .unwrap()
        else {
            panic!("a switch replacement uses SetMatrix");
        };
        assert_eq!(
            matrix.cells[0].definition_id.as_deref(),
            Some("switch:replacement")
        );
        let replacement = engine.handle(boardstudio_core::model::CoreRequest::Edit {
            id: "replace-linked-key".into(),
            command: EditCommand {
                base_revision: accepted.revision,
                transaction_id: "replace-linked-key".into(),
                phase: EditPhase::Commit,
                target_ids: vec!["matrix-target".into(), "switch:replacement".into()],
                operation: EditOperation::SetMatrix {
                    matrix,
                    definitions,
                },
            },
        });
        let boardstudio_core::model::CoreReply::Scene {
            document: replaced_document,
            ..
        } = replacement
        else {
            panic!("Core accepts and synchronizes the linked-key replacement");
        };
        let replaced = *replaced_document;
        let target_after = replaced
            .matrices
            .iter()
            .find(|matrix| matrix.id == "matrix-target")
            .unwrap();
        assert_eq!(
            target_after.cells[0].definition_id.as_deref(),
            Some("switch:replacement")
        );
        assert_eq!(target_after.cells[0].assemblies_local, Some(true));

        let undone = engine.handle(boardstudio_core::model::CoreRequest::Undo {
            id: "undo-linked-key".into(),
        });
        let boardstudio_core::model::CoreReply::Scene {
            document: undone_document,
            ..
        } = undone
        else {
            panic!("the linked-key replacement is undoable");
        };
        let undone_target = undone_document
            .matrices
            .iter()
            .find(|matrix| matrix.id == "matrix-target")
            .unwrap();
        assert_eq!(
            undone_target.cells[0].definition_id.as_deref(),
            Some("switch:base")
        );
        assert_eq!(undone_target.cells[0].assemblies_local, None);

        let redone = engine.handle(boardstudio_core::model::CoreRequest::Redo {
            id: "redo-linked-key".into(),
        });
        let boardstudio_core::model::CoreReply::Scene {
            document: redone_document,
            ..
        } = redone
        else {
            panic!("the linked-key replacement is redoable");
        };
        let redone_target = redone_document
            .matrices
            .iter()
            .find(|matrix| matrix.id == "matrix-target")
            .unwrap();
        assert_eq!(
            redone_target.cells[0].definition_id.as_deref(),
            Some("switch:replacement")
        );
        assert_eq!(redone_target.cells[0].assemblies_local, Some(true));
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    fn conflicting_definitions_and_missing_layout_targets_reject_without_mutating_source() {
        let mut accepted = fixture();
        let original = accepted.clone();
        let definition = controller_definition("catalog:controller");
        let placed = part(&definition.id, "part-controller", "U1");
        accepted.definitions.push(
            serde_json::from_value(serde_json::json!({
                "id": "catalog:controller",
                "name": "Conflicting controller",
                "kind": "controller",
                "courtyard": [],
                "pads": []
            }))
            .unwrap(),
        );
        let conflict = placement_operation(&accepted, "board-main", &definition, &placed, None);
        assert!(conflict.is_err());
        assert_eq!(accepted.revision, original.revision);
        assert!(accepted.parts.is_empty());

        let missing_layout = placement_operation(
            &original,
            "board-main",
            &definition,
            &placed,
            Some("missing-layout"),
        );
        assert!(missing_layout.is_err());
        assert!(original.parts.is_empty());
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    fn controller_references_follow_existing_u_sequence_without_reusing_occupied_numbers() {
        let mut document = fixture();
        document.parts = vec![
            part("existing", "one", "U1"),
            part("existing", "two", "R3"),
            part("existing", "three", "U3"),
        ];
        assert_eq!(
            next_component_reference(&document, &PartKind::Controller),
            "U2"
        );
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    fn component_references_use_react_kind_prefixes_and_lowest_free_number() {
        let mut document = fixture();
        document.parts = vec![
            part("existing", "one", "S1"),
            part("existing", "two", "S3"),
            part("existing", "three", "U1"),
            part("existing", "four", "E2"),
            part("existing", "five", "J1"),
        ];
        assert_eq!(next_component_reference(&document, &PartKind::Switch), "S2");
        assert_eq!(
            next_component_reference(&document, &PartKind::Controller),
            "U2"
        );
        assert_eq!(
            next_component_reference(&document, &PartKind::Encoder),
            "E1"
        );
        assert_eq!(
            next_component_reference(&document, &PartKind::Passive),
            "J2"
        );
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    fn cursor_placement_uses_project_grid_and_alt_free_position_policy() {
        let definition = controller_definition("catalog:controller");
        let pending = controller_part(
            definition,
            "part-controller".into(),
            "U1".into(),
            Vec2::default(),
        )
        .unwrap();
        let point = Vec2 { x: 4.6, y: -4.6 };

        assert_eq!(
            grid_snap_point(point, 0.25),
            Vec2 {
                x: 4.7625,
                y: -4.7625
            }
        );
        assert_eq!(grid_snap_point(point, -0.5), Vec2 { x: 4.5, y: -4.5 });
        assert_eq!(grid_snap_point(point, 0.0), point);
        assert_eq!(
            snap_placement_at(
                &fixture(),
                "board-main",
                &pending,
                point,
                PlacementSnapOptions {
                    snap_fraction: 0.25,
                    geometry_snap: false,
                    gap: None,
                    free: false,
                }
            ),
            Vec2 {
                x: 4.7625,
                y: -4.7625
            }
        );
        assert_eq!(
            snap_placement_at(
                &fixture(),
                "board-main",
                &pending,
                point,
                PlacementSnapOptions {
                    snap_fraction: 0.25,
                    geometry_snap: true,
                    gap: None,
                    free: true,
                }
            ),
            point
        );
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    fn initial_placement_uses_world_view_center_after_pan() {
        assert_eq!(
            canvas_world_center(-40.0, 60.0, -20.0, 80.0, Vec2 { x: 13.0, y: -7.0 }),
            Vec2 { x: 23.0, y: 23.0 }
        );
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    fn placement_commits_only_on_primary_pointer_release() {
        assert!(pointer_release_commits(0));
        assert!(!pointer_release_commits(1));
        assert!(!pointer_release_commits(2));
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    fn placement_geometry_snap_uses_pending_definition_and_current_gap_preference() {
        let mut document = fixture();
        let obstacle_definition = controller_definition("existing:controller");
        document.definitions.push(obstacle_definition.clone());
        let mut obstacle = part(&obstacle_definition.id, "existing-controller", "U2");
        obstacle.pose.at = Vec2 { x: 11.0, y: 0.0 };
        document.parts.push(obstacle);
        document.boards[0]
            .part_ids
            .push("existing-controller".into());

        let definition = controller_definition("catalog:controller");
        let pending = controller_part(
            definition.clone(),
            "part-controller".into(),
            "U1".into(),
            Vec2::default(),
        )
        .unwrap();
        let snap_document = document_with_pending_definition(&document, &definition);
        let at = snap_placement_at(
            &snap_document,
            "board-main",
            &pending,
            Vec2 { x: 10.2, y: 0.0 },
            PlacementSnapOptions {
                snap_fraction: 0.25,
                geometry_snap: true,
                gap: None,
                free: false,
            },
        );
        assert_eq!(at.x, 11.0);

        let mut settings = LayoutSnapSettings::default();
        assert_eq!(placement_gap(&settings), Some(1.0));
        settings.gap_override = "0.7".into();
        assert_eq!(placement_gap(&settings), Some(0.7));
        settings.gap_snap = false;
        assert_eq!(placement_gap(&settings), None);
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    fn captured_owner_rejects_changed_snapshot_scope_generation_or_guide_stage() {
        let initial = accepted(fixture(), 11);
        let scope = Scope {
            session_epoch: SessionEpoch(7),
            document_id: "project".into(),
            board_id: "board-main".into(),
            instance_id: None,
        };
        let owner = PlacementOwner::capture(
            &initial,
            PlacementCapture {
                scope: scope.clone(),
                generation: 4,
                part_id: "part-controller".into(),
                definition_id: "catalog:controller".into(),
                kind: PartKind::Controller,
                workflow: PlacementWorkflow::WiringController,
                source_workspace: "Parts",
                layout_id: None,
                at: Vec2 { x: 3.0, y: 9.0 },
            },
        )
        .unwrap();

        assert!(owner.is_current(&initial, Some(&scope), 4, true, Some("project"), true));
        assert!(!owner.is_current(
            &accepted(fixture(), 12),
            Some(&scope),
            4,
            true,
            Some("project"),
            true
        ));
        assert!(!owner.is_current(&initial, Some(&scope), 5, true, Some("project"), true));
        assert!(!owner.is_current(&initial, Some(&scope), 4, false, Some("project"), true));
        assert!(!owner.is_current(&initial, Some(&scope), 4, true, Some("other"), true));
        assert!(!owner.is_current(&initial, Some(&scope), 4, true, Some("project"), false));
    }
}

/// What a Layout view-mode switch must cancel before entering 3D.
pub struct LayoutPlacementCancellation {
    pub parts: PartPlacementMount,
    pub matrices: crate::objects::MatrixPlacementMount,
    pub interactions: crate::canvas_interaction::CanvasInteractionArbiter,
}

pub fn layout_view_mode_handler(
    is_owner_current: impl Fn() -> bool + 'static,
    is_assembly_3d: Signal<bool>,
    mut set_assembly_3d: impl FnMut(bool) + 'static,
    placements: LayoutPlacementCancellation,
    before_placement_cancel: impl Fn() + 'static,
    mut after_placement_cancel: impl FnMut() + 'static,
) -> EventHandler<bool> {
    EventHandler::new(move |assembly_3d| {
        if !is_owner_current() {
            return;
        }
        if assembly_3d && !is_assembly_3d() {
            before_placement_cancel();
            if placements.parts.busy || placements.parts.projection.is_some() {
                placements.parts.on_cancel.call(());
            }
            after_placement_cancel();
            match placements.interactions.current() {
                Some(CanvasInteractionOwner::PartPlacement) => {
                    placements
                        .interactions
                        .release(CanvasInteractionOwner::PartPlacement);
                }
                Some(CanvasInteractionOwner::OutlinePerimeter) => {
                    placements
                        .interactions
                        .release(CanvasInteractionOwner::OutlinePerimeter);
                }
                Some(CanvasInteractionOwner::MatrixPlacement) => {
                    if let Some(owner) = placements.matrices.cancel_owner.clone() {
                        placements.matrices.on_cancel.call(owner);
                    }
                }
                Some(CanvasInteractionOwner::MatrixTransform) => {
                    placements
                        .interactions
                        .release(CanvasInteractionOwner::MatrixTransform);
                }
                Some(CanvasInteractionOwner::MirroredPair) | None => {}
            }
        }
        set_assembly_3d(assembly_3d);
    })
}
