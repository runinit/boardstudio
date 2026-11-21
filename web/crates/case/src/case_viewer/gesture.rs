use super::*;
use crate::mechanical_settings;

#[derive(Clone)]
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

#[derive(Clone)]
pub(super) enum CaseMountTarget {
    Mechanical(mechanical_settings::MechanicalMountCollection),
    Authored { body_id: String },
}

#[derive(Clone)]
pub(super) struct MountMoveConstraints {
    outer: Vec<Vec<Vec2>>,
    holes: Vec<Vec<Vec2>>,
}

pub(super) fn case_viewer_handles(
    scene: &CadScene,
    settings: Option<&mechanical_settings::MechanicalSettingsProps>,
    authored_mounts_editable: bool,
) -> Vec<ViewerHandle> {
    let settings = settings.filter(|settings| {
        settings.editable
            && settings.identity.scope == scene.scope
            && settings.identity.snapshot_token == scene.token
            && settings.identity.revision == scene.snapshot.document.revision
    });
    if !scene.exact || scene.prepared.revision != scene.snapshot.document.revision {
        return Vec::new();
    }

    let mut handles = Vec::new();
    if settings.is_some()
        && let Some(assembly) = scene.mechanical.as_ref()
    {
        let z = assembly
            .stack
            .iter()
            .find(|layer| layer.id == "retainer")
            .map(|layer| layer.z + layer.thickness + 0.7)
            .unwrap_or(9.0);
        handles.extend(assembly.gasket_supports.iter().map(|support| ViewerHandle {
            id: format!("gasket-handle:{}", support.id),
            x: support.at.x as f32,
            y: support.at.y as f32,
            z: z as f32,
            tangent_x: support.tangent.x as f32,
            tangent_y: support.tangent.y as f32,
            normal_x: support.normal.x as f32,
            normal_y: support.normal.y as f32,
            length: support.length as f32,
            invalid: support.fit_error.is_some(),
            target: ViewerHandleTarget::Gasket {
                support_id: support.id.clone(),
            },
        }));
    }

    if let (Some(settings), Some(assembly)) = (settings, scene.mechanical.as_ref())
        && let Some(values) = settings.values.as_ref()
    {
        if values.mount != MechanicalMount::Gasket {
            let target_body = if values.mount == MechanicalMount::Rigid {
                "plate"
            } else {
                "bottom"
            };
            if mount_constraints(scene, target_body).is_some() {
                let z = assembly
                    .stack
                    .iter()
                    .find(|layer| layer.id == target_body)
                    .map(|layer| layer.z + 0.8)
                    .unwrap_or(0.8);
                handles.extend(values.suspension_mounts.iter().map(|mount| {
                    mount_handle(
                        mount,
                        z,
                        CaseMountTarget::Mechanical(
                            mechanical_settings::MechanicalMountCollection::Suspension,
                        ),
                    )
                }));
            }
        }
        if let Some(mounts) = values.closure_mounts.as_ref()
            && let Some(_constraints) = mount_constraints(scene, "bottom")
        {
            let z = assembly
                .stack
                .iter()
                .find(|layer| layer.id == "bottom")
                .map(|layer| layer.z + 0.8)
                .unwrap_or(0.8);
            handles.extend(mounts.iter().map(|mount| {
                mount_handle(
                    mount,
                    z,
                    CaseMountTarget::Mechanical(
                        mechanical_settings::MechanicalMountCollection::Closure,
                    ),
                )
            }));
        }
    }
    if authored_mounts_editable
        && let Ok(document) = captured_case_document(&scene.snapshot, &scene.scope)
    {
        for body in document
            .case_bodies
            .iter()
            .filter(|body| body.board_id == scene.scope.board_id)
        {
            let Some(mounts) = body.mounts.as_ref() else {
                continue;
            };
            let Some(_constraints) = mount_constraints(scene, &body.id) else {
                continue;
            };
            let z = body.z.unwrap_or(0.0) + 0.8;
            handles.extend(mounts.iter().map(|mount| {
                mount_handle(
                    mount,
                    z,
                    CaseMountTarget::Authored {
                        body_id: body.id.clone(),
                    },
                )
            }));
        }
    }
    handles
}

fn mount_handle(mount: &Mount, z: f64, target: CaseMountTarget) -> ViewerHandle {
    let (target_id, target) = match target {
        CaseMountTarget::Mechanical(collection) => (
            match collection {
                mechanical_settings::MechanicalMountCollection::Suspension => {
                    "suspension".to_owned()
                }
                mechanical_settings::MechanicalMountCollection::Closure => "closure".to_owned(),
            },
            ViewerHandleTarget::Mount {
                collection,
                mount_id: mount.id.clone(),
            },
        ),
        CaseMountTarget::Authored { body_id } => (
            body_id.clone(),
            ViewerHandleTarget::AuthoredMount {
                body_id,
                mount_id: mount.id.clone(),
            },
        ),
    };
    ViewerHandle {
        id: format!("case-mount:{target_id}/{}", mount.id),
        x: mount.at.x as f32,
        y: mount.at.y as f32,
        z: z as f32,
        tangent_x: 1.0,
        tangent_y: 0.0,
        normal_x: 0.0,
        normal_y: 1.0,
        length: (mount_radius(mount) * 2.0 + 3.0) as f32,
        invalid: false,
        target,
    }
}

pub(super) struct CaseGestureContext<'a> {
    pub(super) scene: &'a CadScene,
    pub(super) identity: &'a ViewerIdentity,
    pub(super) settings: Option<&'a mechanical_settings::MechanicalSettingsProps>,
    pub(super) handles: &'a [ViewerHandle],
    pub(super) gesture: &'a Rc<RefCell<Option<CaseGestureDraft>>>,
    pub(super) preview: &'a mut Signal<Option<Vec<ViewerHandle>>>,
    pub(super) feedback_field: &'a mut Signal<Option<String>>,
    pub(super) message: &'a mut Signal<Option<String>>,
    pub(super) selection: CaseSelection,
    pub(super) runtime: &'a Rc<Runtime>,
    pub(super) draft_preview_owner:
        &'a Rc<RefCell<Option<crate::case_gesture_preview::CaseGesturePreviewOwner>>>,
    pub(super) live_preview: bool,
}

pub(super) fn handle_case_gesture(
    context: CaseGestureContext<'_>,
    phase: HandleGesturePhase,
    handle_id: &str,
    point: Option<[f32; 3]>,
) {
    let CaseGestureContext {
        scene,
        identity,
        settings,
        handles,
        gesture,
        preview,
        feedback_field,
        message,
        selection,
        runtime,
        draft_preview_owner,
        live_preview,
    } = context;
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
        HandleGesturePhase::Start => {
            let Some(handle) = handles.iter().find(|handle| handle.id == handle_id) else {
                return;
            };
            *gesture.borrow_mut() = match &handle.target {
                ViewerHandleTarget::Gasket { support_id } => {
                    if settings.is_none() {
                        return;
                    }
                    let Some(assembly) = scene.mechanical.as_ref() else {
                        return;
                    };
                    let before = assembly.gasket_supports.clone();
                    if !before.iter().any(|support| support.id == *support_id) {
                        return;
                    }
                    selection
                        .select_layer(scene.scope.clone(), format!("gasket:{support_id}:lower"));
                    Some(CaseGestureDraft::Gasket {
                        support_id: support_id.clone(),
                        before: before.clone(),
                        pending: before,
                        tracks: assembly.gasket_tracks.clone(),
                    })
                }
                ViewerHandleTarget::Mount {
                    collection,
                    mount_id,
                } => {
                    let Some(settings) = settings else { return };
                    let Some(values) = settings.values.as_ref() else {
                        return;
                    };
                    let before = match collection {
                        mechanical_settings::MechanicalMountCollection::Suspension => {
                            values.suspension_mounts.clone()
                        }
                        mechanical_settings::MechanicalMountCollection::Closure => {
                            values.closure_mounts.clone().unwrap_or_default()
                        }
                    };
                    if !before.iter().any(|mount| mount.id == *mount_id) {
                        return;
                    }
                    let body_id = match collection {
                        mechanical_settings::MechanicalMountCollection::Suspension
                            if values.mount == MechanicalMount::Rigid =>
                        {
                            "plate"
                        }
                        _ => "bottom",
                    };
                    let Some(constraints) = mount_constraints(scene, body_id) else {
                        return;
                    };
                    Some(CaseGestureDraft::Mount {
                        target: CaseMountTarget::Mechanical(*collection),
                        mount_id: mount_id.clone(),
                        before: before.clone(),
                        pending: before,
                        constraints,
                    })
                }
                ViewerHandleTarget::AuthoredMount { body_id, mount_id } => {
                    if !identity_current || !(selection.body_edit_portal.editable)() {
                        return;
                    }
                    let Ok(document) = captured_case_document(&scene.snapshot, &scene.scope) else {
                        return;
                    };
                    let Some(body) = document
                        .case_bodies
                        .iter()
                        .find(|body| body.id == *body_id && body.board_id == scene.scope.board_id)
                    else {
                        return;
                    };
                    let before = body.mounts.clone().unwrap_or_default();
                    if !before.iter().any(|mount| mount.id == *mount_id) {
                        return;
                    }
                    let Some(constraints) = mount_constraints(scene, body_id) else {
                        return;
                    };
                    Some(CaseGestureDraft::Mount {
                        target: CaseMountTarget::Authored {
                            body_id: body_id.clone(),
                        },
                        mount_id: mount_id.clone(),
                        before: before.clone(),
                        pending: before,
                        constraints,
                    })
                }
            };
            feedback_field.set(None);
            message.set(Some(if handle_id.starts_with("gasket-handle:") {
                "Drag the gasket support; release to save its position.".to_owned()
            } else {
                "Drag the mount; release to save its case position.".to_owned()
            }));
        }
        HandleGesturePhase::Move => {
            let Some(point) = point.map(world_xy) else {
                return;
            };
            let Some(current) = gesture.borrow().clone() else {
                return;
            };
            let mut valid_point = false;
            match current {
                CaseGestureDraft::Gasket {
                    support_id,
                    before,
                    pending,
                    tracks,
                } => {
                    if let Some(next) = move_gasket_support(point, &support_id, &before, &tracks) {
                        valid_point = true;
                        *gesture.borrow_mut() = Some(CaseGestureDraft::Gasket {
                            support_id,
                            before,
                            pending: next.clone(),
                            tracks,
                        });
                        preview.set(Some(gasket_handles(handles, &next, None)));
                        message.set(Some(
                            "Release to place; fit is checked after placement.".into(),
                        ));
                    } else {
                        preview.set(Some(gasket_handles(handles, &pending, Some(&support_id))));
                        message.set(Some("No valid gasket perimeter position here.".into()));
                    }
                }
                CaseGestureDraft::Mount {
                    target,
                    mount_id,
                    before,
                    pending,
                    constraints,
                } => {
                    if let Some(next) = move_case_mount(point, &mount_id, &before, &constraints) {
                        valid_point = true;
                        *gesture.borrow_mut() = Some(CaseGestureDraft::Mount {
                            target: target.clone(),
                            mount_id,
                            before,
                            pending: next.clone(),
                            constraints,
                        });
                        preview.set(Some(mount_handles(handles, &target, &next, None)));
                        message.set(Some("Release to save mount position.".into()));
                    } else {
                        preview.set(Some(mount_handles(
                            handles,
                            &target,
                            &pending,
                            Some(&mount_id),
                        )));
                        message.set(Some(
                            "Placement blocked by the case edge, openings, or another mount."
                                .into(),
                        ));
                    }
                }
            }
            if valid_point
                && live_preview
                && let Some(draft) = gesture.borrow().clone()
                && let Some(document) = case_gesture_preview_document(scene, &draft)
                && let Some(owner) = runtime.update_case_gesture_preview(
                    scene.scope.clone(),
                    scene.token,
                    scene.snapshot.document.revision,
                    document,
                )
            {
                *draft_preview_owner.borrow_mut() = Some(owner);
            }
        }
        HandleGesturePhase::End => {
            if let Some(owner) = draft_preview_owner.borrow_mut().take() {
                runtime.cancel_case_gesture_preview(&owner);
            }
            if !identity_current {
                *gesture.borrow_mut() = None;
                preview.set(None);
                message.set(None);
                return;
            };
            let Some(current) = gesture.borrow().clone() else {
                return;
            };
            let current = if let Some(point) = point.map(world_xy) {
                match current {
                    CaseGestureDraft::Gasket {
                        support_id,
                        before,
                        tracks,
                        ..
                    } => move_gasket_support(point, &support_id, &before, &tracks).map(|pending| {
                        CaseGestureDraft::Gasket {
                            support_id,
                            before,
                            pending,
                            tracks,
                        }
                    }),
                    CaseGestureDraft::Mount {
                        target,
                        mount_id,
                        before,
                        constraints,
                        ..
                    } => move_case_mount(point, &mount_id, &before, &constraints).map(|pending| {
                        CaseGestureDraft::Mount {
                            target,
                            mount_id,
                            before,
                            pending,
                            constraints,
                        }
                    }),
                }
            } else {
                Some(current)
            };
            let Some(current) = current else {
                *gesture.borrow_mut() = None;
                preview.set(None);
                message.set(Some("Blocked move was not saved.".into()));
                return;
            };
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
                        let anchors = pending
                            .iter()
                            .filter(|support| {
                                before
                                    .iter()
                                    .find(|old| old.id == support.id)
                                    .is_none_or(|old| (old.anchor - support.anchor).abs() > 1e-7)
                            })
                            .map(gasket_anchor)
                            .collect::<Vec<_>>();
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
            *gesture.borrow_mut() = None;
            preview.set(None);
        }
        HandleGesturePhase::Cancel => {
            if let Some(owner) = draft_preview_owner.borrow_mut().take() {
                runtime.cancel_case_gesture_preview(&owner);
            }
            *gesture.borrow_mut() = None;
            preview.set(None);
            feedback_field.set(None);
            message.set(Some("Move cancelled.".into()));
        }
    }
}

pub(super) fn direct_gesture_message(
    field_id: Option<String>,
    transient: Option<String>,
    settings: Option<&mechanical_settings::MechanicalSettingsProps>,
) -> Option<String> {
    let Some(field_id) = field_id else {
        return transient;
    };
    let feedback = settings?
        .feedback
        .iter()
        .rev()
        .find(|entry| entry.field_id == field_id)?;
    match feedback.state {
        mechanical_settings::MechanicalSettingsFeedbackState::Pending => {
            Some("Saving position…".into())
        }
        mechanical_settings::MechanicalSettingsFeedbackState::Landed => None,
        mechanical_settings::MechanicalSettingsFeedbackState::Failed => Some(
            feedback
                .message
                .clone()
                .unwrap_or_else(|| "Position could not be saved.".into()),
        ),
    }
}

fn submit_settings_patch(
    settings: &mechanical_settings::MechanicalSettingsProps,
    patch: mechanical_settings::MechanicalSettingsPatch,
) -> bool {
    if !settings.editable {
        return false;
    }
    let mut sequence = settings.request_sequence;
    let Some(request_id) = sequence().checked_add(1) else {
        return false;
    };
    sequence.set(request_id);
    let field_id = patch.field_id();
    settings
        .on_request
        .call(mechanical_settings::MechanicalSettingsRequest {
            identity: settings.identity.clone(),
            request_id,
            field_id,
            patch,
            reviewed_fingerprint: None,
        });
    true
}

fn world_xy(point: [f32; 3]) -> Vec2 {
    Vec2 {
        x: f64::from(point[0]),
        y: f64::from(point[1]),
    }
}

fn gasket_anchor(support: &MechanicalGasketSupport) -> MechanicalGasketAnchor {
    MechanicalGasketAnchor {
        id: support.id.clone(),
        region_id: support.region_id.clone(),
        outline_key: support.outline_key.clone(),
        anchor: support.anchor,
        length: Some(support.length),
        width: Some(support.width),
        placement: Some(GasketPlacement::User),
        unlinked: support.unlinked,
    }
}

fn case_gesture_preview_document(scene: &CadScene, draft: &CaseGestureDraft) -> Option<ProjectDoc> {
    let mut effective = captured_case_document(&scene.snapshot, &scene.scope).ok()?;
    let mut document = (*scene.snapshot.document).clone();
    let mut mechanical_draft = None;
    match draft {
        CaseGestureDraft::Gasket {
            before, pending, ..
        } => {
            let mut configuration = effective.mechanical.take()?;
            let layout = configuration.gasket_layout.as_mut()?;
            for support in pending.iter().filter(|support| {
                before
                    .iter()
                    .find(|old| old.id == support.id)
                    .is_none_or(|old| (old.anchor - support.anchor).abs() > 1e-7)
            }) {
                let anchor = gasket_anchor(support);
                if let Some(existing) = layout
                    .supports
                    .iter_mut()
                    .find(|existing| existing.id == support.id)
                {
                    *existing = anchor;
                } else {
                    layout.supports.push(anchor);
                }
            }
            mechanical_draft = Some(configuration);
        }
        CaseGestureDraft::Mount {
            target: CaseMountTarget::Mechanical(collection),
            pending,
            ..
        } => {
            let mut configuration = effective.mechanical.take()?;
            match collection {
                mechanical_settings::MechanicalMountCollection::Suspension => {
                    configuration.mounts = pending.clone();
                }
                mechanical_settings::MechanicalMountCollection::Closure => {
                    configuration.closure_mounts = Some(pending.clone());
                }
            }
            mechanical_draft = Some(configuration);
        }
        CaseGestureDraft::Mount {
            target: CaseMountTarget::Authored { body_id },
            pending,
            ..
        } => {
            let body = document
                .case_bodies
                .iter_mut()
                .find(|body| body.id == *body_id && body.board_id == scene.scope.board_id)?;
            body.mounts = Some(pending.clone());
        }
    }
    if let Some(configuration) = mechanical_draft {
        if let Some(instance_id) = scene.scope.instance_id.as_deref() {
            let instance = document
                .hardware
                .as_mut()?
                .instances
                .iter_mut()
                .find(|instance| instance.id == instance_id)?;
            // The effective configuration includes shared defaults. The
            // existing projection reapplies those defaults and retains the
            // instance-local mount and gasket placement fields.
            instance.mechanical = Some(configuration);
        } else {
            document.mechanical = Some(configuration);
        }
    }
    Some(document)
}

fn supports_equal_positions(a: &[MechanicalGasketSupport], b: &[MechanicalGasketSupport]) -> bool {
    a.len() == b.len()
        && a.iter().all(|support| {
            b.iter()
                .find(|candidate| candidate.id == support.id)
                .is_some_and(|candidate| (support.anchor - candidate.anchor).abs() <= 1e-7)
        })
}

fn gasket_handles(
    original_handles: &[ViewerHandle],
    supports: &[MechanicalGasketSupport],
    invalid: Option<&str>,
) -> Vec<ViewerHandle> {
    let mut result = original_handles.to_vec();
    for support in supports {
        let id = format!("gasket-handle:{}", support.id);
        if let Some(handle) = result.iter_mut().find(|handle| handle.id == id) {
            handle.x = support.at.x as f32;
            handle.y = support.at.y as f32;
            handle.tangent_x = support.tangent.x as f32;
            handle.tangent_y = support.tangent.y as f32;
            handle.normal_x = support.normal.x as f32;
            handle.normal_y = support.normal.y as f32;
            handle.invalid = invalid == Some(support.id.as_str()) || support.fit_error.is_some();
        }
    }
    result
}

fn mount_handles(
    original_handles: &[ViewerHandle],
    target: &CaseMountTarget,
    mounts: &[Mount],
    invalid: Option<&str>,
) -> Vec<ViewerHandle> {
    let mut result = original_handles.to_vec();
    for handle in &mut result {
        let (handle_target, mount_id) = match &handle.target {
            ViewerHandleTarget::Mount {
                collection,
                mount_id,
            } => (CaseMountTarget::Mechanical(*collection), mount_id),
            ViewerHandleTarget::AuthoredMount { body_id, mount_id } => (
                CaseMountTarget::Authored {
                    body_id: body_id.clone(),
                },
                mount_id,
            ),
            ViewerHandleTarget::Gasket { .. } => continue,
        };
        if !same_mount_target(&handle_target, target) {
            continue;
        }
        if let Some(mount) = mounts.iter().find(|mount| mount.id == *mount_id) {
            handle.x = mount.at.x as f32;
            handle.y = mount.at.y as f32;
            handle.invalid = invalid == Some(mount.id.as_str());
        }
    }
    result
}

fn same_mount_target(left: &CaseMountTarget, right: &CaseMountTarget) -> bool {
    match (left, right) {
        (CaseMountTarget::Mechanical(left), CaseMountTarget::Mechanical(right)) => left == right,
        (
            CaseMountTarget::Authored { body_id: left },
            CaseMountTarget::Authored { body_id: right },
        ) => left == right,
        _ => false,
    }
}

fn move_gasket_support(
    point: Vec2,
    id: &str,
    supports: &[MechanicalGasketSupport],
    tracks: &[MechanicalGasketTrack],
) -> Option<Vec<MechanicalGasketSupport>> {
    let original = supports.iter().find(|support| support.id == id)?;
    let moved = project_gasket(point, original, tracks)?;
    let mut replacements = BTreeMap::from([(id.to_owned(), moved)]);
    if let Some(pair_id) = original.pair_id.as_ref()
        && let Some(axis) = original.mirror_axis
        && !original.unlinked
    {
        let pair = supports.iter().find(|support| support.id == *pair_id)?;
        if !pair.unlinked {
            let moved = replacements.get(id)?;
            let reflected = project_gasket(
                Vec2 {
                    x: 2.0 * axis - moved.at.x,
                    y: moved.at.y,
                },
                pair,
                tracks,
            )?;
            if (reflected.at.x - (2.0 * axis - moved.at.x)).hypot(reflected.at.y - moved.at.y)
                > 0.001
            {
                return None;
            }
            replacements.insert(pair.id.clone(), reflected);
        }
    }
    Some(
        supports
            .iter()
            .map(|support| replacements.get(&support.id).unwrap_or(support).clone())
            .collect(),
    )
}

fn project_gasket(
    point: Vec2,
    support: &MechanicalGasketSupport,
    tracks: &[MechanicalGasketTrack],
) -> Option<MechanicalGasketSupport> {
    let mut best = None;
    let mut best_distance = f64::INFINITY;
    for track in tracks
        .iter()
        .filter(|track| track.region_id == support.region_id)
    {
        let dx = track.end.x - track.start.x;
        let dy = track.end.y - track.start.y;
        let length_squared = dx * dx + dy * dy;
        if length_squared <= f64::EPSILON {
            continue;
        }
        let length = length_squared.sqrt();
        let t = (((point.x - track.start.x) * dx + (point.y - track.start.y) * dy)
            / length_squared)
            .clamp(0.0, 1.0);
        let at = Vec2 {
            x: track.start.x + t * dx,
            y: track.start.y + t * dy,
        };
        let distance = (point.x - at.x).hypot(point.y - at.y);
        if distance >= best_distance {
            continue;
        }
        best_distance = distance;
        let mut next = support.clone();
        next.at = at;
        next.anchor = track.start_anchor + t * (track.end_anchor - track.start_anchor);
        next.tangent = Vec2 {
            x: dx / length,
            y: dy / length,
        };
        next.normal = Vec2 {
            x: dy / length,
            y: -dx / length,
        };
        best = Some(next);
    }
    best
}

fn mount_constraints(scene: &CadScene, body_id: &str) -> Option<MountMoveConstraints> {
    let body = scene
        .prepared
        .bodies
        .iter()
        .find(|body| body.body.id == body_id)?;
    let base = body.body.z.unwrap_or(0.0);
    let top = base
        + body.body.thickness
        + if body.body.kind == CaseKind::Plate {
            0.0
        } else {
            body.body.wall_height.unwrap_or(0.0)
        };
    let mut holes = body
        .regions
        .iter()
        .flat_map(|region| region.holes.iter().cloned())
        .collect::<Vec<_>>();
    holes.extend(
        body.body
            .openings
            .as_deref()
            .unwrap_or_default()
            .iter()
            .filter(|opening| opening.z < top && opening.z + opening.height > base)
            .map(|opening| opening.points.clone()),
    );
    let outer = body
        .regions
        .iter()
        .map(|region| region.outer.clone())
        .filter(|outer| outer.len() >= 3)
        .collect::<Vec<_>>();
    if outer.is_empty() {
        return None;
    }
    Some(MountMoveConstraints { outer, holes })
}

fn move_case_mount(
    point: Vec2,
    id: &str,
    mounts: &[Mount],
    constraints: &MountMoveConstraints,
) -> Option<Vec<Mount>> {
    if !point.x.is_finite() || !point.y.is_finite() {
        return None;
    }
    let original = mounts.iter().find(|mount| mount.id == id)?;
    let radius = mount_radius(original);
    let margin = radius + 0.5;
    if !radius.is_finite() || radius <= 0.0 {
        return None;
    }
    if !constraints
        .outer
        .iter()
        .any(|outer| polygon_contains(outer, point) && edge_distance(point, outer) >= margin)
    {
        return None;
    }
    if constraints
        .holes
        .iter()
        .any(|hole| polygon_contains(hole, point) || edge_distance(point, hole) < margin)
    {
        return None;
    }
    if mounts.iter().any(|other| {
        if other.id == id {
            return false;
        }
        let other_radius = mount_radius(other);
        !other_radius.is_finite()
            || other_radius <= 0.0
            || (point.x - other.at.x).hypot(point.y - other.at.y) < radius + other_radius + 0.5
    }) {
        return None;
    }
    Some(
        mounts
            .iter()
            .map(|mount| {
                if mount.id == id {
                    let mut moved = mount.clone();
                    moved.at = point;
                    moved
                } else {
                    mount.clone()
                }
            })
            .collect(),
    )
}

fn mount_radius(mount: &Mount) -> f64 {
    if mount.kind == MountKind::Boss {
        mount.boss_diameter.unwrap_or(mount.hole_diameter) / 2.0
    } else {
        mount.hole_diameter / 2.0
    }
}

fn edge_distance(point: Vec2, polygon: &[Vec2]) -> f64 {
    let mut nearest = f64::INFINITY;
    for index in 0..polygon.len() {
        let a = &polygon[index];
        let b = &polygon[(index + 1) % polygon.len()];
        let dx = b.x - a.x;
        let dy = b.y - a.y;
        let length = dx * dx + dy * dy;
        let t = if length > 0.0 {
            (((point.x - a.x) * dx + (point.y - a.y) * dy) / length).clamp(0.0, 1.0)
        } else {
            0.0
        };
        nearest = nearest.min((point.x - a.x - t * dx).hypot(point.y - a.y - t * dy));
    }
    nearest
}
