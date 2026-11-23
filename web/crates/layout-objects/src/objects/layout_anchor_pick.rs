//! Canvas Anchor pick mode for an anchored matrix or an independent part. The Anchor token arms
//! it; every committable key then draws a violet ring, hovering previews the Core-evaluated
//! post-commit poses in amber, and a click commits one `SetMatrixAnchor` / `SetPartAnchor` Undo
//! entry. Esc, a second token press or a selection change leaves pick mode without a revision.
use super::TreeContext;
use super::layout_rail::{
    GLYPH_ANCHOR, LayoutTransformRuntime, feedback_event, rail_over_points, raise_toast,
    token_face, token_paint, world_per_screen,
};
use super::layout_rail_chrome::{feedback_chrome, feedback_line_width, feedback_panel_contents};
use crate::anchor_pick::{
    AnchorMajorKind, AnchorPickCandidates, CANDIDATE_HIT_PX, CANDIDATE_RING_HOVER_PX,
    CANDIDATE_RING_PX, MAJOR_RING_PX, PickTarget, anchor_is_target, anchor_key_label,
    anchor_pick_candidates, matrix_display_name, pick_commit, preview_anchor_poses,
};
use crate::presentation::canvas_interaction::CanvasInteractionArbiter;
use boardstudio_application::{AcceptedSnapshot, EditResolver, Resolution};
use boardstudio_core_kernel::matrix::layout::{evaluated_part_pose, layout_half_offset};
use boardstudio_model::model::{Matrix, MatrixScene, Pose2, ProjectDoc, Vec2};
use boardstudio_web_ui_model::state::FieldPreviewSource;
use boardstudio_web_ui_model::transform_feedback::{FeedbackDelta, FeedbackEvent, FeedbackState};
use dioxus::prelude::*;
use dioxus_web::WebEventExt;
use std::{cell::RefCell, collections::HashMap, rc::Rc};
use web_sys::SvgElement;

/// Which selection a canvas Anchor pick retargets.
#[derive(Clone, PartialEq)]
pub enum AnchorPickOwner {
    Matrix {
        matrix: Rc<Matrix>,
        projection: Rc<MatrixScene>,
        context: TreeContext,
    },
    Part {
        part_id: String,
    },
}

impl AnchorPickOwner {
    fn context(&self) -> TreeContext {
        match self {
            Self::Matrix { context, .. } => context.clone(),
            Self::Part { part_id } => TreeContext::Component {
                part_id: Some(part_id.clone()),
                matrix_id: None,
                row: None,
                column: None,
                assembly_id: None,
            },
        }
    }
}

/// The owner's Layout id behind the half-origin major target.
fn owner_layout(document: &ProjectDoc, context: &TreeContext) -> Option<String> {
    let layout = match context {
        TreeContext::Matrix { matrix_id } => document
            .layouts
            .iter()
            .find(|layout| layout.matrix_ids.contains(matrix_id)),
        TreeContext::Component {
            part_id: Some(part_id),
            matrix_id: None,
            ..
        } => document
            .layouts
            .iter()
            .find(|layout| layout.part_ids.contains(part_id)),
        _ => None,
    };
    layout.map(|layout| layout.id.clone())
}

fn candidate_pose(scenes: &[MatrixScene], target: &PickTarget) -> Option<Vec2> {
    scenes
        .iter()
        .find(|scene| scene.matrix_id == target.matrix_id)?
        .cells
        .iter()
        .find(|cell| cell.row == target.row && cell.column == target.column)
        .map(|cell| cell.pose.at)
}

fn pick_resolver(context: TreeContext, target: PickTarget) -> EditResolver {
    EditResolver::new("layout-anchor-pick", move |accepted: &AcceptedSnapshot| {
        let Some(commit) = pick_commit(&accepted.document, &context, &target) else {
            return Resolution::Retire("The anchor target is no longer available.".into());
        };
        let owner = match &context {
            TreeContext::Matrix { matrix_id } => matrix_id.clone(),
            TreeContext::Component {
                part_id: Some(part_id),
                ..
            } => part_id.clone(),
            _ => return Resolution::Retire("The selection can no longer anchor.".into()),
        };
        Resolution::submit(vec![owner], commit.operation)
    })
}

/// The armed Anchor popup, pinned to the canvas's top-right corner so it never covers a candidate.
fn pinned_popup(
    rows: Vec<(String, f64, &'static str, bool)>,
    (view_x, view_y, view_width): (f64, f64, f64),
    px: f64,
) -> Element {
    let panel_w = rows
        .iter()
        .map(|(text, size, _, _)| feedback_line_width(text, *size))
        .fold(0.0_f64, f64::max)
        + 16.0;
    let (panel_x, panel_y) = (
        view_x + view_width - panel_w * px - 12.0 * px,
        -view_y - 12.0 * px,
    );
    rsx! {
        g { "data-feedback-popup": "true", class: "fill-foreground", transform: "translate({panel_x} {panel_y}) scale({px} {-px})", style: "pointer-events:none", "aria-hidden": "true",
            {feedback_panel_contents(panel_w, rows)}
        }
    }
}

fn item_label(document: &ProjectDoc, owner: &AnchorPickOwner) -> String {
    match owner {
        AnchorPickOwner::Matrix { matrix, .. } => matrix_display_name(document, &matrix.id),
        AnchorPickOwner::Part { part_id } => document
            .parts
            .iter()
            .find(|part| part.id == *part_id)
            .map(|part| {
                if part.reference.trim().is_empty() {
                    part.id.clone()
                } else {
                    part.reference.clone()
                }
            })
            .unwrap_or_else(|| part_id.clone()),
    }
}

#[component]
pub fn LayoutAnchorPickOverlay(
    runtime: LayoutTransformRuntime,
    surface: Rc<RefCell<Option<SvgElement>>>,
    arbiter: CanvasInteractionArbiter,
    pick_owner: AnchorPickOwner,
    /// How many tokens the rail holds, the Anchor token last.
    rail_slots: usize,
    #[props(default)] surface_size: Option<Signal<(f64, f64)>>,
    view_x: f64,
    view_y: f64,
    width: f64,
    height: f64,
) -> Element {
    let _ = surface_size.map(|size| size());
    let runtime = runtime.0;
    let armed = use_signal(|| false);
    let hovered_target = use_signal(|| None::<PickTarget>);
    let preview = use_signal(Vec::<(String, Pose2)>::new);
    let feedback = use_signal(FeedbackState::default);
    let toast_gen = use_signal(|| 0_u64);
    let context = pick_owner.context();
    let disarm = move || {
        let (mut armed, mut hovered_target, mut preview) = (armed, hovered_target, preview);
        armed.set(false);
        hovered_target.set(None);
        preview.set(Vec::new());
        // peek: the stale-arm effect calls this and must not subscribe to the feedback it writes.
        let mut feedback = feedback;
        let dismissed = boardstudio_web_ui_model::transform_feedback::advance_feedback(
            &feedback.peek(),
            FeedbackEvent::Dismiss,
        );
        feedback.set(dismissed);
    };
    // A stale arm (owner or selection moved on) drops without writing.
    use_effect(use_reactive((&context,), move |_| disarm()));
    use_hook(move || {
        let document = web_sys::window().and_then(|window| window.document());
        let listener =
            wasm_bindgen::closure::Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
                if !event.default_prevented()
                    && event.key() == "Escape"
                    && armed.try_peek().is_ok_and(|armed| *armed)
                {
                    event.prevent_default();
                    disarm();
                }
            }) as Box<dyn FnMut(_)>);
        use wasm_bindgen::JsCast;
        if let Some(document) = &document {
            let _ = document
                .add_event_listener_with_callback("keydown", listener.as_ref().unchecked_ref());
        }
        Rc::new(EscapeGuard { document, listener })
    });

    let Some(accepted) = runtime.model().accepted else {
        return rsx! {};
    };
    let document = accepted.document.clone();
    let scenes: Vec<MatrixScene> = try_use_context::<Option<FieldPreviewSource>>()
        .flatten()
        .map(|source| source.scene.matrix_scenes.clone())
        .unwrap_or_default();
    let Some(projected) = anchor_pick_candidates(&document, &[], &context) else {
        return rsx! {};
    };
    let px = world_per_screen(&surface, width, height);
    let rail_points: Vec<Vec2> = match &pick_owner {
        AnchorPickOwner::Matrix { projection, .. } => {
            let points: Vec<Vec2> = projection
                .cells
                .iter()
                .filter(|cell| cell.enabled)
                .map(|cell| cell.pose.at)
                .collect();
            if points.is_empty() {
                projection
                    .cells
                    .first()
                    .map(|cell| vec![cell.pose.at])
                    .unwrap_or_default()
            } else {
                points
            }
        }
        AnchorPickOwner::Part { part_id } => {
            let Some(part) = document.parts.iter().find(|part| part.id == *part_id) else {
                return rsx! {};
            };
            let top = document
                .definitions
                .iter()
                .find(|d| d.id == part.definition_id)
                .map(|d| {
                    d.courtyard
                        .iter()
                        .map(|p| p.y)
                        .fold(f64::NEG_INFINITY, f64::max)
                })
                .filter(|top| top.is_finite())
                .unwrap_or(0.0);
            let at = evaluated_part_pose(&document, part).at;
            vec![Vec2 {
                x: at.x,
                y: at.y + top,
            }]
        }
    };
    let Some(layout) = rail_over_points(&rail_points, rail_slots, px) else {
        return rsx! {};
    };
    let (token_x, token_y) = layout
        .token_centers
        .last()
        .copied()
        .unwrap_or((layout.rail.x, layout.rail.y));
    let (fill, stroke) = token_paint(armed(), false);
    let current = projected
        .keys
        .iter()
        .find(|key| key.current)
        .map(|key| anchor_key_label(&document, &key.matrix_id, key.row, key.column))
        .unwrap_or_else(|| "half origin".to_owned());
    let item = item_label(&document, &pick_owner);
    let ring_text = |target: &PickTarget| {
        format!(
            "{item} → {}",
            anchor_key_label(&document, &target.matrix_id, target.row, target.column)
        )
    };
    let popup_value = hovered_target().map_or_else(
        || format!("{item} · {current}"),
        |target| ring_text(&target),
    );
    let popup_rows = vec![
        ("Anchor".to_owned(), 11.0, "var(--muted-foreground)", false),
        (popup_value, 15.0, "inherit", true),
        (
            "Click to anchor · Esc: cancel".to_owned(),
            10.5,
            "var(--muted-foreground)",
            false,
        ),
    ];
    let current_poses: HashMap<String, Pose2> = match &pick_owner {
        AnchorPickOwner::Matrix { projection, .. } => projection
            .cells
            .iter()
            .filter_map(|cell| cell.member_id.clone().map(|id| (id, cell.pose)))
            .collect(),
        AnchorPickOwner::Part { part_id } => document
            .parts
            .iter()
            .find(|part| part.id == *part_id)
            .map(|part| (part.id.clone(), evaluated_part_pose(&document, part)))
            .into_iter()
            .collect(),
    };
    let choose = move |value: String| {
        feedback_event(
            feedback,
            FeedbackEvent::Choose {
                name: "Anchor".to_owned(),
                value,
            },
        )
    };
    let hover_ring = {
        let (document, context, open_scratch, current, item) = (
            document.clone(),
            context.clone(),
            runtime.core_services().open_scratch,
            current.clone(),
            item.clone(),
        );
        Rc::new(move |target: Option<PickTarget>| {
            let (mut hovered_target, mut preview) = (hovered_target, preview);
            hovered_target.set(target.clone());
            match target {
                Some(target) => {
                    preview.set(preview_anchor_poses(
                        &document,
                        &context,
                        &target,
                        open_scratch,
                    ));
                    choose(format!(
                        "{item} → {}",
                        anchor_key_label(&document, &target.matrix_id, target.row, target.column)
                    ));
                }
                None => {
                    preview.set(Vec::new());
                    choose(current.clone());
                }
            }
        })
    };
    // Clicking a ring commits one Undo entry and leaves pick mode; a stale pick reports and disarms.
    let commit_pick = {
        let (runtime, arbiter, context) = (runtime.clone(), arbiter.clone(), context.clone());
        Rc::new(move |target: PickTarget| {
            if arbiter.current().is_some() || runtime.model().gesture.is_some() {
                return;
            }
            // The labels name the anchor before the edit lands.
            let before = runtime
                .model()
                .accepted
                .and_then(|accepted| pick_commit(&accepted.document, &context, &target));
            match runtime.commit(
                "layout-anchor-pick",
                pick_resolver(context.clone(), target.clone()),
            ) {
                Ok(snapshot) if anchor_is_target(&snapshot.document, &context, &target) => {
                    if let Some(commit) = before {
                        raise_toast(
                            feedback,
                            toast_gen,
                            format!(
                                "{} anchored to {}",
                                commit.item_label, commit.new_anchor_label
                            ),
                            FeedbackDelta::Text(format!("was {}", commit.old_anchor_label)),
                        );
                    }
                }
                Ok(_) => runtime.report("The anchor target is no longer available."),
                Err(message) => runtime.report(message),
            }
            disarm();
        })
    };
    let toggle = {
        let current = current.clone();
        Rc::new(move || {
            if armed() {
                disarm();
            } else {
                let mut armed = armed;
                armed.set(true);
                choose(current.clone());
            }
        })
    };
    let layout_id = owner_layout(&document, &context);
    let face = token_face(
        fill,
        rsx! { path { d: "{GLYPH_ANCHOR}", style: "fill:none;stroke:{stroke};stroke-width:1.5px;stroke-linecap:round;stroke-linejoin:round;vector-effect:non-scaling-stroke" } },
        Some("Pick anchor"),
    );
    let feedback_state = feedback();
    rsx! {
        g {
            "data-transform-handle": "true", class: "outline-none focus-visible:stroke-foreground",
            transform: "translate({token_x} {token_y}) scale({px} {-px})", role: "button", tabindex: "0", "aria-label": "Pick anchor", "data-pick": "anchor",
            style: "cursor:pointer;touch-action:none",
            onpointerdown: { let toggle = toggle.clone(); move |event: PointerEvent| {
                let Some(pointer) = event.data().try_as_web_event() else { return };
                pointer.prevent_default();
                pointer.stop_propagation();
                toggle();
            } },
            onkeydown: { let toggle = toggle.clone(); move |event: KeyboardEvent| {
                if event.key() == Key::Enter || event.key() == Key::Character(" ".into()) { event.prevent_default(); toggle(); }
            } },
            onpointerenter: { let current = current.clone(); move |_| if !armed() { choose(current.clone()) } },
            onpointerleave: move |_| if !armed() { feedback_event(feedback, FeedbackEvent::Dismiss) },
            onfocus: { let current = current.clone(); move |_| if !armed() { choose(current.clone()) } },
            onblur: move |_| if !armed() { feedback_event(feedback, FeedbackEvent::Dismiss) },
            {face}
        }
        if armed() {
            for (key, target, pose) in projected.keys.iter().filter_map(|key| {
                let target = PickTarget { matrix_id: key.matrix_id.clone(), row: key.row, column: key.column };
                candidate_pose(&scenes, &target).map(|pose| (key, target, pose))
            }) {
                {
                    let hovered = hovered_target().is_some_and(|hover| hover == target);
                    let ring_r = if hovered { CANDIDATE_RING_HOVER_PX } else { CANDIDATE_RING_PX } / 2.0;
                    let label = anchor_key_label(&document, &target.matrix_id, target.row, target.column);
                    let (hover_ring, commit_pick, enter, down, press) = (hover_ring.clone(), commit_pick.clone(), target.clone(), target.clone(), target.clone());
                    rsx! {
                        g { "data-anchor-pick-candidate": "true", class: "outline-none focus-visible:stroke-foreground",
                            transform: "translate({pose.x} {pose.y}) scale({px} {-px})", "data-anchor": "{target.matrix_id}:{target.row}:{target.column}",
                            role: "button", tabindex: "0", "aria-label": "Anchor to {label}",
                            onpointerenter: { let hover = hover_ring.clone(); move |_| hover(Some(enter.clone())) },
                            onpointerleave: move |_| hover_ring(None),
                            onpointerdown: { let commit = commit_pick.clone(); move |event: PointerEvent| {
                                let Some(pointer) = event.data().try_as_web_event() else { return };
                                pointer.prevent_default();
                                pointer.stop_propagation();
                                commit(down.clone());
                            } },
                            onkeydown: move |event: KeyboardEvent| { if event.key() == Key::Enter { event.prevent_default(); commit_pick(press.clone()); } },
                            circle { r: "{CANDIDATE_HIT_PX / 2.0}", style: "fill:transparent;stroke:none;pointer-events:all" }
                            circle { r: "{ring_r}", style: "fill:var(--card);stroke:var(--geometry-part);stroke-width:1.5px;vector-effect:non-scaling-stroke" }
                            if key.current { circle { r: "1.6", style: "fill:#fff;stroke:none;pointer-events:none" } }
                        }
                    }
                }
            }
            for (major, pose, label) in major_rings(&projected, &document, layout_id.as_deref()) {
                {
                    let ring_r = MAJOR_RING_PX / 2.0;
                    rsx! {
                        g { "data-anchor-pick-major": "true", transform: "translate({pose.x} {pose.y}) scale({px} {-px})", opacity: "0.55", style: "pointer-events:none", "aria-hidden": "true",
                            circle { r: "{ring_r}", style: "fill:var(--card);stroke:var(--geometry-part);stroke-width:1.5px;vector-effect:non-scaling-stroke" }
                            path { d: "M{-ring_r - 4.0} 0h{ring_r * 2.0 + 8.0}M0 {-ring_r - 4.0}v{ring_r * 2.0 + 8.0}", style: "fill:none;stroke:var(--geometry-part);stroke-width:1px;vector-effect:non-scaling-stroke" }
                            text { x: "{ring_r + 6.0}", y: "{0.0 - ring_r - 2.0}", style: "font-size:11px;fill:var(--muted-foreground)", "{label}" }
                            if major { circle { r: "2.2", style: "fill:#fff;stroke:none;pointer-events:none" } }
                        }
                    }
                }
            }
            // Amber marks the Core-evaluated post-commit poses that move.
            for (current_pose, pose) in preview().into_iter().filter_map(|(id, pose)| current_poses.get(&id).copied().filter(|current| *current != pose).map(|current| (current, pose))) {
                circle { "data-anchor-pick-preview": "true", cx: "{pose.at.x}", cy: "{pose.at.y}", r: "{CANDIDATE_RING_PX / 2.0 * px}",
                    style: "fill:none;stroke:var(--warning-dark);stroke-width:{1.5 * px};pointer-events:none", "aria-hidden": "true" }
                line { "data-anchor-pick-preview": "true", x1: "{current_pose.at.x}", y1: "{current_pose.at.y}", x2: "{pose.at.x}", y2: "{pose.at.y}",
                    style: "stroke:var(--warning-dark);stroke-width:{1.0 * px};stroke-dasharray:{3.0 * px} {2.0 * px};pointer-events:none", "aria-hidden": "true" }
            }
            {pinned_popup(popup_rows, (view_x, view_y, width), px)}
        }
        {feedback_chrome(&feedback_state, !armed(), None, Some(&layout), px, (view_x, view_y, width))}
    }
}

/// Removes the document Escape listener with the overlay.
struct EscapeGuard {
    document: Option<web_sys::Document>,
    listener: wasm_bindgen::closure::Closure<dyn FnMut(web_sys::KeyboardEvent)>,
}

impl Drop for EscapeGuard {
    fn drop(&mut self) {
        use wasm_bindgen::JsCast;
        if let Some(document) = &self.document {
            let _ = document.remove_event_listener_with_callback(
                "keydown",
                self.listener.as_ref().unchecked_ref(),
            );
        }
    }
}

/// The display-only major targets (parts and the half origin): dimmed rings that never commit.
fn major_rings(
    projected: &AnchorPickCandidates,
    document: &ProjectDoc,
    layout_id: Option<&str>,
) -> Vec<(bool, Vec2, String)> {
    projected
        .majors
        .iter()
        .filter_map(|major| {
            let (pose, label) = match &major.kind {
                AnchorMajorKind::Part(id) => {
                    let part = document.parts.iter().find(|part| part.id == *id)?;
                    (
                        evaluated_part_pose(document, part).at,
                        if part.reference.trim().is_empty() {
                            part.id.clone()
                        } else {
                            part.reference.clone()
                        },
                    )
                }
                // An unsplit layout has no half offset: its half frame sits at the board origin.
                AnchorMajorKind::HalfOrigin => (
                    layout_half_offset(document, layout_id?).unwrap_or(Vec2 { x: 0.0, y: 0.0 }),
                    "Half origin".to_owned(),
                ),
            };
            Some((major.current, pose, label))
        })
        .collect()
}
