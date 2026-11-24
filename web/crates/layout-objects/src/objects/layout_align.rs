//! Layout Align toolbar controls and synchronous alignment execution.
use super::layout_align_geometry::reference_choice;
pub use super::layout_align_geometry::{
    AlignCommand, alignment_delta, reconcile_reference_choice, transformed_envelope,
};
use super::layout_align_logic::{make_edit, workspace_route_matches};
use super::layout_toolbar::{LayoutCommandMenu, close_layout_command_menu};
use super::mirror_links::{layout_for_matrix, partner_layout};
use super::{LayoutWorkspaceState, ScopedTreeContext, TreeContext};
use crate::runtime::Runtime;
use boardstudio_application::{
    AcceptedSnapshot, Durability, EditResolver, Lifecycle, Resolution, Scope,
};
use boardstudio_model::model::{MatrixScene, Part, ProjectDoc, Vec2};
use boardstudio_web_layout_core::matrix_by_id;
use boardstudio_web_ui_model::Workspace;
use boardstudio_web_ui_shared::ui::{
    button::{Button, ButtonSize, ButtonVariant},
    callout::Callout,
    combobox::{Combobox, ComboboxOption},
    label::Label,
    select::{Select, SelectOption},
    toast::{ToastOptions, use_toast},
};
use dioxus::prelude::*;
use std::{collections::BTreeSet, rc::Rc};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlignReference {
    pub id: String,
    pub label: String,
}

#[derive(Clone, PartialEq)]
pub struct LayoutAlignMount {
    pub references: Vec<AlignReference>,
    pub selected_reference: Option<String>,
    pub enabled: bool,
    pub disabled_reason: Option<String>,
    pub on_reference: EventHandler<String>,
    pub on_align: EventHandler<AlignCommand>,
}

#[derive(Clone, PartialEq, Default)]
struct AlignProjection {
    references: Vec<AlignReference>,
    selected_reference: Option<String>,
    enabled: bool,
    disabled_reason: Option<String>,
    moving_ids: Vec<String>,
    reference_authoritative: bool,
}

/// May run inside the owning workspace: the Layout reference choice persists on
/// `LayoutWorkspaceState` (session lifetime) across workspace switches, while any
/// other owner keeps a mount-local choice.
pub fn use_canvas_align(
    runtime: Rc<Runtime>,
    version: Signal<u64>,
    selected_context: Signal<Option<ScopedTreeContext>>,
    workspace: Signal<Workspace>,
    scope_generation: Signal<u64>,
    owner_workspace: Workspace,
) -> LayoutAlignMount {
    let local_reference = use_signal(|| None::<String>);
    let persisted = try_use_context::<LayoutWorkspaceState>().map(|s| s.selected_reference());
    let mut selected_reference = match (owner_workspace, persisted) {
        (Workspace::Layout, Some(persisted)) => persisted,
        _ => local_reference,
    };
    let toast = use_toast();
    let _ = (version(), scope_generation());
    let selected = selected_context.read().clone();
    let cur_ref = selected_reference.read().clone();
    let projection = project(
        &runtime,
        selected.as_ref(),
        workspace(),
        cur_ref.as_deref(),
        owner_workspace,
    );

    let eligible_ids: Vec<_> = projection.references.iter().map(|r| r.id.clone()).collect();
    let authoritative = projection.reference_authoritative;
    // Reconcile after render using peek(); non-authoritative projections (owner
    // workspace hidden) never overwrite the persisted reference choice.
    use_effect(use_reactive(
        (&eligible_ids, &authoritative),
        move |(eligible, auth)| {
            let cur = selected_reference.peek().clone();
            reconcile_reference_choice(cur.as_deref(), &eligible, auth, |n| {
                selected_reference.set(n)
            });
        },
    ));

    let on_reference = use_callback({
        let runtime = runtime.clone();
        move |reference_id: String| {
            let cur = project(
                &runtime,
                selected_context.peek().as_ref(),
                workspace(),
                None,
                owner_workspace,
            );
            if cur.references.iter().any(|r| r.id == reference_id) {
                selected_reference.set(Some(reference_id));
            }
        }
    });

    let on_align = use_callback({
        let runtime = runtime.clone();
        move |command: AlignCommand| {
            let ws = workspace();
            if !workspace_route_matches(ws, owner_workspace) {
                return;
            }
            let Some(sel) = selected_context.peek().clone() else {
                return;
            };
            let cur = project(
                &runtime,
                Some(&sel),
                ws,
                selected_reference.peek().as_deref(),
                owner_workspace,
            );
            let Some(ref_id) = cur
                .selected_reference
                .filter(|_| cur.enabled && !cur.moving_ids.is_empty())
            else {
                return;
            };
            let res = runtime.commit(
                "layout-align",
                align_resolver(sel.context, cur.moving_ids, ref_id.clone(), command),
            );
            let label = runtime
                .model()
                .accepted
                .as_ref()
                .and_then(|s| s.document.parts.iter().find(|p| p.id == ref_id))
                .map_or(ref_id, |p| p.reference.clone());
            match res {
                Ok(_) => toast.success(
                    format!("Aligned selection to {label}. Reference unchanged."),
                    ToastOptions::new(),
                ),
                Err(msg) => toast.error(msg, ToastOptions::new()),
            }
        }
    });

    LayoutAlignMount {
        references: projection.references,
        selected_reference: projection.selected_reference,
        enabled: projection.enabled,
        disabled_reason: projection.disabled_reason,
        on_reference,
        on_align,
    }
}

#[component]
pub fn LayoutAlignToolbar(
    mount: LayoutAlignMount,
    open_menu: Signal<Option<LayoutCommandMenu>>,
    show_relationships: bool,
    has_selection_context: bool,
    on_show_relationships: EventHandler<()>,
) -> Element {
    let selected_reference = use_memo(use_reactive((&mount.selected_reference,), |(v,)| v));
    let on_ref = move |v: Option<String>| {
        if let Some(v) = v {
            mount.on_reference.call(v);
        }
    };
    rsx! {
        super::layout_toolbar::LayoutCommandPopup {
            open_menu, menu: LayoutCommandMenu::Align, label: "Align", trigger: rsx! { span { "Align" } },
            Label { "Reference part" }
            if mount.references.len() > 12 {
                Combobox::<String> {
                    aria_label: "Alignment reference", list_aria_label: "Alignment reference",
                    value: Some(selected_reference.into()), on_value_change: on_ref,
                    for (index, part) in mount.references.iter().enumerate() {
                        ComboboxOption::<String> { index, value: part.id.clone(), text_value: part.label.clone(), "{part.label}" }
                    }
                }
            } else {
                Select::<String> {
                    aria_label: "Alignment reference", value: Some(selected_reference.into()),
                    disabled: mount.references.is_empty(), on_value_change: on_ref,
                    for (index, part) in mount.references.iter().enumerate() {
                        SelectOption::<String> { index, value: part.id.clone(), text_value: part.label.clone(), "{part.label}" }
                    }
                }
            }
            if let Some(reason) = mount.disabled_reason.as_ref() { Callout { "{reason}" } }
            div { class: "layout-alignment-actions", role: "group", aria_label: "Align selection",
                for command in AlignCommand::ALL {
                    Button {
                        key: "{command.label()}", size: ButtonSize::Sm, variant: ButtonVariant::Secondary,
                        disabled: !mount.enabled || mount.selected_reference.is_none(),
                        onclick: move |_| { mount.on_align.call(command); close_layout_command_menu(open_menu, LayoutCommandMenu::Align); },
                        "{command.label()}"
                    }
                }
            }
            if show_relationships {
                Button {
                    size: ButtonSize::Sm, variant: ButtonVariant::Secondary, disabled: !has_selection_context,
                    onclick: move |_| { on_show_relationships.call(()); close_layout_command_menu(open_menu, LayoutCommandMenu::Align); },
                    "Relationships"
                }
            }
        }
    }
}

fn project(
    runtime: &Runtime,
    selected: Option<&ScopedTreeContext>,
    workspace: Workspace,
    current_reference: Option<&str>,
    owner_workspace: Workspace,
) -> AlignProjection {
    if workspace != owner_workspace {
        return AlignProjection::default();
    }
    let model = runtime.model();
    let (Some(selected), Some(scope), Some(snapshot)) =
        (selected, runtime.scope(), model.accepted.as_ref())
    else {
        return AlignProjection::default();
    };
    if scope != selected.scope
        || snapshot.session_epoch != scope.session_epoch
        || snapshot.document.id != scope.document_id
        || model.active_board_id != scope.board_id
        || model.active_instance_id != scope.instance_id
    {
        return AlignProjection::default();
    }
    let (Some(moving_ids), Some(board)) = (
        super::resolve_selection(&model, &selected.context),
        snapshot
            .document
            .boards
            .iter()
            .find(|b| b.id == scope.board_id),
    ) else {
        return AlignProjection::default();
    };
    let moving_set: BTreeSet<_> = moving_ids.iter().cloned().collect();
    let selected_set: BTreeSet<_> = model.selected_part_ids.iter().cloned().collect();
    if selected_set != moving_set || moving_ids.is_empty() {
        return AlignProjection::default();
    }
    let doc = &snapshot.document;
    let visible: Vec<&Part> = doc
        .parts
        .iter()
        .filter(|p| board.part_ids.contains(&p.id))
        .collect();
    let linked = linked_partner_parts(
        doc,
        &snapshot.scene.matrix_scenes,
        &scope,
        &selected.context,
        &moving_ids,
    );
    let references: Vec<_> = visible
        .iter()
        .filter(|p| !moving_set.contains(&p.id) && !linked.contains(&p.id))
        .map(|p| AlignReference {
            id: p.id.clone(),
            label: p.reference.clone(),
        })
        .collect();
    let eligible_ids: Vec<_> = references.iter().map(|r| r.id.clone()).collect();
    let selected_reference = reference_choice(current_reference, &eligible_ids, true);
    let moving: Vec<_> = moving_ids
        .iter()
        .filter_map(|id| visible.iter().find(|p| p.id == *id).copied())
        .collect();
    if moving.len() != moving_ids.len() {
        return AlignProjection {
            references,
            selected_reference,
            moving_ids,
            reference_authoritative: true,
            ..Default::default()
        };
    }
    let locked = is_locked_or_driven(doc, &moving);
    let row = matches!(selected.context, TreeContext::Row { .. });
    let scoped = matches!(
        selected.context,
        TreeContext::Matrix { .. }
            | TreeContext::Column { .. }
            | TreeContext::Key { .. }
            | TreeContext::Component { .. }
    );
    let unsupported = validate_envelopes(doc, &moving, selected_reference.as_deref()).err();
    let active = model.lifecycle == Lifecycle::Ready
        && matches!(model.durability, Durability::Saved { revision } | Durability::Saving { revision } if revision == doc.revision)
        && model.display_preview.is_none()
        && model.gesture.is_none();
    let disabled_reason = if locked {
        Some("Unlock or remove the driving constraint before aligning.".into())
    } else if row {
        Some(
            "A row can cross differently splayed columns. Align individual keys or a column."
                .into(),
        )
    } else {
        unsupported.clone()
    };
    AlignProjection {
        references,
        selected_reference,
        enabled: active && scoped && !locked && unsupported.is_none(),
        disabled_reason,
        moving_ids,
        reference_authoritative: true,
    }
}

fn is_locked_or_driven(doc: &ProjectDoc, parts: &[&Part]) -> bool {
    parts
        .iter()
        .any(|p| p.locked == Some(true) || doc.constraint_driving_part(&p.id) != Ok(None))
}

fn linked_partner_parts(
    doc: &ProjectDoc,
    scenes: &[MatrixScene],
    scope: &Scope,
    context: &TreeContext,
    moving_ids: &[String],
) -> BTreeSet<String> {
    super::matrix_id_for_context(context)
        .map(str::to_owned)
        .or_else(|| {
            moving_ids
                .iter()
                .find_map(|id| doc.matrices.iter().find(|m| m.part_ids.contains(id)))
                .map(|m| m.id.clone())
        })
        .and_then(|mid| {
            let layout = layout_for_matrix(&doc.layouts, &scope.board_id, &mid)?;
            let partner =
                partner_layout(&doc.layouts, layout).filter(|p| p.board_id == scope.board_id)?;
            let pid = layout
                .mirror_link
                .as_ref()
                .or(partner.mirror_link.as_ref())?
                .partner_matrix(&mid)?;
            linked_selection(context, moving_ids, doc, scenes, &scope.board_id)
                .then(|| matrix_by_id(doc, pid))
                .flatten()
        })
        .map(|m| m.part_ids.iter().cloned().collect())
        .unwrap_or_default()
}

type AlignEnvelopes = (Vec<Vec2>, Vec<Vec<Vec2>>);

fn validate_envelopes(
    doc: &ProjectDoc,
    moving: &[&Part],
    reference_id: Option<&str>,
) -> Result<Option<AlignEnvelopes>, String> {
    let env = |p: &Part, role: &str| {
        doc.definitions
            .iter()
            .find(|d| d.id == p.definition_id)
            .and_then(|d| transformed_envelope(p, d).ok())
            .ok_or_else(|| {
                format!(
                    "Unsupported alignment {role}: {} has no supported envelope.",
                    p.reference
                )
            })
    };
    let mov_envs = moving
        .iter()
        .map(|p| env(p, "target"))
        .collect::<Result<Vec<_>, _>>()?;
    let Some(ref_id) = reference_id else {
        return Ok(None);
    };
    let ref_part = doc
        .parts
        .iter()
        .find(|p| p.id == ref_id)
        .ok_or_else(|| "The chosen alignment reference is no longer available.".to_owned())?;
    Ok(Some((env(ref_part, "reference")?, mov_envs)))
}

fn linked_selection(
    context: &TreeContext,
    moving_ids: &[String],
    doc: &ProjectDoc,
    scenes: &[MatrixScene],
    board_id: &str,
) -> bool {
    let TreeContext::Component {
        assembly_id,
        matrix_id,
        part_id,
        ..
    } = context
    else {
        return !moving_ids.is_empty();
    };
    let (None, Some(pid)) = (assembly_id, part_id.as_deref()) else {
        return false;
    };
    let Some(board) = doc.boards.iter().find(|b| b.id == board_id) else {
        return false;
    };
    board.part_ids.iter().any(|id| id == pid)
        && doc.matrices.iter().any(|m| {
            matrix_id.as_deref().is_none_or(|id| id == m.id)
                && m.part_ids.iter().any(|id| id == pid)
                && m.part_ids.iter().any(|id| board.part_ids.contains(id))
                && scenes.iter().any(|s| {
                    s.matrix_id == m.id
                        && s.cells
                            .iter()
                            .any(|c| c.enabled && c.member_id.as_deref() == Some(pid))
                })
        })
}

fn align_resolver(
    context: TreeContext,
    moving_ids: Vec<String>,
    reference_id: String,
    command: AlignCommand,
) -> EditResolver {
    EditResolver::new("layout-align", move |accepted: &AcceptedSnapshot| {
        let doc = &accepted.document;
        if doc.parts.iter().all(|p| p.id != reference_id) {
            return Resolution::Retire("The alignment reference no longer exists.".into());
        }
        let moving: Vec<&Part> = moving_ids
            .iter()
            .filter_map(|id| doc.parts.iter().find(|p| p.id == *id))
            .collect();
        if moving.len() != moving_ids.len() {
            return Resolution::Retire("A selected part no longer exists.".into());
        }
        if is_locked_or_driven(doc, &moving) {
            return Resolution::Retire(
                "A selected part is locked or driven by a relationship.".into(),
            );
        }
        let (ref_env, mov_envs) = match validate_envelopes(doc, &moving, Some(&reference_id)) {
            Ok(Some(envs)) => envs,
            Ok(None) => return Resolution::Unchanged,
            Err(reason) => return Resolution::Retire(reason),
        };
        match alignment_delta(&mov_envs, &ref_env, command) {
            Err(reason) => Resolution::Retire(reason),
            Ok(delta) if delta == Vec2::default() => Resolution::Unchanged,
            Ok(delta) => match make_edit(doc, &context, &moving_ids, delta) {
                Some((op, targets)) => Resolution::submit(targets, op),
                None => Resolution::Retire("The aligned selection no longer exists.".into()),
            },
        }
    })
}
