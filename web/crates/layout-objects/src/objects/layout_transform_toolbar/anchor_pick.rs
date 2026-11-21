//! Ticket 08: canvas Anchor pick mode for an anchored matrix or an independent part.
use super::*;

/// Ticket 08: which selection a canvas Anchor pick retargets: an anchored
/// matrix (with its scene for the rail centre) or an independent part.
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
    pub(super) fn context(&self) -> TreeContext {
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

    /// The rail the Anchor token joins: last in canonical order on either rail.
    pub(super) fn rail(&self) -> Rail {
        match self {
            Self::Matrix { .. } => Rail::Matrix,
            Self::Part { .. } => Rail::Part,
        }
    }
}

/// Ticket 08: the owner's Layout id behind the half-origin major target.
pub(super) fn anchor_pick_layout(document: &ProjectDoc, context: &TreeContext) -> Option<String> {
    match context {
        TreeContext::Matrix { matrix_id } => document
            .layouts
            .iter()
            .find(|layout| layout.matrix_ids.iter().any(|id| id == matrix_id))
            .map(|layout| layout.id.clone()),
        TreeContext::Component {
            part_id: Some(part_id),
            matrix_id: None,
            ..
        } => document
            .layouts
            .iter()
            .find(|layout| layout.part_ids.iter().any(|id| id == part_id))
            .map(|layout| layout.id.clone()),
        _ => None,
    }
}

/// Ticket 08: one candidate key's canvas pose from the evaluated scenes.
pub(super) fn anchor_candidate_pose(scenes: &[MatrixScene], target: &PickTarget) -> Option<Vec2> {
    scenes
        .iter()
        .find(|scene| scene.matrix_id == target.matrix_id)
        .and_then(|scene| {
            scene
                .cells
                .iter()
                .find(|cell| cell.row == target.row && cell.column == target.column)
        })
        .map(|cell| cell.pose.at)
}

/// Ticket 08: the current anchor in words for the armed popup: the live key,
/// or the half origin while detached.
pub(super) fn anchor_pick_current_label(
    document: &ProjectDoc,
    projected: &AnchorPickCandidates,
) -> String {
    projected
        .keys
        .iter()
        .find(|key| key.current)
        .map(|key| anchor_key_label(document, &key.matrix_id, key.row, key.column))
        .unwrap_or_else(|| "half origin".to_owned())
}

/// Ticket 08: resolve one pick against the accepted document: owner and
/// target both live, so Core's refusal stays for genuinely driven keys.
pub(super) fn anchor_pick_resolver(context: TreeContext, target: PickTarget) -> EditResolver {
    EditResolver::new(
        "layout-anchor-pick",
        move |accepted: &AcceptedSnapshot| match pick_commit(&accepted.document, &context, &target)
        {
            Some(commit) => {
                let owner_id = match &context {
                    TreeContext::Matrix { matrix_id } => matrix_id.clone(),
                    TreeContext::Component {
                        part_id: Some(part_id),
                        ..
                    } => part_id.clone(),
                    _ => {
                        return Resolution::Retire("The selection can no longer anchor.".into());
                    }
                };
                Resolution::submit(vec![owner_id], commit.operation)
            }
            None => Resolution::Retire("The anchor target is no longer available.".into()),
        },
    )
}

/// Ticket 08: whether the owner's live anchor names the target after the
/// settle loop.
pub(super) fn anchor_pick_landed(
    document: &ProjectDoc,
    context: &TreeContext,
    target: &PickTarget,
) -> bool {
    crate::anchor_pick::anchor_is_target(document, context, target)
}

/// Ticket 08: the canvas Anchor pick mode for one anchored matrix or
/// independent part. The Anchor token toggles the armed state; while armed
/// every committable key draws a violet ring (loop-forming targets draw no
/// ring), hovering previews the Core-evaluated post-commit pose in amber,
/// and clicking commits one `SetMatrixAnchor` / `SetPartAnchor` Undo entry
/// with the carried toast. Esc, a second token press or a selection change
/// leaves pick mode with no revision, and the selection and focus stay put.
#[component]
pub fn LayoutAnchorPickOverlay(
    runtime: LayoutTransformRuntime,
    svg: LayoutTransformSvg,
    arbiter: CanvasInteractionArbiter,
    owner: LayoutOwnerIdentity,
    selected_context: Signal<Option<ScopedTreeContext>>,
    scope_generation: Signal<u64>,
    workspace: Signal<&'static str>,
    #[props(default)] surface_size: Option<Signal<(f64, f64)>>,
    pick_owner: AnchorPickOwner,
    view_x: f64,
    view_y: f64,
    width: f64,
    height: f64,
) -> Element {
    let _ = surface_size.map(|size| size());
    // W3-C: canvas-owned rail sets; siblings share the handle, a second canvas owns its own.
    let svg_handle = svg.clone();
    let rail = pick_owner.rail();
    use_rail_mount(rail, svg_handle.clone(), LayoutTransformTool::Anchor);
    let armed = use_signal(|| false);
    let hovered_target = use_signal(|| None::<PickTarget>);
    let preview = use_signal(Vec::<(String, Pose2)>::new);
    // Ticket 06, reused: the value popup and commit toast content (pure
    // reducer; rendering only) plus the toast generation the 1.8 s timer
    // checks. While armed the popup renders pinned to the canvas's top-right
    // corner; otherwise it docks above the rail like every other token.
    let feedback = use_signal(FeedbackState::default);
    let toast_gen = use_signal(|| 0_u64);
    // Production `version` drives settlement (workbench root provides it;
    // browser test hosts provide it the same way). No local fallback: without
    // a publisher settlement would silently never run.
    let version = transform_version();
    // Single-ticket Anchor commit: one waiting entry (ticket plus the toast
    // its landing needs) settles on `version`. No keyed map or polling.
    // `Landed` raises the carried toast only if the live anchor names the
    // target; `Failed` reports (save failures included); `Retired` is silent.
    // Dropping the entry releases its observers without cancelling the edit.
    #[derive(Clone)]
    struct AnchorPickFollowUp {
        owner: LayoutOwnerIdentity,
        context: TreeContext,
        target: PickTarget,
        item_label: String,
        old_anchor_label: String,
        new_anchor_label: String,
    }
    let waiting = use_signal(|| None::<(EditTicket, AnchorPickFollowUp)>);
    use_effect(use_reactive((&version(),), {
        let runtime = runtime.0.clone();
        let mut waiting = waiting;
        move |_| {
            let Some((ticket, follow_up)) = waiting.read().clone() else {
                return;
            };
            let live = transform_commit_is_live(
                &runtime,
                workspace,
                scope_generation,
                selected_context,
                &follow_up.owner,
                &follow_up.context,
            );
            match ticket.settlement(live) {
                Settlement::Pending => {}
                Settlement::Landed { .. } => {
                    waiting.set(None);
                    let landed = runtime.model().accepted.as_ref().is_some_and(|snapshot| {
                        anchor_pick_landed(
                            &snapshot.document,
                            &follow_up.context,
                            &follow_up.target,
                        )
                    });
                    if !landed {
                        runtime.report("The anchor target is no longer available.");
                        return;
                    }
                    raise_commit_toast(
                        feedback,
                        toast_gen,
                        format!(
                            "{} anchored to {}",
                            follow_up.item_label, follow_up.new_anchor_label
                        ),
                        FeedbackDelta::Text(format!("was {}", follow_up.old_anchor_label)),
                    );
                }
                Settlement::Failed { message } => {
                    waiting.set(None);
                    runtime.report(message);
                }
                Settlement::Retired => {
                    waiting.set(None);
                }
            }
        }
    }));

    // Leaving pick mode clears the armed peer, the hover preview and the
    // popup. The selection and focus stay put: nothing here writes them.
    let svg_for_disarm = svg_handle.clone();
    let disarm = {
        let svg_for_disarm = svg_for_disarm.clone();
        Rc::new(move || {
            let mut armed = armed;
            let mut hovered_target = hovered_target;
            let mut preview = preview;
            let mut feedback = feedback;
            armed.set(false);
            hovered_target.set(None);
            preview.set(Vec::new());
            // peek: the stale-arm effect calls this, and subscribing it to
            // the feedback it writes would re-run it forever.
            let dismissed = advance_feedback(&feedback.peek(), FeedbackEvent::Dismiss);
            feedback.set(dismissed);
            set_rail_active(rail, &svg_for_disarm, None);
        })
    };
    use_canvas_escape(&svg_handle, {
        let disarm = disarm.clone();
        move |_| {
            let claims = armed();
            if claims {
                disarm();
            }
            claims
        }
    });
    // A stale arm (owner, selection or scope moved on) drops without writing.
    {
        let disarm = disarm.clone();
        let pick_owner = pick_owner.clone();
        let owner_for_effect = owner.clone();
        use_effect(move || {
            let _ = selected_context();
            let _ = scope_generation();
            let _ = pick_owner;
            let _ = owner_for_effect;
            disarm();
        });
    }

    let model = runtime.0.model();
    let accepted = model.accepted.clone();
    let context = pick_owner.context();
    let document = accepted.as_ref().map(|snapshot| snapshot.document.clone());
    let scenes: Vec<MatrixScene> = try_use_context::<Option<FieldPreviewSource>>()
        .flatten()
        .map(|source| source.scene.matrix_scenes.clone())
        .unwrap_or_default();
    let projected: Option<AnchorPickCandidates> = document
        .as_ref()
        .and_then(|document| anchor_pick_candidates(document, &[], &context));
    let Some(document) = document else {
        return rsx! {};
    };
    let Some(projected) = projected else {
        return rsx! {};
    };
    // World units per screen px: rings and labels stay screen-size at every
    // zoom, matching the badge and rail chrome.
    let world_per_screen = handle_scale(&svg, width, height) / 3.0;
    let px = world_per_screen;
    // The rail peers share one mount set per canvas; the Anchor token sits
    // last in canonical order. The matrix rail drops Spread on column 0,
    // like the drag overlay, so the token stays centred on what draws.
    let rail_tools: Vec<LayoutTransformTool> = match &pick_owner {
        AnchorPickOwner::Matrix { .. } => matrix_rail_tools(&svg_handle, &context),
        AnchorPickOwner::Part { .. } => rail_peers(Rail::Part, &svg_handle),
    };
    let rail_index = rail_tools
        .iter()
        .position(|peer| *peer == LayoutTransformTool::Anchor);
    let rail_points: Vec<Vec2> = match &pick_owner {
        AnchorPickOwner::Matrix { projection, .. } => {
            let mut points: Vec<Vec2> = projection
                .cells
                .iter()
                .filter(|cell| cell.enabled)
                .map(|cell| cell.pose.at)
                .collect();
            if points.is_empty() {
                points = projection
                    .cells
                    .first()
                    .map(|cell| vec![cell.pose.at])
                    .unwrap_or_default();
            }
            points
        }
        AnchorPickOwner::Part { part_id } => {
            let Some(part) = document.parts.iter().find(|part| part.id == *part_id) else {
                return rsx! {};
            };
            let evaluated = evaluated_part_pose(&document, part);
            let courtyard_top = part_courtyard_top(&document, &part.definition_id);
            vec![Vec2 {
                x: evaluated.at.x,
                y: evaluated.at.y + courtyard_top,
            }]
        }
    };
    let Some(rail_layout) =
        rail_index.and_then(|_| rail_over_points(&rail_points, rail_tools.len(), world_per_screen))
    else {
        return rsx! {};
    };
    let (tok_x, tok_y) = rail_layout
        .token_centers
        .get(rail_index.expect("the Anchor token draws"))
        .copied()
        .unwrap_or((rail_layout.rail.x, rail_layout.rail.y));
    // The Anchor token never lights on hover: hovering shows the popup only.
    let (token_fill, glyph_stroke) = token_paint(armed(), false);
    let current_label = anchor_pick_current_label(&document, &projected);
    let item_label: String = match &pick_owner {
        AnchorPickOwner::Matrix { matrix, .. } => matrix_display_name(&document, &matrix.id),
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
    };
    // The pinned popup rows: the item against the current or hovered target.
    // Width and pin recompute with the value, so the panel hugs the corner.
    let popup_value = match hovered_target() {
        Some(target) => format!(
            "{item_label} → {}",
            anchor_key_label(&document, &target.matrix_id, target.row, target.column)
        ),
        None => format!("{item_label} · {current_label}"),
    };
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
    // The pinned panel measures its own rows; see anchor_pick_popup_panel.
    // Arming toggles on the token; the peers dim while one is armed, like a
    // drag. Nothing writes the selection or focus.
    let svg_handle_for_toggle = svg_handle.clone();
    let toggle_arm = {
        let disarm = disarm.clone();
        let current_label = current_label.clone();
        Rc::new(move |_| {
            let mut armed = armed;
            let mut feedback = feedback;
            if armed() {
                disarm();
                return;
            }
            armed.set(true);
            set_rail_active(
                rail,
                &svg_handle_for_toggle,
                Some(LayoutTransformTool::Anchor),
            );
            feedback.set(advance_feedback(
                &feedback(),
                FeedbackEvent::Choose {
                    name: "Anchor".to_owned(),
                    value: current_label.clone(),
                },
            ));
        })
    };
    // Hovering a ring previews the Core-evaluated post-commit pose in amber;
    // leaving restores the current-target popup.
    let hover_ring = {
        let document = document.clone();
        let context = context.clone();
        let open_scratch = runtime.0.core_services().open_scratch;
        let current_label = current_label.clone();
        let item_label = item_label.clone();
        Rc::new(move |target: Option<PickTarget>| {
            let mut hovered_target = hovered_target;
            let mut preview = preview;
            let mut feedback = feedback;
            hovered_target.set(target.clone());
            match target {
                Some(target) => {
                    preview.set(preview_anchor_poses(
                        &document,
                        &context,
                        &target,
                        open_scratch,
                    ));
                    feedback.set(advance_feedback(
                        &feedback(),
                        FeedbackEvent::Choose {
                            name: "Anchor".to_owned(),
                            value: format!(
                                "{item_label} → {}",
                                anchor_key_label(
                                    &document,
                                    &target.matrix_id,
                                    target.row,
                                    target.column
                                )
                            ),
                        },
                    ));
                }
                None => {
                    preview.set(Vec::new());
                    feedback.set(advance_feedback(
                        &feedback(),
                        FeedbackEvent::Choose {
                            name: "Anchor".to_owned(),
                            value: current_label.clone(),
                        },
                    ));
                }
            }
        })
    };
    // Clicking a ring commits one Undo entry through the shared resolver and
    // raises the carried toast only once the live anchor names the target.
    // The selection and focus stay put; a stale pick reports and disarms.
    let commit_pick = {
        let runtime_for_commit = runtime.0.clone();
        let waiting_for_commit = waiting;
        let disarm = disarm.clone();
        let context = context.clone();
        let owner_for_commit = owner.clone();
        let arbiter_for_commit = arbiter.clone();
        Rc::new(move |target: PickTarget| {
            let mut waiting = waiting_for_commit;
            if arbiter_for_commit.current().is_some()
                || runtime_for_commit.model().gesture.is_some()
            {
                return;
            }
            if !transform_owner_is_current(
                &runtime_for_commit,
                workspace,
                scope_generation,
                selected_context,
                &owner_for_commit,
                &context,
            ) {
                runtime_for_commit.report("The selection changed. Pick the anchor again.");
                disarm();
                return;
            }
            let Some(accepted) = runtime_for_commit.model().accepted.clone() else {
                return;
            };
            let Some(commit) = pick_commit(&accepted.document, &context, &target) else {
                return;
            };
            // Queue one Anchor commit as intent; the `version` effect settles
            // it (durable toast on landing, `report` on failure). Queued
            // picks compose through the resolver; unmounting releases the
            // observer without cancelling the edit.
            if waiting.read().is_some() {
                return;
            }
            let ticket = begin_transform_ticket(
                &runtime_for_commit,
                "layout-anchor-pick",
                anchor_pick_resolver(context.clone(), target.clone()),
            );
            waiting.set(Some((
                ticket,
                AnchorPickFollowUp {
                    owner: owner_for_commit.clone(),
                    context: context.clone(),
                    target: target.clone(),
                    item_label: commit.item_label.clone(),
                    old_anchor_label: commit.old_anchor_label.clone(),
                    new_anchor_label: commit.new_anchor_label.clone(),
                },
            )));
            // Leave pick mode at the click: the rings go, and a second click
            // cannot queue a second commit while this one settles.
            disarm();
        })
    };
    // The live poses behind the amber comparison: the owner's own scene
    // cells for a matrix, the evaluated part for a part.
    let current_poses: std::collections::HashMap<String, Pose2> = match &pick_owner {
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
    let active_peer = rail_active(rail, &svg_handle);
    let token_opacity = if active_peer.is_some_and(|peer| peer != LayoutTransformTool::Anchor) {
        "0.35"
    } else {
        "1"
    };
    let token_style = format!("cursor:pointer;opacity:{token_opacity};touch-action:none");
    let glyph = rail_token_glyph(LayoutTransformTool::Anchor, false, 0.0, glyph_stroke);
    let face = token_face(token_fill, glyph, Some("Pick anchor"));
    let feedback_state = feedback();
    let layout_id = anchor_pick_layout(&document, &context);
    rsx! {
        if rail_index == Some(0) {
            {rail_backdrop(&rail_layout, rail_tools.len(), px)}
        }
        g {
            "data-transform-handle": "true", class: "outline-none focus-visible:stroke-foreground",
            transform: "translate({tok_x} {tok_y}) scale({px} {-px})",
            role: "button", tabindex: "0", "aria-label": "Pick anchor",
            "data-pick": "anchor",
            style: "{token_style}",
            onpointerdown: { let toggle = toggle_arm.clone(); move |event| {
                let Some(pointer) = event.data().try_as_web_event() else { return; };
                pointer.prevent_default();
                pointer.stop_propagation();
                toggle(());
            } },
            onkeydown: { let toggle = toggle_arm.clone(); move |event: KeyboardEvent| {
                let Some(key) = event.data().try_as_web_event() else { return; };
                if key.key() != "Enter" && key.key() != " " { return; }
                key.prevent_default();
                toggle(());
            } },
            onpointerenter: { let mut feedback = feedback; let value = current_label.clone(); move |_| {
                if armed() { return; }
                feedback.set(advance_feedback(&feedback(), FeedbackEvent::Choose { name: "Anchor".to_owned(), value: value.clone() }));
            } },
            onpointerleave: { let mut feedback = feedback; move |_| {
                if armed() { return; }
                feedback.set(advance_feedback(&feedback(), FeedbackEvent::Dismiss));
            } },
            onfocus: { let mut feedback = feedback; let value = current_label.clone(); move |_| {
                if armed() { return; }
                feedback.set(advance_feedback(&feedback(), FeedbackEvent::Choose { name: "Anchor".to_owned(), value: value.clone() }));
            } },
            onblur: { let mut feedback = feedback; move |_| {
                if armed() { return; }
                feedback.set(advance_feedback(&feedback(), FeedbackEvent::Dismiss));
            } },
            {face}
        }
        if armed() {
            for key in projected.keys.iter().filter_map(|key| {
                let target = PickTarget { matrix_id: key.matrix_id.clone(), row: key.row, column: key.column };
                anchor_candidate_pose(&scenes, &target).map(|pose| (key, target, pose))
            }) {
                {
                    let (key, target, pose) = (key.0, key.1, key.2);
                    let hovered = hovered_target().is_some_and(|hover| hover == target);
                    let ring_r = if hovered { CANDIDATE_RING_HOVER_PX } else { CANDIDATE_RING_PX } / 2.0;
                    let label = anchor_key_label(&document, &target.matrix_id, target.row, target.column);
                    let hover_ring = hover_ring.clone();
                    let commit_pick = commit_pick.clone();
                    let enter_target = target.clone();
                    let commit_target_down = target.clone();
                    let commit_target_key = target.clone();
                    rsx! {
                        g {
                            "data-anchor-pick-candidate": "true", class: "outline-none focus-visible:stroke-foreground",
                            transform: "translate({pose.x} {pose.y}) scale({px} {-px})",
                            "data-anchor": "{target.matrix_id}:{target.row}:{target.column}",
                            role: "button", tabindex: "0",
                            "aria-label": "Anchor to {label}",
                            onpointerenter: { let hover = hover_ring.clone(); move |_| hover(Some(enter_target.clone())) },
                            onpointerleave: { let hover = hover_ring.clone(); move |_| hover(None) },
                            onpointerdown: { let commit = commit_pick.clone(); move |event| {
                                let Some(pointer) = event.data().try_as_web_event() else { return; };
                                pointer.prevent_default();
                                pointer.stop_propagation();
                                commit(commit_target_down.clone());
                            } },
                            onkeydown: { let commit = commit_pick.clone(); move |event: KeyboardEvent| {
                                let Some(key) = event.data().try_as_web_event() else { return; };
                                if key.key() != "Enter" { return; }
                                key.prevent_default();
                                commit(commit_target_key.clone());
                            } },
                            circle { r: "{CANDIDATE_HIT_PX / 2.0}", style: "fill:transparent;stroke:none;pointer-events:all" }
                            circle { r: "{ring_r}", style: "fill:var(--card);stroke:var(--geometry-part);stroke-width:1.5px;vector-effect:non-scaling-stroke" }
                            if key.current {
                                circle { r: "1.6", style: "fill:#fff;stroke:none;pointer-events:none" }
                            }
                        }
                    }
                }
            }
            // Prototype majors are display-only under the current Core
            // contract (keys only): dimmed rings with labels, no pointer
            // events, so they never commit. A detached owner's half origin
            // keeps the white current dot.
            for major in projected.majors.iter().filter_map(|major| {
                let (pose, label) = match &major.kind {
                    AnchorMajorKind::Part(id) => {
                        let part = document.parts.iter().find(|part| part.id == *id)?;
                        let label = if part.reference.trim().is_empty() { part.id.clone() } else { part.reference.clone() };
                        (evaluated_part_pose(&document, part).at, label)
                    }
                    AnchorMajorKind::HalfOrigin => {
                        let layout = layout_id.clone()?;
                        // An unsplit layout has no half offset: its half frame
                        // sits at the board origin.
                        let origin = layout_half_offset(&document, &layout)
                            .unwrap_or(Vec2 { x: 0.0, y: 0.0 });
                        (origin, "Half origin".to_owned())
                    }
                };
                Some((major, pose, label))
            }) {
                {
                    let (major, pose, label) = (major.0, major.1, major.2);
                    let ring_r = MAJOR_RING_PX / 2.0;
                    rsx! {
                        g {
                            "data-anchor-pick-major": "true",
                            transform: "translate({pose.x} {pose.y}) scale({px} {-px})",
                            opacity: "0.55",
                            style: "pointer-events:none",
                            "aria-hidden": "true",
                            circle { r: "{ring_r}", style: "fill:var(--card);stroke:var(--geometry-part);stroke-width:1.5px;vector-effect:non-scaling-stroke" }
                            path { d: "M{-ring_r - 4.0} 0h{ring_r * 2.0 + 8.0}M0 {-ring_r - 4.0}v{ring_r * 2.0 + 8.0}", style: "fill:none;stroke:var(--geometry-part);stroke-width:1px;vector-effect:non-scaling-stroke" }
                            text { x: "{ring_r + 6.0}", y: "{0.0 - ring_r - 2.0}", style: "font-size:11px;fill:var(--muted-foreground)", "{label}" }
                            if major.current {
                                circle { r: "2.2", style: "fill:#fff;stroke:none;pointer-events:none" }
                            }
                        }
                    }
                }
            }
            // Amber marks the Core-evaluated post-commit poses that move.
            for moved in preview().into_iter().filter_map(|(id, pose)| {
                let current = current_poses.get(&id).copied()?;
                (current != pose).then_some((id, current, pose))
            }) {
                {
                    let (_, current, pose) = moved;
                    let amber_r = CANDIDATE_RING_PX / 2.0 * px;
                    rsx! {
                        circle {
                            "data-anchor-pick-preview": "true",
                            cx: "{pose.at.x}", cy: "{pose.at.y}", r: "{amber_r}",
                            style: "fill:none;stroke:var(--warning-dark);stroke-width:{1.5 * px};pointer-events:none",
                            "aria-hidden": "true",
                        }
                        line {
                            "data-anchor-pick-preview": "true",
                            x1: "{current.at.x}", y1: "{current.at.y}",
                            x2: "{pose.at.x}", y2: "{pose.at.y}",
                            style: "stroke:var(--warning-dark);stroke-width:{1.0 * px};stroke-dasharray:{3.0 * px} {2.0 * px};pointer-events:none",
                            "aria-hidden": "true",
                        }
                    }
                }
            }
            {anchor_pick_popup_panel(popup_rows, view_x, view_y, width, px)}
        }
        // While armed the pinned panel above replaces the docked popup.
        {feedback_chrome(&feedback_state, !armed(), None, Some(&rail_layout), px, (view_x, view_y, width))}
    }
}

/// Ticket 08: the armed Anchor popup, pinned to the canvas's top-right
/// corner so it never covers a candidate. Same panel treatment as the
/// value popup, with the pick hint instead of the drag hint.
pub(super) fn anchor_pick_popup_panel(
    rows: Vec<(String, f64, &'static str, bool)>,
    view_x: f64,
    view_y: f64,
    view_width: f64,
    px: f64,
) -> Element {
    let panel_w = rows
        .iter()
        .map(|(text, size, _, _)| feedback_line_width(text, *size))
        .fold(0.0_f64, f64::max)
        + 16.0;
    let panel_x = view_x + view_width - panel_w * px - 12.0 * px;
    let panel_y = -view_y - 12.0 * px;
    rsx! {
        g {
            "data-feedback-popup": "true", class: "fill-foreground",
            transform: "translate({panel_x} {panel_y}) scale({px} {-px})",
            style: "pointer-events:none",
            "aria-hidden": "true",
            {feedback_panel_contents(panel_w, rows)}
        }
    }
}
