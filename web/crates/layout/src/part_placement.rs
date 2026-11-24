//! Catalogue component placement: pure ownership/resolver helpers and the browser
//! session owning definition loading, snap preview, and synchronous commit/cancel routing.

pub mod plan;

#[cfg(target_arch = "wasm32")]
pub use boardstudio_web_ui_model::state::ComponentPlacementAction;
pub use plan::{
    PendingPart, PlacementSnapOptions, canvas_world_center, component_part, controller_part,
    next_component_reference, placement_gap, placement_operation, pointer_release_commits,
    snap_placement_at, update_pending_part,
};

pub(crate) use plan::PlacementIntent;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod key_component_tests;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod plan_tests;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod test_support;

#[cfg(target_arch = "wasm32")]
pub use browser::{DefinitionLoader, PartPlacementHost, PartPlacementSession, use_part_placement};

use boardstudio_application::{
    AcceptedSnapshot, EditResolver, Resolution, Scope, SessionEpoch, SnapshotToken,
};
use boardstudio_model::model::{
    EditOperation, MatrixAssembly, MatrixCell, Part, PartDefinition, PartKind, ProjectDoc, Vec2,
};
use boardstudio_web_catalogue::matrix_input::matrix_input_available;
use boardstudio_web_ui_model::Workspace;
use boardstudio_web_ui_model::tree::{ScopedTreeContext, TreeContext, matrix_visible_on_board};
use plan::placement_operation_for_intent;
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

#[derive(Clone, Debug, PartialEq)]
pub struct PlacementOwner {
    pub token: SnapshotToken,
    pub session_epoch: SessionEpoch,
    pub project_id: String,
    pub revision: u64,
    pub scope: Scope,
    pub generation: u64,
    pub(crate) source_workspace: Workspace,
    pub board_id: String,
    pub part_id: String,
    pub definition_id: String,
    pub reference: String,
    pub layout_id: Option<String>,
    pub at: Vec2,
    pub(crate) workflow: PlacementWorkflow,
    pub(crate) intent: PlacementIntent,
}

pub(crate) struct PlacementCapture<'a> {
    pub scope: Scope,
    pub generation: u64,
    pub part_id: String,
    pub definition_id: String,
    pub kind: &'a PartKind,
    pub workflow: PlacementWorkflow,
    pub intent: PlacementIntent,
    pub source_workspace: Workspace,
    pub layout_id: Option<String>,
    pub at: Vec2,
}

impl PlacementOwner {
    pub(crate) fn capture(snap: &AcceptedSnapshot, c: PlacementCapture<'_>) -> Option<Self> {
        if snap.document.id != c.scope.document_id
            || snap.session_epoch != c.scope.session_epoch
            || !snap
                .document
                .boards
                .iter()
                .any(|b| b.id == c.scope.board_id)
        {
            return None;
        }
        Some(Self {
            token: snap.token,
            session_epoch: snap.session_epoch,
            project_id: snap.document.id.clone(),
            revision: snap.document.revision,
            board_id: c.scope.board_id.clone(),
            scope: c.scope,
            generation: c.generation,
            source_workspace: c.source_workspace,
            part_id: c.part_id,
            definition_id: c.definition_id,
            reference: next_component_reference(&snap.document, c.kind),
            layout_id: c.layout_id,
            at: c.at,
            workflow: c.workflow,
            intent: c.intent,
        })
    }

    pub fn is_current(
        &self,
        snap: &AcceptedSnapshot,
        scope: Option<&Scope>,
        generation: u64,
        guide_open: bool,
        guide_project_id: Option<&str>,
        guide_stage_is_wiring: bool,
    ) -> bool {
        scope == Some(&self.scope)
            && generation == self.generation
            && snap.token == self.token
            && snap.session_epoch == self.session_epoch
            && snap.document.id == self.project_id
            && snap.document.revision == self.revision
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

pub(crate) fn placement_resolver(
    board_id: String,
    definition: PartDefinition,
    part: Part,
    layout_id: Option<String>,
    intent: PlacementIntent,
) -> EditResolver {
    EditResolver::new(
        "layout-component-placement",
        move |acc: &AcceptedSnapshot| match placement_operation_for_intent(
            &acc.document,
            &board_id,
            &definition,
            &part,
            layout_id.as_deref(),
            intent,
        ) {
            Ok(op) => Resolution::submit(vec![part.id.clone()], op),
            Err(msg) => Resolution::Retire(msg),
        },
    )
}

/// Resolve applying a loaded component definition to the selected key.
pub(crate) fn key_component_resolver(
    scope: Scope,
    selection: ScopedTreeContext,
    definition: PartDefinition,
) -> EditResolver {
    EditResolver::new("layout-key-component", move |acc: &AcceptedSnapshot| {
        let op = match selected_key_component_operation(
            &acc.document,
            &scope,
            &selection,
            &definition,
        ) {
            Ok(op) => op,
            Err(msg) => return Resolution::Retire(msg),
        };
        let EditOperation::SetMatrix {
            matrix,
            definitions,
        } = &op
        else {
            return Resolution::Retire("The selected key can no longer be edited.".into());
        };
        let mut ids = vec![matrix.id.clone()];
        if definitions.is_some() {
            ids.push(definition.id.clone());
        }
        Resolution::submit(ids, op)
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
    if scope.document_id != accepted.id || !accepted.boards.iter().any(|b| b.id == scope.board_id) {
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
        .find(|m| {
            m.id == *matrix_id && matrix_visible_on_board(accepted, &scope.board_id, matrix_id)
        })
        .filter(|m| {
            m.board_id.as_deref().is_none_or(|id| id == scope.board_id)
                && *row < m.rows
                && *column < m.columns
        })
        .cloned()
        .ok_or_else(|| "The selected matrix key is no longer available.".to_string())?;
    let current = matrix
        .cells
        .iter()
        .find(|c| c.row == *row && c.column == *column)
        .cloned();
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
    if matches!(definition.kind, PartKind::Switch) || matrix_input_available(definition) {
        cell.definition_id = Some(definition.id.clone());
    } else {
        let safe: String = definition
            .id
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || matches!(c, '-' | '_') {
                    c
                } else {
                    '-'
                }
            })
            .collect();
        let id = ((cell.assemblies.len() + 1)..)
            .map(|n| format!("library-{safe}-{n}"))
            .find(|cand| !cell.assemblies.iter().any(|a| a.id == *cand))
            .unwrap_or_else(|| format!("library-{safe}-next"));
        cell.assemblies.push(MatrixAssembly {
            id,
            definition_id: definition.id.clone(),
            offset: Vec2::default(),
            rotation: None,
            side: None,
        });
    }
    let changed = current.as_ref().is_none_or(|p| {
        p.definition_id != cell.definition_id
            || p.variant != cell.variant
            || p.assemblies != cell.assemblies
    });
    if changed
        && accepted
            .layouts
            .iter()
            .any(|l| l.matrix_ids.iter().any(|id| id == matrix_id) && l.mirror_link.is_some())
    {
        cell.assemblies_local = Some(true);
    }
    matrix
        .cells
        .retain(|e| e.row != *row || e.column != *column);
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

#[cfg(target_arch = "wasm32")]
mod browser {
    use super::plan::{
        PlacementIntent, document_with_pending_definition, grid_snap_point,
        placement_source_definition, update_pending_part,
    };
    use super::{
        ActivePartPlacement, ComponentPlacementAction, PlacementCapture, PlacementOwner,
        PlacementWorkflow, component_part, controller_part, key_component_resolver,
        placement_resolver,
    };
    use crate::canvas_interaction::{CanvasMode, LayoutInteraction};
    use crate::objects::{self, LayoutSnapSettings};
    use crate::parts::{PartsQuery, PartsSelection};
    use crate::runtime::Runtime;
    use crate::selection::SelectionAdapter;
    use crate::setup_guide::{SetupGuidePreferences, SetupGuideStage};
    use boardstudio_application::{
        AcceptedSnapshot, Durability, Event, Lifecycle, ReadModel, Scope, SelectionMode,
    };
    use boardstudio_model::model::{PartDefinition, PartKind, ProjectDoc, Vec2};
    use boardstudio_web_ui_model::Workspace;
    use boardstudio_web_ui_model::tree::{ScopedTreeContext, TreeContext};
    use dioxus::prelude::*;
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
    };
    use wasm_bindgen::{JsCast, JsValue};
    use wasm_bindgen_futures::spawn_local;

    #[derive(Clone)]
    pub struct PartPlacementSession {
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

    pub struct PartPlacementHost {
        pub runtime: Rc<Runtime>,
        pub load_definition: DefinitionLoader,
        pub workspace: Signal<Workspace>,
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
    }

    pub type DefinitionLoader = Rc<
        dyn Fn(
            ProjectDoc,
            String,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<PartDefinition, String>>>,
        >,
    >;

    #[derive(Clone, Default)]
    struct PlacementUi {
        request_id: u64,
        chooser: Option<(AcceptedSnapshot, Scope, u64)>,
        preparing: Option<(u64, PlacementOwner)>,
        active: Option<ActivePartPlacement>,
        error: Option<String>,
    }

    #[derive(Clone)]
    struct Ctx {
        host: Rc<PartPlacementHost>,
        ui: Signal<PlacementUi>,
        alive: Rc<Cell<bool>>,
        inter: LayoutInteraction,
    }

    type RenderGuard = (
        AcceptedSnapshot,
        Scope,
        u64,
        Workspace,
        Option<ScopedTreeContext>,
    );

    fn snapshot_saved(rt: &Runtime, acc: &AcceptedSnapshot) -> bool {
        let m = rt.model();
        m.lifecycle == Lifecycle::Ready
            && m.durability
                == (Durability::Saved {
                    revision: acc.document.revision,
                })
            && m.accepted.as_ref().is_some_and(|c| {
                c.token == acc.token
                    && c.session_epoch == acc.session_epoch
                    && c.document.id == acc.document.id
                    && c.document.revision == acc.document.revision
            })
    }

    fn chooser_live(
        chooser: Option<&(AcceptedSnapshot, Scope, u64)>,
        rt: &Runtime,
        generation: u64,
        ws: Workspace,
    ) -> bool {
        chooser.is_some_and(|(a, s, g)| {
            ws == Workspace::Parts
                && generation == *g
                && rt.scope().as_ref() == Some(s)
                && snapshot_saved(rt, a)
        })
    }

    fn guide_wiring(prefs: &Option<SetupGuidePreferences>, project_id: &str) -> bool {
        prefs.as_ref().is_some_and(|p| {
            p.open && p.project_id == project_id && p.current_stage == SetupGuideStage::Wiring
        })
    }

    fn owner_live(
        owner: &PlacementOwner,
        rt: &Runtime,
        m: &ReadModel,
        generation: u64,
        ws: Workspace,
        expected_ws: Workspace,
        prefs: &Option<SetupGuidePreferences>,
    ) -> bool {
        let Some(snap) = m.accepted.as_ref() else {
            return false;
        };
        ws == expected_ws
            && m.active_board_id == owner.board_id
            && rt.scope().as_ref() == Some(&owner.scope)
            && owner.is_current(
                snap,
                rt.scope().as_ref(),
                generation,
                prefs.as_ref().is_some_and(|p| p.open),
                prefs.as_ref().map(|p| p.project_id.as_str()),
                prefs
                    .as_ref()
                    .is_some_and(|p| p.current_stage == SetupGuideStage::Wiring),
            )
    }

    fn guard_live(
        guard: Option<&RenderGuard>,
        host: &PartPlacementHost,
        expected: Workspace,
    ) -> bool {
        guard.is_some_and(|(a, s, generation, ws, sel)| {
            *ws == expected
                && (host.workspace)() == *ws
                && (host.generation)() == *generation
                && (host.adapter.selected_context)() == *sel
                && host.runtime.scope().as_ref() == Some(s)
                && snapshot_saved(&host.runtime, a)
        })
    }

    impl Ctx {
        fn owner_current(
            &self,
            o: &PlacementOwner,
            m: &ReadModel,
            gen_id: u64,
            ws: Workspace,
            expected_ws: Workspace,
            prefs: &Option<SetupGuidePreferences>,
        ) -> bool {
            owner_live(o, &self.host.runtime, m, gen_id, ws, expected_ws, prefs)
        }

        fn prune_with(
            &self,
            m: &ReadModel,
            gen_id: u64,
            ws: Workspace,
            prefs: &Option<SetupGuidePreferences>,
        ) {
            let (drop_chooser, drop_active, drop_preparing) = {
                let s = self.ui.peek();
                (
                    s.chooser.is_some()
                        && !chooser_live(s.chooser.as_ref(), &self.host.runtime, gen_id, ws),
                    s.active.as_ref().is_some_and(|p| {
                        !self.owner_current(&p.owner, m, gen_id, ws, Workspace::Layout, prefs)
                    }),
                    s.preparing.as_ref().is_some_and(|(_, o)| {
                        !self.owner_current(o, m, gen_id, ws, o.source_workspace, prefs)
                    }),
                )
            };
            if !drop_chooser && !drop_active && !drop_preparing {
                return;
            }
            let mut ui = self.ui;
            let mut s = ui.write();
            if drop_chooser {
                s.chooser = None;
            }
            if drop_active {
                s.active = None;
            }
            if drop_preparing {
                s.preparing = None;
            }
        }

        fn reset_selection(
            &self,
            ws: Workspace,
            context: Option<ScopedTreeContext>,
            part_ids: Vec<String>,
        ) {
            let (mut sel, mut anchor, mut ws_s) = (
                self.host.adapter.selected_context,
                self.host.adapter.anchor_scope,
                self.host.workspace,
            );
            sel.set(context);
            anchor.set(None);
            ws_s.set(ws);
            self.host.runtime.submit(Event::SelectParts {
                operation_id: self.host.runtime.operation(),
                part_ids,
                range_part_ids: vec![],
                mode: SelectionMode::Replace,
            });
        }
    }

    pub fn use_part_placement(host: PartPlacementHost) -> PartPlacementSession {
        let inter = use_context::<LayoutInteraction>();
        let ui = use_signal(PlacementUi::default);
        let alive = use_hook(|| Rc::new(Cell::new(true)));
        use_drop({
            let a = alive.clone();
            move || a.set(false)
        });

        let h = Rc::new(host);
        let c = Ctx {
            host: h.clone(),
            ui,
            alive,
            inter,
        };
        let render_snap = h.runtime.model().accepted.filter(|a| {
            snapshot_saved(&h.runtime, a)
                && h.runtime.scope().as_ref().is_some_and(|s| {
                    a.document.id == s.document_id && a.session_epoch == s.session_epoch
                })
        });
        let guard = render_snap.zip(h.runtime.scope()).map(|(a, s)| {
            (
                a,
                s,
                (h.generation)(),
                (h.workspace)(),
                (h.adapter.selected_context)(),
            )
        });
        let ordinary_ctrl = chooser_live(
            ui.read().chooser.as_ref(),
            &h.runtime,
            (h.generation)(),
            (h.workspace)(),
        );
        let guided_ctrl = h
            .runtime
            .model()
            .accepted
            .as_ref()
            .is_some_and(|s| guide_wiring(&(h.guide_preferences)(), &s.document.id));

        let (obs_ver, obs_gen, obs_ws, obs_guide) = (
            (h.version)(),
            (h.generation)(),
            (h.workspace)(),
            (h.guide_preferences)(),
        );
        let c_eff = c.clone();
        use_effect(use_reactive!(|obs_ver, obs_gen, obs_ws, obs_guide| {
            let _ = obs_ver;
            c_eff.prune_with(&c_eff.host.runtime.model(), obs_gen, obs_ws, &obs_guide);
        }));

        let latest = use_hook(|| Rc::new(RefCell::new((c.clone(), guard.clone(), ordinary_ctrl))));
        *latest.borrow_mut() = (c, guard, ordinary_ctrl);

        let on_choose_controller = use_callback({
            let latest = latest.clone();
            move |_| {
                let (c, guard, _) = latest.borrow().clone();
                choose_controller(&c, guard.as_ref());
            }
        });
        let on_place_controller = use_callback({
            let latest = latest.clone();
            move |id| {
                let (c, guard, ordinary) = latest.borrow().clone();
                let wf = if ordinary {
                    PlacementWorkflow::PcbController
                } else {
                    PlacementWorkflow::WiringController
                };
                if wf != PlacementWorkflow::PcbController
                    || guard_live(guard.as_ref(), &c.host, Workspace::Parts)
                {
                    start_placement(
                        &c,
                        id,
                        PartKind::Controller,
                        wf,
                        false,
                        None,
                        PlacementIntent::Existing,
                    );
                }
            }
        });
        let on_controller_back = use_callback({
            let latest = latest.clone();
            move |_| {
                let (c, _, _) = latest.borrow().clone();
                if chooser_live(
                    c.ui.read().chooser.as_ref(),
                    &c.host.runtime,
                    (c.host.generation)(),
                    (c.host.workspace)(),
                ) && c.inter.current().is_none()
                {
                    let (mut ui, mut ws) = (c.ui, c.host.workspace);
                    ui.write().chooser = None;
                    ws.set(Workspace::Pcb);
                }
            }
        });
        let on_place_component = use_callback({
            let latest = latest.clone();
            move |a| {
                let (c, guard, _) = latest.borrow().clone();
                place_component(&c, guard.as_ref(), a);
            }
        });
        let on_move = use_callback(move |at| {
            if inter.current() == Some(CanvasMode::PlacingPart) {
                let mut ui = ui;
                if let Some(p) = ui.write().active.as_mut() {
                    update_pending_part(&mut p.pending, at);
                }
            }
        });
        let on_commit = use_callback({
            let latest = latest.clone();
            move |at| {
                let (c, _, _) = latest.borrow().clone();
                commit_placement(&c, at);
            }
        });
        let on_cancel = use_callback(move |_| {
            let (c, _, _) = latest.borrow().clone();
            cancel_placement(&c);
        });

        let (projection, busy, error) = {
            let s = ui.read();
            let projection = s
                .active
                .as_ref()
                .filter(|p| {
                    owner_live(
                        &p.owner,
                        &h.runtime,
                        &h.runtime.model(),
                        (h.generation)(),
                        (h.workspace)(),
                        Workspace::Layout,
                        &(h.guide_preferences)(),
                    )
                })
                .cloned();
            (projection, s.preparing.is_some(), s.error.clone())
        };
        let owns_canvas = busy || projection.is_some();
        use_effect(use_reactive(
            (
                &(h.version)(),
                &(h.workspace)(),
                &(h.generation)(),
                &owns_canvas,
            ),
            move |(_, _, _, owns)| {
                if !owns {
                    inter.end(CanvasMode::PlacingPart);
                }
            },
        ));

        PartPlacementSession {
            projection,
            busy,
            error,
            on_choose_controller,
            on_place_controller,
            controller_placement_enabled: ordinary_ctrl || guided_ctrl,
            controller_back: ordinary_ctrl.then_some(on_controller_back),
            on_place_component,
            on_move,
            on_commit,
            on_cancel,
        }
    }

    fn choose_controller(c: &Ctx, guard: Option<&RenderGuard>) {
        if c.inter.current().is_some() {
            return;
        }
        let Some((snap, scope, generation, ws, _)) = guard else {
            return;
        };
        if !guard_live(guard, &c.host, *ws) {
            return;
        }
        let guided = guide_wiring(&(c.host.guide_preferences)(), &snap.document.id);
        if !guided && *ws != Workspace::Pcb {
            return;
        }
        let mut ui = c.ui;
        ui.write().chooser = (!guided).then(|| (snap.clone(), scope.clone(), *generation));
        let (mut q, mut p_sel, mut obj, mut insp) = (
            c.host.parts_query,
            c.host.parts_selection,
            c.host.objects_open,
            c.host.inspect_open,
        );
        q.set("controller".into());
        p_sel.set(None);
        obj.set(true);
        insp.set(!guided);
        c.reset_selection(
            if guided {
                Workspace::Layout
            } else {
                Workspace::Parts
            },
            None,
            vec![],
        );
    }

    fn place_component(c: &Ctx, guard: Option<&RenderGuard>, action: ComponentPlacementAction) {
        let expected = match &action {
            ComponentPlacementAction::PartsInspector { .. } => Workspace::Parts,
            _ => match guard
                .filter(|(_, _, _, w, _)| matches!(w, Workspace::Layout | Workspace::Pcb))
            {
                Some((_, _, _, w, _)) => *w,
                None => return,
            },
        };
        if !guard_live(guard, &c.host, expected) {
            return;
        }
        let (def_id, kind, apply_to_key, src_def, intent) = match action {
            ComponentPlacementAction::AddObject {
                definition_id,
                kind,
            } => (
                definition_id,
                kind,
                false,
                None,
                PlacementIntent::LinkedCatalogueDefault,
            ),
            ComponentPlacementAction::AddMirrorOnlyObject {
                definition_id,
                kind,
            } => (
                definition_id,
                kind,
                false,
                None,
                PlacementIntent::MirrorOnly,
            ),
            ComponentPlacementAction::AddSourceObject {
                module_definition_id,
                definition,
            } => (
                definition.id.clone(),
                definition.kind.clone(),
                false,
                Some((module_definition_id, definition)),
                PlacementIntent::Existing,
            ),
            ComponentPlacementAction::PartsInspector {
                definition_id,
                kind,
            } => (definition_id, kind, true, None, PlacementIntent::Existing),
        };
        start_placement(
            c,
            def_id,
            kind,
            PlacementWorkflow::GeneralComponent,
            apply_to_key,
            src_def,
            intent,
        );
    }

    fn cancel_placement(c: &Ctx) {
        let owner_ws = {
            let s = c.ui.read();
            s.active
                .as_ref()
                .map(|p| (p.owner.clone(), Workspace::Layout))
                .or_else(|| {
                    s.preparing
                        .as_ref()
                        .map(|(_, o)| (o.clone(), o.source_workspace))
                })
        };
        let Some((owner, expected_ws)) = owner_ws else {
            return;
        };
        let m = c.host.runtime.model();
        let current = m.accepted.is_some()
            && c.owner_current(
                &owner,
                &m,
                (c.host.generation)(),
                (c.host.workspace)(),
                expected_ws,
                &(c.host.guide_preferences)(),
            );
        {
            let mut ui = c.ui;
            let mut s = ui.write();
            s.active = None;
            s.preparing = None;
        }
        c.inter.end(CanvasMode::PlacingPart);
        if current {
            let ws = if owner.workflow == PlacementWorkflow::PcbController {
                Workspace::Pcb
            } else {
                Workspace::Layout
            };
            c.reset_selection(ws, None, vec![]);
        }
    }

    fn commit_placement(c: &Ctx, at: Vec2) {
        if c.inter.current() != Some(CanvasMode::PlacingPart) {
            return;
        }
        let Some(mut placement) = c.ui.read().active.clone() else {
            return;
        };
        update_pending_part(&mut placement.pending, at);
        placement.owner.at = placement.pending.at;
        let model = c.host.runtime.model();
        if model.accepted.is_none()
            || !c.owner_current(
                &placement.owner,
                &model,
                (c.host.generation)(),
                (c.host.workspace)(),
                Workspace::Layout,
                &(c.host.guide_preferences)(),
            )
            || model.lifecycle != Lifecycle::Ready
            || model.durability
                != (Durability::Saved {
                    revision: placement.owner.revision,
                })
        {
            return;
        }
        let owner = placement.owner;
        {
            let mut ui = c.ui;
            let mut s = ui.write();
            s.active = None;
            s.error = None;
        }
        c.inter.end(CanvasMode::PlacingPart);
        let res = c.host.runtime.commit(
            "layout-component-placement",
            placement_resolver(
                owner.board_id.clone(),
                placement.pending.definition,
                placement.pending.part,
                owner.layout_id.clone(),
                owner.intent,
            ),
        );
        let (mut ws, mut kind) = (c.host.workspace, c.host.layout_selection_kind);
        match res {
            Ok(snap) => {
                // Select from the accepted document at landing; nothing when the placed part is no longer there.
                let general = owner.workflow == PlacementWorkflow::GeneralComponent;
                let placed = snap.document.parts.iter().any(|p| p.id == owner.part_id);
                let updated = c.host.runtime.model();
                let ctx = (general && placed)
                    .then(|| objects::context_for_part(&updated, &owner.part_id))
                    .flatten()
                    .map(|context| ScopedTreeContext {
                        scope: owner.scope.clone(),
                        context,
                    });
                if general {
                    kind.set(objects::LayoutSelectionKind::Part);
                }
                c.reset_selection(
                    if owner.workflow == PlacementWorkflow::PcbController {
                        Workspace::Pcb
                    } else {
                        Workspace::Layout
                    },
                    ctx,
                    if general && placed {
                        vec![owner.part_id]
                    } else {
                        vec![]
                    },
                );
            }
            Err(msg) => {
                ws.set(owner.source_workspace);
                c.ui.clone().write().error = Some(msg);
            }
        }
    }

    fn browser_uuid() -> Result<String, String> {
        let crypto = js_sys::Reflect::get(&js_sys::global(), &JsValue::from_str("crypto"))
            .map_err(|e| format!("Could not access browser identity service: {e:?}"))?;
        let random_uuid = js_sys::Reflect::get(&crypto, &JsValue::from_str("randomUUID"))
            .map_err(|e| format!("Could not access browser identity service: {e:?}"))?
            .dyn_into::<js_sys::Function>()
            .map_err(|_| "The browser cannot generate a safe component identity.".to_string())?;
        random_uuid
            .call0(&crypto)
            .map_err(|e| format!("Could not generate a component identity: {e:?}"))?
            .as_string()
            .ok_or_else(|| "Could not generate a component identity.".into())
    }

    fn start_placement(
        c: &Ctx,
        def_id: String,
        kind: PartKind,
        workflow: PlacementWorkflow,
        apply_to_key: bool,
        source_definition: Option<(String, PartDefinition)>,
        intent: PlacementIntent,
    ) {
        let mut ui = c.ui;
        let (gen_id, source_ws, prefs) = (
            (c.host.generation)(),
            (c.host.workspace)(),
            (c.host.guide_preferences)(),
        );
        let ws_ok = match workflow {
            PlacementWorkflow::WiringController => {
                source_ws == Workspace::Parts
                    || (source_ws == Workspace::Layout
                        && prefs
                            .as_ref()
                            .is_some_and(|p| p.open && p.current_stage == SetupGuideStage::Wiring))
            }
            PlacementWorkflow::PcbController => source_ws == Workspace::Parts,
            PlacementWorkflow::GeneralComponent => matches!(
                source_ws,
                Workspace::Parts | Workspace::Layout | Workspace::Pcb
            ),
        };
        if !ws_ok {
            return;
        }
        let model = c.host.runtime.model();
        c.prune_with(&model, gen_id, source_ws, &prefs);
        if ui.read().active.is_some()
            || ui.read().preparing.is_some()
            || (c.host.workspace)() != source_ws
        {
            return;
        }
        let (Some(snap), Some(scope)) = (model.accepted.as_ref(), c.host.runtime.scope()) else {
            return;
        };
        if apply_to_key && source_definition.is_some() {
            ui.write().error =
                Some("A module source component cannot be applied to a matrix key.".into());
            return;
        }
        let src_def =
            source_definition.map(|(m, d)| placement_source_definition(&snap.document, &m, d));
        let def_id = src_def.as_ref().map(|d| d.id.clone()).unwrap_or(def_id);
        let kind = src_def.as_ref().map(|d| d.kind.clone()).unwrap_or(kind);
        let pcb_live = chooser_live(
            ui.read().chooser.as_ref(),
            &c.host.runtime,
            gen_id,
            source_ws,
        );
        if (workflow == PlacementWorkflow::WiringController
            && !guide_wiring(&prefs, &snap.document.id))
            || (workflow == PlacementWorkflow::PcbController && !pcb_live)
            || model.lifecycle != Lifecycle::Ready
            || model.durability
                != (Durability::Saved {
                    revision: snap.document.revision,
                })
            || model.display_preview.is_some()
            || model.gesture.is_some()
            || model.active_board_id != scope.board_id
        {
            return;
        }
        let mut next_req = || {
            let mut s = ui.write();
            s.request_id = s.request_id.saturating_add(1);
            s.request_id
        };
        if apply_to_key
            && let Some(selected) = (c.host.adapter.selected_context)()
                .filter(|s| matches!(s.context, TreeContext::Key { .. }))
        {
            if selected.scope != scope
                || !crate::selection::context_is_current(&model, &scope, &selected.context)
            {
                return;
            }
            let (c2, snap, req_id) = (c.clone(), snap.clone(), next_req());
            spawn_local(async move {
                apply_key_component(
                    &c2,
                    &snap,
                    (gen_id, source_ws, selected, def_id, kind, req_id),
                )
                .await;
            });
            return;
        }
        let part_id = match browser_uuid() {
            Ok(id) => format!("ui-{id}"),
            Err(msg) => {
                ui.write().error = Some(msg);
                return;
            }
        };
        // Ticket 22: the initial placement grid follows the selected matrix pitch (19.05 mm when none).
        let pitch = (c.host.adapter.selected_context)()
            .and_then(|s| crate::objects::matrix_id_for_context(&s.context).map(str::to_owned))
            .and_then(|id| snap.document.matrices.iter().find(|m| m.id == id))
            .map(|m| m.pitch);
        let at = grid_snap_point(
            c.host.canvas_center,
            c.host.snap_settings.read().snap_fraction,
            pitch,
        );
        let layout_id = (c.host.layout_target)().filter(|id| {
            snap.document
                .layouts
                .iter()
                .any(|l| l.id == *id && l.board_id == scope.board_id)
        });
        let Some(owner) = PlacementOwner::capture(
            snap,
            PlacementCapture {
                scope,
                generation: gen_id,
                part_id,
                definition_id: def_id.clone(),
                kind: &kind,
                workflow,
                intent,
                source_workspace: source_ws,
                layout_id,
                at,
            },
        ) else {
            return;
        };
        if !c.inter.begin(CanvasMode::PlacingPart) {
            return;
        }
        let req_id = next_req();
        {
            let mut s = ui.write();
            s.preparing = Some((req_id, owner.clone()));
            s.error = None;
        }
        let (c2, snap) = (c.clone(), snap.clone());
        spawn_local(async move {
            prepare_component(&c2, &snap, owner, (def_id, kind, workflow, src_def, req_id)).await;
        });
    }

    async fn apply_key_component(
        c: &Ctx,
        accepted: &AcceptedSnapshot,
        (accepted_gen, source_ws, selected, def_id, kind, req_id): (
            u64,
            Workspace,
            ScopedTreeContext,
            String,
            PartKind,
            u64,
        ),
    ) {
        let loaded = (c.host.load_definition)((*accepted.document).clone(), def_id).await;
        if !c.alive.get() || c.ui.read().request_id != req_id {
            return;
        }
        let route_current = c.host.runtime.scope().as_ref() == Some(&selected.scope)
            && (c.host.adapter.selected_context)().as_ref() == Some(&selected)
            && (c.host.workspace)() == source_ws
            && (c.host.generation)() == accepted_gen;
        let def = match loaded {
            Ok(d) if d.kind == kind => d,
            invalid => {
                if snapshot_saved(&c.host.runtime, accepted) && route_current {
                    c.ui.clone().write().error = Some(invalid.err().unwrap_or_else(|| {
                        "The selected component changed before it was applied.".into()
                    }));
                }
                return;
            }
        };
        if !route_current {
            return;
        }
        // The resolver checks the key in the accepted document it actually edits.
        c.ui.clone().write().error = c
            .host
            .runtime
            .commit(
                "layout-key-component",
                key_component_resolver(selected.scope.clone(), selected, def),
            )
            .err();
    }

    async fn prepare_component(
        c: &Ctx,
        accepted: &AcceptedSnapshot,
        owner: PlacementOwner,
        (def_id, kind, workflow, src_def, req_id): (
            String,
            PartKind,
            PlacementWorkflow,
            Option<PartDefinition>,
            u64,
        ),
    ) {
        let res = match src_def {
            Some(d) => Ok(d),
            None => (c.host.load_definition)((*accepted.document).clone(), def_id).await,
        };
        if !c.alive.get()
            || c.ui
                .read()
                .preparing
                .as_ref()
                .is_none_or(|(id, o)| *id != req_id || o != &owner)
        {
            return;
        }
        let mut ui = c.ui;
        let m = c.host.runtime.model();
        if !c.owner_current(
            &owner,
            &m,
            (c.host.generation)(),
            (c.host.workspace)(),
            owner.source_workspace,
            &(c.host.guide_preferences)(),
        ) || m.lifecycle != Lifecycle::Ready
            || m.durability
                != (Durability::Saved {
                    revision: accepted.document.revision,
                })
        {
            ui.write().preparing = None;
            return;
        }
        let def = match res {
            Ok(d) if d.kind == kind => d,
            other => {
                let mut s = ui.write();
                s.preparing = None;
                s.error =
                    Some(other.err().unwrap_or_else(|| {
                        "The selected component changed before placement.".into()
                    }));
                return;
            }
        };
        let Some(pending) = (if workflow.is_controller() {
            controller_part(
                def,
                owner.part_id.clone(),
                owner.reference.clone(),
                owner.at,
            )
        } else {
            Some(component_part(
                def,
                owner.part_id.clone(),
                owner.reference.clone(),
                owner.at,
            ))
        }) else {
            let mut s = ui.write();
            s.preparing = None;
            s.error = Some("The selected catalogue item is not a controller.".into());
            return;
        };
        {
            let mut s = ui.write();
            s.preparing = None;
            s.active = Some(ActivePartPlacement {
                owner,
                snap_document: Rc::new(document_with_pending_definition(
                    &accepted.document,
                    &pending.definition,
                )),
                pending,
            });
        }
        if workflow.is_controller() {
            let mut q = c.host.parts_query;
            q.set("controller".into());
        }
        c.reset_selection(Workspace::Layout, None, vec![]);
    }
}
