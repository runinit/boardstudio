//! Case handle-drag state machine: gasket/mount drafts, provisional CAD
//! preview documents and commit-on-release.

use super::CaseSelection;
use crate::case_bodies::CaseBodyEdit;
use crate::case_gesture_math::{
    MountMoveConstraints, move_case_mount, move_gasket_support, supports_equal_positions,
};
use crate::case_handles::{gasket_handles, mechanical_mounts, mount_constraints, mount_handles};
use crate::mechanical_settings::{
    self, MechanicalMountCollection as Collection, MechanicalSettingsPatchExt,
    MechanicalSettingsProps as Settings, gasket_anchor, submit_settings_patch,
};
use crate::runtime::{CadScene, Runtime};
use crate::shared_viewer::{
    CaseMountTarget, HandleGesturePhase, ViewerHandle, ViewerHandleTarget, ViewerIdentity,
};
use crate::viewport_camera::Drag;
use boardstudio_model::model::{
    MechanicalGasketAnchor, MechanicalGasketSupport, MechanicalGasketTrack, Mount, ProjectDoc, Vec2,
};
use boardstudio_web_host::cad_jobs::captured_case_document;
use dioxus::prelude::*;
use std::{cell::RefCell, rc::Rc};

pub(super) enum CaseGestureDraft {
    Gasket {
        support_id: String,
        before: Vec<MechanicalGasketSupport>,
        pending: Vec<MechanicalGasketSupport>,
        tracks: Vec<MechanicalGasketTrack>,
    },
    Mount {
        target: CaseMountTarget,
        mount_id: String,
        before: Vec<Mount>,
        pending: Vec<Mount>,
        constraints: MountMoveConstraints,
    },
}

impl CaseGestureDraft {
    fn move_to(&mut self, point: Vec2) -> bool {
        match self {
            Self::Gasket {
                support_id,
                before,
                pending,
                tracks,
            } => {
                if let Some(next) = move_gasket_support(point, support_id, before, tracks) {
                    *pending = next;
                    return true;
                }
            }
            Self::Mount {
                mount_id,
                before,
                pending,
                constraints,
                ..
            } => {
                if let Some(next) = move_case_mount(point, mount_id, before, constraints) {
                    *pending = next;
                    return true;
                }
            }
        }
        false
    }
}

pub(super) type CaseDrag =
    Drag<CaseGestureDraft, crate::case_gesture_preview::CaseGesturePreviewOwner>;

pub(super) fn retire_drag(
    runtime: &Runtime,
    gesture: &RefCell<CaseDrag>,
) -> Option<CaseGestureDraft> {
    let (draft, preview) = gesture.borrow_mut().retire();
    if let Some(preview) = preview {
        runtime.cancel_case_gesture_preview(&preview);
    }
    draft
}

#[allow(clippy::too_many_arguments)]
pub(super) fn handle_case_gesture(
    scene: &CadScene,
    identity: &ViewerIdentity,
    settings: Option<&mechanical_settings::MechanicalSettingsProps>,
    handles: &[ViewerHandle],
    gesture: &RefCell<CaseDrag>,
    preview: &mut Signal<Option<Vec<ViewerHandle>>>,
    feedback_field: &mut Signal<Option<String>>,
    message: &mut Signal<Option<String>>,
    selection: CaseSelection,
    runtime: &Rc<Runtime>,
    live_preview: bool,
    phase: HandleGesturePhase,
    handle_id: &str,
    point: Option<[f32; 3]>,
) {
    let identity_current = scene.exact
        && scene.scope == identity.scope
        && scene.token == identity.snapshot_token
        && scene.prepared.revision == scene.snapshot.document.revision;
    let settings = settings.filter(|settings| {
        settings.editable
            && settings.identity.scope == identity.scope
            && settings.identity.snapshot_token == identity.snapshot_token
            && settings.identity.revision == scene.snapshot.document.revision
            && identity_current
    });
    match phase {
        HandleGesturePhase::Start if identity_current => {
            let Some(handle) = handles.iter().find(|handle| handle.id == handle_id) else {
                return;
            };
            let Some(draft) = start_drag(scene, settings, selection, &handle.target) else {
                return;
            };
            gesture.borrow_mut().draft = Some(draft);
            feedback_field.set(None);
            message.set(Some(if handle_id.starts_with("gasket-handle:") {
                "Drag the gasket support; release to save its position.".to_owned()
            } else {
                "Drag the mount; release to save its case position.".to_owned()
            }));
        }
        HandleGesturePhase::Move if identity_current => {
            let Some(point) = point.map(world_xy) else {
                return;
            };
            let (handle_overlay, guidance, provisional) = {
                let mut gesture = gesture.borrow_mut();
                let Some(draft) = gesture.draft.as_mut() else {
                    return;
                };
                let valid = draft.move_to(point);
                let (overlay, guidance) = match draft {
                    CaseGestureDraft::Gasket {
                        support_id,
                        pending,
                        ..
                    } => (
                        gasket_handles(handles, pending, (!valid).then_some(support_id.as_str())),
                        if valid {
                            "Release to place; fit is checked after placement."
                        } else {
                            "No valid gasket perimeter position here."
                        },
                    ),
                    CaseGestureDraft::Mount {
                        target,
                        mount_id,
                        pending,
                        ..
                    } => (
                        mount_handles(
                            handles,
                            target,
                            pending,
                            (!valid).then_some(mount_id.as_str()),
                        ),
                        if valid {
                            "Release to save mount position."
                        } else {
                            "Placement blocked by the case edge, openings, or another mount."
                        },
                    ),
                };
                let provisional = (valid && live_preview)
                    .then(|| case_gesture_preview_document(scene, draft))
                    .flatten();
                (overlay, guidance, provisional)
            };
            preview.set(Some(handle_overlay));
            message.set(Some(guidance.into()));
            if let Some(document) = provisional
                && let Some(owner) = runtime.update_case_gesture_preview(
                    scene.scope.clone(),
                    scene.token,
                    scene.snapshot.document.revision,
                    document,
                )
            {
                gesture.borrow_mut().preview = Some(owner);
            }
        }
        HandleGesturePhase::End => {
            let current = retire_drag(runtime, gesture);
            preview.set(None);
            if !identity_current {
                message.set(None);
                return;
            }
            let Some(mut current) = current else { return };
            if point.is_some_and(|point| !current.move_to(world_xy(point))) {
                message.set(Some("Blocked move was not saved.".into()));
                return;
            }
            let mut body_edit = None;
            let patch = match current {
                CaseGestureDraft::Gasket {
                    support_id,
                    before,
                    pending,
                    ..
                } => {
                    if supports_equal_positions(&before, &pending) {
                        None
                    } else {
                        let anchors = changed_anchors(&before, &pending).collect::<Vec<_>>();
                        (!anchors.is_empty()).then_some(
                            mechanical_settings::MechanicalSettingsPatch::SetGasketSupportPlacement {
                                support_id,
                                anchors,
                            },
                        )
                    }
                }
                CaseGestureDraft::Mount {
                    target,
                    mount_id,
                    before,
                    pending,
                    ..
                } => pending
                    .iter()
                    .find(|mount| mount.id == mount_id)
                    .zip(before.iter().find(|mount| mount.id == mount_id))
                    .filter(|(next, old)| {
                        (next.at.x - old.at.x).abs() > 1e-7 || (next.at.y - old.at.y).abs() > 1e-7
                    })
                    .and_then(|(mount, _)| match target {
                        CaseMountTarget::Mechanical(collection) => Some(
                            mechanical_settings::MechanicalSettingsPatch::SetMountPosition {
                                collection,
                                mount_id,
                                at: mount.at,
                            },
                        ),
                        CaseMountTarget::Authored { body_id } => {
                            body_edit = Some(CaseBodyEdit::SetMountPosition {
                                body_id,
                                mount_id,
                                at: mount.at,
                            });
                            None
                        }
                    }),
            };
            if let Some(patch) = patch {
                let field_id = patch.field_id();
                if settings.is_some_and(|settings| submit_settings_patch(settings, patch)) {
                    feedback_field.set(Some(field_id));
                    message.set(Some("Saving position…".into()));
                } else {
                    message.set(Some("Position could not be submitted.".into()));
                }
            } else if let Some(edit) = body_edit {
                if let Some(dispatch) = selection.body_edit_portal.dispatch.read().clone() {
                    dispatch(
                        edit,
                        identity.scope.clone(),
                        identity.snapshot_token,
                        scene.snapshot.document.revision,
                    );
                    message.set(Some("Saving body mount position…".into()));
                } else {
                    message.set(Some("The Case body edit owner is unavailable.".into()));
                }
            } else {
                message.set(None);
            }
        }
        HandleGesturePhase::Cancel => {
            retire_drag(runtime, gesture);
            preview.set(None);
            feedback_field.set(None);
            message.set(Some("Move cancelled.".into()));
        }
        _ => {}
    }
}

fn start_drag(
    scene: &CadScene,
    settings: Option<&Settings>,
    selection: CaseSelection,
    handle: &ViewerHandleTarget,
) -> Option<CaseGestureDraft> {
    let (body, mount_id, mounts, target) = match handle {
        ViewerHandleTarget::Gasket { support_id } => {
            settings?;
            let assembly = scene.mechanical.as_ref()?;
            let before = &assembly.gasket_supports;
            before.iter().find(|support| support.id == *support_id)?;
            selection.select_layer(scene.scope.clone(), format!("gasket:{support_id}:lower"));
            return Some(CaseGestureDraft::Gasket {
                support_id: support_id.clone(),
                before: before.clone(),
                pending: before.clone(),
                tracks: assembly.gasket_tracks.clone(),
            });
        }
        ViewerHandleTarget::Mount { owner, mount_id } => {
            let (body, mounts) = match owner {
                CaseMountTarget::Mechanical(collection) => {
                    let (body, mounts) =
                        mechanical_mounts(settings?.configuration.as_deref()?, *collection);
                    (body, mounts.to_vec())
                }
                CaseMountTarget::Authored { body_id } => {
                    if !(selection.body_edit_portal.editable)() {
                        return None;
                    }
                    let document = captured_case_document(&scene.snapshot, &scene.scope).ok()?;
                    let body = document.case_bodies.into_iter().find(|body| {
                        body.id == *body_id && body.board_id == scene.scope.board_id
                    })?;
                    (body_id.as_str(), body.mounts?)
                }
            };
            (body, mount_id, mounts, owner.clone())
        }
    };
    mounts.iter().find(|mount| mount.id == *mount_id)?;
    Some(CaseGestureDraft::Mount {
        target,
        mount_id: mount_id.clone(),
        pending: mounts.clone(),
        before: mounts,
        constraints: mount_constraints(scene, body)?,
    })
}

fn world_xy(point: [f32; 3]) -> Vec2 {
    Vec2 {
        x: f64::from(point[0]),
        y: f64::from(point[1]),
    }
}

fn changed_anchors<'a>(
    before: &'a [MechanicalGasketSupport],
    pending: &'a [MechanicalGasketSupport],
) -> impl Iterator<Item = MechanicalGasketAnchor> + 'a {
    pending
        .iter()
        .filter(|support| {
            before
                .iter()
                .find(|old| old.id == support.id)
                .is_none_or(|old| (old.anchor - support.anchor).abs() > 1e-7)
        })
        .map(gasket_anchor)
}

fn case_gesture_preview_document(scene: &CadScene, draft: &CaseGestureDraft) -> Option<ProjectDoc> {
    let effective = captured_case_document(&scene.snapshot, &scene.scope).ok()?;
    let mut document = (*scene.snapshot.document).clone();
    let mut configuration = effective.mechanical;
    match draft {
        CaseGestureDraft::Gasket {
            before, pending, ..
        } => {
            let supports = &mut configuration.as_mut()?.gasket_layout.as_mut()?.supports;
            for anchor in changed_anchors(before, pending) {
                if let Some(existing) = supports
                    .iter_mut()
                    .find(|existing| existing.id == anchor.id)
                {
                    *existing = anchor;
                } else {
                    supports.push(anchor);
                }
            }
        }
        CaseGestureDraft::Mount {
            target, pending, ..
        } => match target {
            CaseMountTarget::Mechanical(Collection::Suspension) => {
                configuration.as_mut()?.mounts = pending.clone();
            }
            CaseMountTarget::Mechanical(Collection::Closure) => {
                configuration.as_mut()?.closure_mounts = Some(pending.clone());
            }
            CaseMountTarget::Authored { body_id } => {
                let body = document
                    .case_bodies
                    .iter_mut()
                    .find(|body| body.id == *body_id && body.board_id == scene.scope.board_id)?;
                body.mounts = Some(pending.clone());
                return Some(document);
            }
        },
    }
    // The projection reapplies shared defaults and retains instance-local placements.
    let target = if let Some(instance_id) = scene.scope.instance_id.as_deref() {
        &mut document
            .hardware
            .as_mut()?
            .instances
            .iter_mut()
            .find(|instance| instance.id == instance_id)?
            .mechanical
    } else {
        &mut document.mechanical
    };
    *target = Some(configuration?);
    Some(document)
}
