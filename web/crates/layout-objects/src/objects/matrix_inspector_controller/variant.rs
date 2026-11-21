//! Preset-variant snapshots and history predicates for the Matrix Inspector.
use super::*;
use boardstudio_web_runtime::operation_outcomes::Settlement;

pub(super) fn exact_variant_snapshot(
    runtime: &Runtime,
    expected: &boardstudio_application::AcceptedSnapshot,
    variant_id: &str,
    source_scope: &Scope,
) -> Result<boardstudio_application::AcceptedSnapshot, String> {
    let model = runtime.model();
    let Some(current) = model.accepted else {
        return Err("The duplicated project is no longer active.".into());
    };
    let current_scope = runtime.scope();
    if model.lifecycle != Lifecycle::Ready
        || !variant_session_matches(
            expected.session_epoch,
            variant_id,
            current.session_epoch,
            &current.document.id,
        )
        || !current_scope.is_some_and(|scope| {
            scope.session_epoch == expected.session_epoch
                && scope.document_id == variant_id
                && scope.board_id == source_scope.board_id
                && scope.instance_id == source_scope.instance_id
        })
    {
        return Err("The duplicated project changed before its matrix preset was ready.".into());
    }
    Ok(current)
}

pub(super) fn current_variant_snapshot(
    runtime: &Runtime,
    expected: &boardstudio_application::AcceptedSnapshot,
    variant_id: &str,
    source_scope: &Scope,
) -> Result<Option<boardstudio_application::AcceptedSnapshot>, String> {
    let model = runtime.model();
    let current_scope = runtime.scope();
    if !current_scope.is_some_and(|scope| {
        scope.session_epoch == expected.session_epoch
            && scope.document_id == variant_id
            && scope.board_id == source_scope.board_id
            && scope.instance_id == source_scope.instance_id
    }) {
        return Ok(None);
    }
    let Some(current) = model.accepted else {
        return Err("The duplicated project is no longer active.".into());
    };
    if model.lifecycle != Lifecycle::Ready {
        return Err("The duplicated project changed before its matrix preset was ready.".into());
    }
    if !variant_session_matches(
        expected.session_epoch,
        variant_id,
        current.session_epoch,
        &current.document.id,
    ) {
        return Ok(None);
    }
    Ok(Some(current))
}

pub(super) async fn wait_for_variant_edit(
    runtime: &Rc<Runtime>,
    before: &boardstudio_application::AcceptedSnapshot,
    variant_id: &str,
    source_scope: &Scope,
    alive: &Rc<Cell<bool>>,
    resolver: EditResolver,
) -> Result<VariantEditWait, String> {
    // Single-ticket variant edit: no keyed map. Settlement is observed here
    // because this async flow has no parent `version` signal to settle on;
    // dropping the ticket releases its observers without cancelling the edit.
    let ticket = runtime.resolve_edit(
        "layout-matrix-variant",
        Some("matrix variant".into()),
        resolver,
    );
    for _ in 0..1_200 {
        match ticket.settlement(alive.get()) {
            Settlement::Landed { .. } => return Ok(VariantEditWait::Landed),
            Settlement::Failed { message } => return Err(message),
            Settlement::Retired => return Ok(VariantEditWait::Retired),
            Settlement::Pending => {}
        }
        if !alive.get() {
            return Ok(VariantEditWait::Retired);
        }
        let model = runtime.model();
        let Some(current) = model.accepted.as_ref() else {
            return Ok(VariantEditWait::Retired);
        };
        let current_scope = runtime.scope();
        if !variant_session_matches(
            before.session_epoch,
            variant_id,
            current.session_epoch,
            &current.document.id,
        ) || !current_scope.is_some_and(|scope| {
            scope.session_epoch == before.session_epoch
                && scope.document_id == variant_id
                && scope.board_id == source_scope.board_id
                && scope.instance_id == source_scope.instance_id
        }) {
            return Ok(VariantEditWait::Retired);
        }
        gloo_timers::future::TimeoutFuture::new(25).await;
    }
    Err("The copied matrix preset did not finish saving.".into())
}

pub(super) async fn restore_source_after_variant_failure(
    runtime: &Rc<Runtime>,
    request: &MatrixDuplicateRequest,
    variant_id: &str,
    variant_session_epoch: boardstudio_application::SessionEpoch,
    owned_clone: Option<&boardstudio_application::AcceptedSnapshot>,
    report_scope: &Rc<RefCell<Option<VariantReportOwner>>>,
) -> Result<bool, String> {
    let source_scope = &request.owner.scope;
    let source_id = source_scope.document_id.as_str();
    let source_session_epoch = source_scope.session_epoch;
    let source_token = request.snapshot_token;
    let source_revision = request.revision;
    let model = runtime.model();
    let Some(current) = model.accepted else {
        return Ok(false);
    };
    let current_scope = runtime.scope();
    let owned_clone_is_current = owned_clone.is_some_and(|owned| {
        variant_session_matches(
            owned.session_epoch,
            variant_id,
            current.session_epoch,
            &current.document.id,
        ) && current_scope.as_ref().is_some_and(|scope| {
            scope.session_epoch == owned.session_epoch
                && scope.document_id == variant_id
                && scope.board_id == source_scope.board_id
                && scope.instance_id == source_scope.instance_id
        }) && current.token == owned.token
            && current.document.revision == owned.document.revision
    });
    let failed_open_is_current = owned_clone.is_none()
        && current.session_epoch == variant_session_epoch
        && current.document.id == variant_id
        && model.lifecycle == Lifecycle::RecoveryRequired;
    let source_needs_recovery = current.session_epoch == source_session_epoch
        && current.document.id == source_id
        && model.lifecycle == Lifecycle::RecoveryRequired
        && current.token == source_token
        && current.document.revision == source_revision
        && current_scope
            .as_ref()
            .is_some_and(|scope| scope == source_scope);
    if !(owned_clone_is_current || failed_open_is_current || source_needs_recovery) {
        return Ok(false);
    }
    let Some(recovery_scope) = current_scope else {
        return Ok(false);
    };
    let restored = runtime
        .reopen_saved_if_current(
            source_id.to_owned(),
            &current,
            model.lifecycle,
            &recovery_scope,
        )
        .await?;
    let restored_scope = runtime
        .scope()
        .filter(|scope| {
            scope.session_epoch == restored.session_epoch
                && scope.document_id == restored.document.id
        })
        .ok_or_else(|| "The saved original reopened without an active project scope.".to_owned())?;
    *report_scope.borrow_mut() = Some(VariantReportOwner {
        scope: restored_scope,
        token: restored.token,
        revision: restored.document.revision,
    });
    Ok(true)
}

pub(super) async fn navigate_variant_to_source_context(
    runtime: &Rc<Runtime>,
    accepted: boardstudio_application::AcceptedSnapshot,
    variant_id: &str,
    source_scope: &Scope,
    alive: &Rc<Cell<bool>>,
) -> Result<boardstudio_application::AcceptedSnapshot, String> {
    let exact = exact_variant_snapshot(runtime, &accepted, variant_id, source_scope);
    if let Ok(current) = exact {
        return Ok(current);
    }
    let current = runtime
        .model()
        .accepted
        .ok_or_else(|| "The duplicated project is no longer active.".to_owned())?;
    if runtime.model().lifecycle != Lifecycle::Ready
        || current.session_epoch != accepted.session_epoch
        || current.document.id != variant_id
        || current.token != accepted.token
        || current.document.revision != accepted.document.revision
    {
        return Err("The duplicated project changed before its board context was selected.".into());
    }
    if !runtime.scope().is_some_and(|scope| {
        scope.session_epoch == accepted.session_epoch
            && scope.document_id == variant_id
            && scope.board_id == source_scope.board_id
            && scope.instance_id.is_none()
    }) {
        return Err("A newer board navigation superseded matrix variant setup.".into());
    }
    let operation_id = runtime.operation();
    let outcome = runtime.observe_operation(operation_id);
    runtime.submit(Event::Navigate {
        operation_id,
        board_id: source_scope.board_id.clone(),
        instance_id: source_scope.instance_id.clone(),
    });
    for _ in 0..1_200 {
        if !alive.get() {
            return Err("The matrix design variant owner closed.".into());
        }
        let current = runtime
            .model()
            .accepted
            .ok_or_else(|| "The duplicated project is no longer active.".to_owned())?;
        if current.session_epoch != accepted.session_epoch
            || current.document.id != variant_id
            || current.token != accepted.token
            || current.document.revision != accepted.document.revision
        {
            return Err("A newer project action superseded matrix variant navigation.".into());
        }
        if let Some(outcome) = outcome.borrow_mut().take() {
            if outcome != TerminalOutcome::Completed {
                return Err(format!(
                    "The copied project could not select the original board context: {outcome:?}"
                ));
            }
            return exact_variant_snapshot(runtime, &accepted, variant_id, source_scope);
        }
        gloo_timers::future::TimeoutFuture::new(25).await;
    }
    Err("The copied project board context did not settle.".into())
}

pub(super) use super::super::matrix_inspector_logic::variant_session_matches;

pub(super) async fn wait_for_matrix_project(
    runtime: &Rc<Runtime>,
    outcome: OutcomeSlot,
    expected_id: &str,
    expected_session_epoch: boardstudio_application::SessionEpoch,
    alive: &Rc<Cell<bool>>,
) -> Result<boardstudio_application::AcceptedSnapshot, String> {
    for _ in 0..1_200 {
        if !alive.get() {
            return Err("The matrix design variant owner closed.".into());
        }
        if let Some(outcome) = outcome.borrow_mut().take() {
            if outcome != TerminalOutcome::Completed {
                return Err(format!(
                    "The project operation did not complete: {outcome:?}"
                ));
            }
            let model = runtime.model();
            let accepted = model
                .accepted
                .ok_or_else(|| "The project operation was not accepted.".to_owned())?;
            if accepted.document.id != expected_id
                || accepted.session_epoch != expected_session_epoch
            {
                return Err("The project operation was superseded by another open action.".into());
            }
            if model.lifecycle != Lifecycle::Ready
                || model.durability
                    != (Durability::Saved {
                        revision: accepted.document.revision,
                    })
            {
                return Err(
                    "The project operation completed before its save was confirmed.".into(),
                );
            }
            return Ok(accepted);
        }
        gloo_timers::future::TimeoutFuture::new(25).await;
    }
    Err("The duplicated project did not finish saving.".into())
}

pub(super) fn is_reversible(document: &ProjectDoc) -> bool {
    document
        .parameters
        .get("reversibleLayout")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
}

pub(super) fn matrix_setup_preset(
    preset: MatrixPreset,
) -> crate::matrix_setup_operation::MatrixSetupPreset {
    use crate::matrix_setup_operation::MatrixSetupPreset as Setup;
    match preset {
        MatrixPreset::MxSolder => Setup::MxSolder,
        MatrixPreset::MxHotswap => Setup::MxHotswap,
        MatrixPreset::ChocSolder => Setup::ChocSolder,
        MatrixPreset::ChocHotswap => Setup::ChocHotswap,
        MatrixPreset::MxRgb => Setup::MxRgb,
        MatrixPreset::ChocRgb => Setup::ChocRgb,
        MatrixPreset::MxHotswapRgb => Setup::MxHotswapRgb,
        MatrixPreset::ChocHotswapRgb => Setup::ChocHotswapRgb,
    }
}

/// Mirrors the pinned `matrixWithPreset` recipe while retaining matrix-only fields and each
/// pre-existing cell's enabled state, local pose and non-diode/non-LED assembly membership.
pub fn matrix_with_preset(
    matrix: &Matrix,
    prepared: &Matrix,
    variant: &str,
    orientation: SwitchOrientation,
) -> Matrix {
    let old_cells = matrix
        .cells
        .iter()
        .map(|cell| ((cell.row, cell.column), cell))
        .collect::<std::collections::HashMap<_, _>>();
    let orientation_rotation = match orientation {
        SwitchOrientation::South => 0.0,
        SwitchOrientation::North => 180.0,
    };
    let cells = prepared
        .cells
        .iter()
        .map(|template| {
            let old = old_cells.get(&(template.row, template.column)).copied();
            let old_rotation = old.and_then(|cell| cell.rotation).unwrap_or(0.0);
            let old_orientation_rotation = old
                .and_then(|cell| cell.variant.as_deref())
                .filter(|variant| variant.starts_with("preset/") && variant.ends_with("/north"))
                .map_or(0.0, |_| 180.0);
            let next_rotation = old_rotation - old_orientation_rotation + orientation_rotation;
            let delta = (next_rotation - old_rotation).to_radians();
            let (sin, cos) = delta.sin_cos();
            let mut cell = template.clone();
            cell.variant = Some(variant.to_owned());
            cell.enabled = old.is_none_or(|cell| cell.enabled);
            cell.offset = old.and_then(|cell| cell.offset);
            cell.rotation = Some(next_rotation);
            cell.assemblies_local = Some(true);
            if let Some(old) = old {
                cell.assemblies.extend(
                    old.assemblies
                        .iter()
                        .filter(|assembly| assembly.id != "diode" && assembly.id != "led")
                        .map(|assembly| {
                            let mut assembly = assembly.clone();
                            assembly.offset = Vec2 {
                                x: assembly.offset.x * cos + assembly.offset.y * sin,
                                y: -assembly.offset.x * sin + assembly.offset.y * cos,
                            };
                            assembly.rotation =
                                Some(assembly.rotation.unwrap_or(0.0) - delta.to_degrees());
                            assembly.side.get_or_insert(Side::Front);
                            assembly
                        }),
                );
            }
            cell
        })
        .collect();
    let mut replacement = matrix.clone();
    replacement.definition_id = prepared.definition_id.clone();
    replacement.cells = cells;
    replacement
}
