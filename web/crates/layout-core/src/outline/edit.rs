//! Pure outline edit conversion, evaluation, and inspector projection helpers.

use super::planner::{
    EditablePerimeter, OutlineAction, OutlineActionKind, OutlinePointTarget, PerimeterAnchor,
    connection_point_world, outline_connection_control, unique_outline_entity_id,
    unique_outline_version_id,
};
use crate::outline_settings::{OutlineEdit as OutlineSettingEdit, generated_feature};
use boardstudio_application::{AcceptedSnapshot, Durability, Lifecycle, ReadModel, Scope};
use boardstudio_core_kernel::outline_edit::{
    OutlineChange, OutlineEdit, OutlineEditDisposition, OutlineEditTarget,
};
use boardstudio_model::model::{
    Board, BoardOutline, BoardOutlineScene, Contour, EditPhase, Operation, OutlineConnection,
    OutlineControlPoint, OutlineFeature, OutlineVersion, ProjectDoc, Side, Vec2,
};
use boardstudio_web_ui_model::tree::TreeContext;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum OutlineDrawTool {
    Polygon(Operation),
    Connect,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutlineFeedback {
    pub scope: Scope,
    pub generation: u64,
    pub board_id: String,
    pub message: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutlineVersionOwnership {
    Generated,
    Linked,
    Edited,
}

impl OutlineVersionOwnership {
    pub fn label(self) -> &'static str {
        match self {
            Self::Generated => "Generated",
            Self::Linked => "Linked refinement",
            Self::Edited => "Edited outline",
        }
    }
}

/// One selectable outline: its Core-owned ownership plus the accepted scene's located problem.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutlineVersionChoice {
    pub id: String,
    pub name: String,
    pub ownership: OutlineVersionOwnership,
    pub problem_finding_id: Option<String>,
    pub problem_message: Option<String>,
    pub problem_selectable: bool,
    pub blocked_exports: Vec<String>,
}

impl OutlineVersionChoice {
    pub fn label(&self) -> String {
        format!("{} \u{b7} {}", self.ownership.label(), self.name)
    }
}

pub fn outline_state<'a>(
    snapshot: &'a AcceptedSnapshot,
    board_id: &str,
) -> Option<&'a BoardOutline> {
    snapshot
        .document
        .board_outlines
        .iter()
        .find(|s| s.board_id == board_id)
}

pub fn active_outline<'a>(
    snapshot: &'a AcceptedSnapshot,
    board_id: &str,
) -> (Option<&'a BoardOutline>, Option<&'a OutlineVersion>) {
    let state = outline_state(snapshot, board_id);
    let version = state
        .and_then(|s| s.active_version_id.as_deref())
        .and_then(|id| state?.versions.iter().find(|v| v.id == id));
    (state, version)
}

pub fn outline_version<'a>(
    outline: Option<&'a BoardOutline>,
    version_id: &str,
) -> Option<&'a OutlineVersion> {
    outline?.versions.iter().find(|v| v.id == version_id)
}

fn board_outline_scene<'a>(
    snapshot: &'a AcceptedSnapshot,
    board_id: &str,
) -> Option<&'a BoardOutlineScene> {
    snapshot
        .scene
        .board_outline_scenes
        .iter()
        .find(|s| s.board_id == board_id)
}

pub fn version_geometry(
    outline: Option<&BoardOutline>,
    active_version_id: Option<&str>,
    board: &Board,
    document: &ProjectDoc,
    generated: Option<&OutlineFeature>,
) -> (
    Vec<OutlineFeature>,
    Vec<OutlineConnection>,
    Option<OutlineFeature>,
) {
    let active = active_version_id.and_then(|id| outline_version(outline, id));
    let features = active
        .map(|v| {
            v.geometry
                .features
                .iter()
                .filter(|f| !matches!(f, OutlineFeature::PartEnvelope { .. }))
                .cloned()
                .collect()
        })
        .unwrap_or_else(|| {
            board
                .outline_ids
                .iter()
                .filter_map(|id| document.outline.iter().find(|f| f.id() == id))
                .filter(|f| !matches!(f, OutlineFeature::PartEnvelope { .. }))
                .cloned()
                .collect()
        });
    let conn_feature = active
        .and_then(|v| {
            v.geometry
                .features
                .iter()
                .find(|f| matches!(f, OutlineFeature::PartEnvelope { .. }))
                .cloned()
        })
        .or_else(|| generated.cloned());
    let connections = match conn_feature.as_ref() {
        Some(OutlineFeature::PartEnvelope { connections, .. }) => connections.clone(),
        _ => Vec::new(),
    };
    (features, connections, conn_feature)
}

pub fn blocked_export_labels(snapshot: &AcceptedSnapshot, board_id: &str) -> Vec<String> {
    let Some(r) = snapshot
        .scene
        .board_readiness
        .iter()
        .find(|r| r.board_id == board_id)
    else {
        return Vec::new();
    };
    [
        (!r.outline, "Outline"),
        (!r.pcb, "PCB"),
        (!r.case_ready, "Case"),
    ]
    .into_iter()
    .filter(|(blocked, _)| *blocked)
    .map(|(_, label)| label.to_owned())
    .collect()
}

pub fn editable_perimeter(
    snapshot: &AcceptedSnapshot,
    board_id: &str,
    active_version_id: Option<&str>,
    selected_feature_id: Option<&str>,
) -> Option<EditablePerimeter> {
    if let Some(vid) = active_version_id {
        let OutlineFeature::Polygon {
            id,
            points,
            anchor_part_id,
            operation,
        } = outline_version(outline_state(snapshot, board_id), vid)?
            .geometry
            .features
            .iter()
            .find(|f| {
                matches!(f, OutlineFeature::Polygon { .. })
                    && selected_feature_id.is_none_or(|id| f.id() == id)
            })?
        else {
            return None;
        };
        let anchor = anchor_part_id
            .as_deref()
            .and_then(|id| snapshot.document.parts.iter().find(|p| p.id == id))
            .map(|p| PerimeterAnchor {
                at: p.pose.at,
                rotation: p.pose.rotation,
                back: p.side == Side::Back,
            });
        return Some(EditablePerimeter {
            target: OutlinePointTarget::Fixed {
                version_id: vid.to_owned(),
                feature_id: id.clone(),
                anchor_part_id: anchor_part_id.clone(),
                operation: *operation,
            },
            points: points.clone(),
            canvas_points: points
                .iter()
                .map(|p| anchor.map_or(*p, |a| a.world(*p)))
                .collect(),
            anchor,
        });
    }
    let source = board_outline_scene(snapshot, board_id)?
        .source_contours
        .first()?;
    Some(EditablePerimeter {
        target: OutlinePointTarget::Generated { contour: 0 },
        points: source.points.clone(),
        canvas_points: source.points.clone(),
        anchor: None,
    })
}

pub fn gap_focus_camera(
    snapshot: &AcceptedSnapshot,
    board_id: &str,
    gap_id: &str,
    (sw, sh): (f64, f64),
    (bmin_x, bmax_x, bmin_y, bmax_y): (f64, f64, f64, f64),
) -> Option<(Vec2, f64)> {
    let gap = board_outline_scene(snapshot, board_id)?
        .gaps
        .iter()
        .find(|g| g.id == gap_id)?;
    let (min_x, max_x, min_y, max_y) = gap
        .points
        .iter()
        .map(|p| connection_point_world(p, &snapshot.document.parts))
        .filter(|p| p.x.is_finite() && p.y.is_finite())
        .fold(None::<(f64, f64, f64, f64)>, |b, p| {
            Some(b.map_or((p.x, p.x, p.y, p.y), |(a, b, c, d)| {
                (a.min(p.x), b.max(p.x), c.min(p.y), d.max(p.y))
            }))
        })?;
    // Match React's canvas fit around a contour: include 12 mm breathing room and preserve target offset relative to the stable Layout camera basis.
    let (tw, th, bw, bh) = (
        (max_x - min_x + 24.0).max(1.0),
        (max_y - min_y + 24.0).max(1.0),
        (bmax_x - bmin_x).max(50.0),
        (bmax_y - bmin_y).max(50.0),
    );
    let zoom = (bw / tw * (sw - 32.0).max(1.0) / sw)
        .min(bh / th * (sh - 96.0).max(1.0) / sh)
        .clamp(0.15, 8.0);
    Some((
        Vec2 {
            x: (min_x + max_x - bmin_x - bmax_x) * 0.5,
            y: (min_y + max_y - bmin_y - bmax_y) * 0.5 + 48.0 * bh / zoom / sh * 0.5,
        },
        zoom,
    ))
}

/// Every identity a new outline version must avoid: persisted versions plus retired ones,
/// whose IDs stay reserved forever and Core refuses to reissue them.
fn all_version_ids(document: &ProjectDoc) -> Vec<String> {
    document
        .board_outlines
        .iter()
        .flat_map(|s| {
            s.versions
                .iter()
                .map(|v| v.id.clone())
                .chain(s.retired_version_ids.iter().cloned())
        })
        .collect()
}

fn next_edited_outline_name(state: Option<&BoardOutline>) -> String {
    let next = state
        .into_iter()
        .flat_map(|s| &s.versions)
        .filter_map(|v| v.name.strip_prefix("Edited outline ")?.parse::<u32>().ok())
        .max()
        .unwrap_or(0)
        .saturating_add(1);
    format!("Edited outline {next}")
}

fn board_target(board_id: &str) -> OutlineEditTarget {
    OutlineEditTarget::Board {
        board_id: board_id.to_owned(),
    }
}
fn version_target(board_id: &str, vid: &str) -> OutlineEditTarget {
    OutlineEditTarget::Version {
        board_id: board_id.to_owned(),
        version_id: vid.to_owned(),
    }
}
fn settings_target(board_id: &str, active: Option<&str>) -> OutlineEditTarget {
    active.map_or_else(|| board_target(board_id), |v| version_target(board_id, v))
}

type Plan = (OutlineEditTarget, OutlineChange, Option<Vec<Contour>>);

fn update(
    doc: &ProjectDoc,
    snap: &AcceptedSnapshot,
    board_id: &str,
    active: Option<&str>,
    edit: &OutlineSettingEdit,
) -> Option<Plan> {
    use OutlineChange as C;
    use OutlineSettingEdit as S;
    // The Generated envelope owns these fields; a selected version owns its own copy.
    let generated_only =
        |c: OutlineChange| active.is_none().then(|| (board_target(board_id), c, None));
    let any_target = |c: OutlineChange| (settings_target(board_id, active), c, None);
    // Fields tied to the envelope are editable on a linked refinement too, so a source change
    // never forces a Freeze just to adjust them. A fixed edited copy has no envelope to apply them to.
    let linked_or_generated = |c: OutlineChange| match active {
        None => generated_only(c),
        Some(vid) => doc
            .board_outlines
            .iter()
            .find(|s| s.board_id == board_id)?
            .versions
            .iter()
            .find(|v| v.id == vid)
            .is_some_and(|v| !v.ownership.is_fixed())
            .then(|| any_target(c)),
    };
    match edit {
        S::RenameVersion { version_id, name } => Some((
            version_target(board_id, version_id),
            C::Rename {
                name: name.trim().to_owned(),
            },
            None,
        )),
        S::CreateAutomatic => {
            let board = doc.boards.iter().find(|b| b.id == board_id)?;
            Some((
                board_target(board_id),
                C::CreateGenerated {
                    margin: 4.0,
                    settings: crate::outline_settings::reference_outline_settings(),
                    part_ids: board.part_ids.clone(),
                },
                None,
            ))
        }
        S::SetMargin(margin) => linked_or_generated(C::SetMargin { margin: *margin }),
        S::SetBridgeWidth(bridge_width) => linked_or_generated(C::SetBridgeWidth {
            bridge_width: *bridge_width,
        }),
        S::SetRepairEnabled(enabled) => {
            linked_or_generated(C::SetRepairEnabled { enabled: *enabled })
        }
        S::SetMaximumGapSpan(value) => linked_or_generated(C::SetMaximumGapSpan { value: *value }),
        S::SetCorners(corners) => Some(any_target(C::SetCorners { corners: *corners })),
        S::SetSize(size) => Some(any_target(C::SetSize { size: *size })),
        S::SetMinimumConnectionWidth(value) => {
            Some(any_target(C::SetMinimumConnectionWidth { value: *value }))
        }
        S::SetEdgeClearance(value) => Some(any_target(C::SetEdgeClearance { value: *value })),
        S::SetProtectedGap { gap_id, protected } => {
            if active.is_some() {
                return None;
            }
            let scene = board_outline_scene(snap, board_id)?;
            let gap = scene.gaps.iter().find(|g| g.id == *gap_id)?;
            let board = doc.boards.iter().find(|b| b.id == board_id)?;
            if generated_feature(doc, board)?.id() != gap.feature_id {
                return None;
            }
            // Protecting stores the candidate's points, so the edit reviews the source shape it
            // came from and retires if a queued change moved it.
            let (change, reviewed) = if *protected {
                (
                    C::SetProtectedGap {
                        gap_id: gap.id.clone(),
                        points: gap.points.clone(),
                    },
                    Some(scene.source_contours.clone()),
                )
            } else {
                (
                    C::RemoveProtectedGap {
                        gap_id: gap.id.clone(),
                    },
                    None,
                )
            };
            Some((board_target(board_id), change, reviewed))
        }
        S::RemoveProtectedGap { gap_id } => generated_only(C::RemoveProtectedGap {
            gap_id: gap_id.clone(),
        }),
    }
}

/// One typed Core outline edit for this presentation action, or `None` when the action
/// cannot be expressed (including work that is not an edit at all, like focusing a gap).
pub fn outline_edit(
    snapshot: &AcceptedSnapshot,
    action: &OutlineAction,
    seed: u64,
) -> Option<OutlineEdit> {
    use OutlineActionKind as K;
    use OutlineChange as C;
    let doc = snapshot.document.as_ref();
    let board_id = &action.envelope.board_id;
    let board = doc.boards.iter().find(|b| b.id == *board_id)?;
    let state = doc.board_outlines.iter().find(|s| s.board_id == *board_id);
    let active = state.and_then(|s| s.active_version_id.as_deref());
    let (target, change, reviewed) = match &action.kind {
        K::Activate { version_id } => (
            settings_target(board_id, version_id.as_deref()),
            C::Activate,
            None,
        ),
        K::Copy => (
            board_target(board_id),
            C::Copy {
                version_id: unique_outline_version_id(seed, all_version_ids(doc)),
                name: next_edited_outline_name(state),
            },
            None,
        ),
        K::Delete { version_id } => (version_target(board_id, version_id), C::Delete, None),
        K::Update { edit } => update(doc, snapshot, board_id, active, edit)?,
        K::Refine => (
            board_target(board_id),
            C::CreateLinkedRefinement {
                version_id: unique_outline_version_id(seed, all_version_ids(doc)),
                name: next_edited_outline_name(state),
            },
            None,
        ),
        K::Freeze { version_id } => (
            version_target(board_id, version_id),
            C::Freeze {
                name: next_edited_outline_name(state),
            },
            None,
        ),
        K::EditPerimeter { target, points, .. } => {
            let reviewed = Some(
                board_outline_scene(snapshot, board_id)?
                    .source_contours
                    .clone(),
            );
            match target {
                OutlinePointTarget::Generated { contour } => (
                    board_target(board_id),
                    C::EditGeneratedPerimeter {
                        contour: *contour,
                        points: points.clone(),
                    },
                    reviewed,
                ),
                OutlinePointTarget::Fixed {
                    version_id,
                    feature_id,
                    ..
                } => (
                    version_target(board_id, version_id),
                    C::EditFeaturePerimeter {
                        feature_id: feature_id.clone(),
                        points: points.clone(),
                    },
                    reviewed,
                ),
            }
        }
        K::AddFeature { context, feature } => {
            let target = match context {
                TreeContext::OutlineVersion {
                    version_id: Some(vid),
                    ..
                } => version_target(board_id, vid),
                _ => board_target(board_id),
            };
            (
                target,
                C::AddFeature {
                    feature: feature.clone(),
                },
                None,
            )
        }
        K::AddConnection { points } => {
            let OutlineFeature::PartEnvelope {
                connections,
                settings,
                part_ids,
                ..
            } = generated_feature(doc, board)?
            else {
                return None;
            };
            let eligible: Vec<_> = doc
                .parts
                .iter()
                .filter(|p| {
                    part_ids.contains(&p.id) && !p.outline.as_ref().is_some_and(|o| o.excluded)
                })
                .collect();
            let id = unique_outline_entity_id(
                "outline-connection",
                seed,
                connections.iter().map(|c| c.id.clone()),
            );
            let pts = points
                .iter()
                .enumerate()
                .map(|(i, pt)| {
                    if i == 0 || i + 1 == points.len() {
                        outline_connection_control(*pt, &eligible)
                    } else {
                        OutlineControlPoint {
                            at: *pt,
                            part_id: None,
                        }
                    }
                })
                .collect();
            (
                board_target(board_id),
                C::AddConnection {
                    connection: OutlineConnection {
                        id,
                        width: settings.bridge_width,
                        points: pts,
                    },
                },
                None,
            )
        }
        K::SetFeature {
            version_id,
            before,
            after,
        } => {
            if before.id() != after.id() {
                return None;
            }
            (
                settings_target(board_id, version_id.as_deref()),
                C::SetFeature {
                    feature: after.clone(),
                },
                None,
            )
        }
        K::RemoveFeature {
            version_id,
            feature_id,
        } => (
            version_target(board_id, version_id),
            C::RemoveFeature {
                feature_id: feature_id.clone(),
            },
            None,
        ),
        K::FocusGap { .. } => return None,
    };
    Some(OutlineEdit {
        target,
        change,
        seed,
        reviewed,
    })
}

/// The durable ids the submitted edit addresses, for transaction bookkeeping.
pub fn target_ids(action: &OutlineAction) -> Vec<String> {
    use OutlineActionKind::*;
    let mut ids = vec![action.envelope.board_id.clone()];
    match &action.kind {
        Activate {
            version_id: Some(vid),
        }
        | Delete { version_id: vid }
        | Freeze { version_id: vid } => ids.push(vid.clone()),
        EditPerimeter {
            target:
                OutlinePointTarget::Fixed {
                    version_id,
                    feature_id,
                    ..
                },
            ..
        }
        | RemoveFeature {
            version_id,
            feature_id,
        } => {
            ids.push(version_id.clone());
            ids.push(feature_id.clone());
        }
        SetFeature {
            version_id, after, ..
        } => {
            if let Some(vid) = version_id {
                ids.push(vid.clone());
            }
            ids.push(after.id().to_owned());
        }
        _ => {}
    }
    ids
}

/// Admission at dispatch: evaluate against the current snapshot with the shape the user reviewed.
/// Core owns validation messages for Malformed edits (`Err`), while Unchanged or retired edits are
/// ignored early (`Ok(None)`) without creating history or queueing a doomed commit.
pub fn evaluate_outline_edit(
    snapshot: &AcceptedSnapshot,
    action: &OutlineAction,
    seed: u64,
) -> Result<Option<OutlineEdit>, String> {
    let Some(edit) = outline_edit(snapshot, action, seed) else {
        return Ok(None);
    };
    match boardstudio_core_kernel::outline_edit::evaluate(snapshot.document.as_ref(), &edit) {
        Ok(OutlineEditDisposition::Changed) => Ok(Some(edit)),
        Ok(OutlineEditDisposition::Unchanged | OutlineEditDisposition::Retire) => Ok(None),
        Err(message) => Err(message),
    }
}

pub fn outline_action_matches_saved_owner(
    action: &OutlineAction,
    model: &ReadModel,
    current_scope: Option<&Scope>,
    generation: u64,
) -> bool {
    let env = &action.envelope;
    let perimeter_edit = matches!(
        &action.kind,
        OutlineActionKind::EditPerimeter {
            phase: EditPhase::Preview | EditPhase::Commit,
            ..
        }
    );
    generation == env.generation
        && current_scope == Some(&env.scope)
        && model.accepted.as_ref().is_some_and(|s| {
            s.token == env.token
                && s.document.revision == env.revision
                && s.document.id == env.scope.document_id
                && s.session_epoch == env.scope.session_epoch
        })
        && env.scope.board_id == env.board_id
        && model.lifecycle == Lifecycle::Ready
        && model.durability
            == (Durability::Saved {
                revision: env.revision,
            })
        && (model.display_preview.is_none() || perimeter_edit)
        && model.gesture.is_none()
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use crate::outline::OutlineActionEnvelope;
    use boardstudio_application::{Event, SessionEpoch, SnapshotToken};
    use boardstudio_web_test_runtime::new_native_runtime;

    #[test]
    fn outline_copy_requires_the_captured_saved_owner() {
        let runtime = new_native_runtime();
        let mut document = ProjectDoc::empty("outline", "Outline");
        document.boards.push(Board {
            id: "board".into(),
            name: "Board".into(),
            outline_ids: vec![],
            part_ids: vec![],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        runtime.submit(Event::Open {
            operation_id: runtime.operation(),
            document,
        });
        let mut model = runtime.model();
        let scope = runtime.scope().unwrap();
        let snapshot = model.accepted.as_ref().unwrap();
        let (revision, token) = (snapshot.document.revision, snapshot.token);
        let action = OutlineAction {
            envelope: OutlineActionEnvelope {
                scope: scope.clone(),
                token,
                revision,
                generation: 2,
                board_id: scope.board_id.clone(),
            },
            kind: OutlineActionKind::Copy,
        };
        let admitted = |model: &ReadModel, scope: Option<&Scope>, captured_gen| {
            outline_action_matches_saved_owner(&action, model, scope, captured_gen)
        };
        assert!(admitted(&model, Some(&scope), 2));
        assert!(!admitted(&model, Some(&scope), 3));
        assert!(!admitted(&model, None, 2));
        let replacement = Scope {
            session_epoch: SessionEpoch(scope.session_epoch.0 + 1),
            ..scope.clone()
        };
        assert!(!admitted(&model, Some(&replacement), 2));
        model.durability = Durability::Saving { revision };
        assert!(!admitted(&model, Some(&scope), 2));
        model.durability = Durability::Saved { revision };
        model.accepted.as_mut().unwrap().token = SnapshotToken(token.0 + 1);
        assert!(!admitted(&model, Some(&scope), 2));
    }
}
