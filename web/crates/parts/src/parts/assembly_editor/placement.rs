use super::*;
use boardstudio_web_ui_model::Workspace;

/// Shared post-await liveness gate for both assembly placement paths.
fn abort_if_view_changed(
    editor_generation: Signal<u64>,
    view_generation: u64,
    workspace: Signal<Workspace>,
    runtime: &Rc<crate::runtime::Runtime>,
    scope: &Scope,
    mut preparing: Signal<bool>,
) -> bool {
    if editor_generation() != view_generation
        || workspace() != Workspace::Parts
        || runtime.scope().as_ref() != Some(scope)
    {
        preparing.set(false);
        return true;
    }
    preparing.set(false);
    false
}

/// The accepted snapshot matching the pre-await submission identity.
fn accepted_if_current(
    runtime: &Rc<crate::runtime::Runtime>,
    workspace: Signal<Workspace>,
    scope: &Scope,
    document_id: &str,
    session_epoch: boardstudio_application::SessionEpoch,
) -> Option<AcceptedSnapshot> {
    if workspace() != Workspace::Parts || runtime.scope().as_ref() != Some(scope) {
        return None;
    }
    let accepted = runtime.model().accepted?;
    (accepted.document.id == document_id && accepted.session_epoch == session_epoch)
        .then_some(accepted)
}

/// The live accepted snapshot for a placement prepared against `source`,
/// reporting the standard reopen message when the project moved on.
fn accepted_for(
    runtime: &Rc<crate::runtime::Runtime>,
    scope: &Scope,
    source: &AcceptedSnapshot,
    mut feedback: Signal<Option<String>>,
    verb: &str,
) -> Option<AcceptedSnapshot> {
    let Some(accepted) = runtime.model().accepted else {
        feedback.set(Some("The accepted project is unavailable.".into()));
        return None;
    };
    if runtime.scope().as_ref() != Some(scope)
        || accepted.document.id != source.document.id
        || accepted.session_epoch != source.session_epoch
    {
        feedback.set(Some(format!(
            "The project changed while this assembly draft was open. Reopen it before {verb}."
        )));
        return None;
    }
    Some(accepted)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn begin_place_assembly_on_board(
    runtime: Rc<crate::runtime::Runtime>,
    mut workspace: Signal<Workspace>,
    scope: Option<Scope>,
    source: AcceptedSnapshot,
    definitions: Vec<PartDefinition>,
    draft: AssemblyDraft,
    origin_x: f64,
    origin_y: f64,
    mut preparing: Signal<bool>,
    mut selected_context: Signal<Option<super::super::super::objects::ScopedTreeContext>>,
    mut editing: Signal<Option<AssemblyDraft>>,
    on_board_placed: EventHandler<()>,
    editor_generation: Signal<u64>,
    mut feedback: Signal<Option<String>>,
) {
    let view_generation = editor_generation();
    if preparing() || workspace() != Workspace::Parts {
        return;
    }
    let Some(scope) = scope else {
        feedback.set(Some("Select a board before placing an assembly.".into()));
        return;
    };
    if draft.scope.as_ref() != Some(&scope)
        || draft.document_id != source.document.id
        || draft.session_epoch != source.session_epoch
    {
        feedback.set(Some(
            "The project or board scope changed while this assembly draft was open. Reopen it before placing.".into(),
        ));
        return;
    }
    if !origin_x.is_finite() || !origin_y.is_finite() {
        feedback.set(Some(
            "Enter finite X and Y coordinates for assembly placement.".into(),
        ));
        return;
    }
    let origin = Vec2 {
        x: origin_x,
        y: origin_y,
    };
    let Some(accepted) = accepted_for(&runtime, &scope, &source, feedback, "placing") else {
        return;
    };

    let operation_id = runtime.operation();
    let placement_id = format!("assembly-{}-{}", draft.value.id, operation_id.0);
    let (mut definitions, parts) = match super::recipes::assembly_placement(
        &accepted.document,
        &draft.value,
        &definitions,
        &draft.assets,
        origin,
        &placement_id,
    ) {
        Ok(proposal) => proposal,
        Err(error) => {
            feedback.set(Some(error));
            return;
        }
    };
    let project_id = accepted.document.id.clone();
    let session_epoch = accepted.session_epoch;
    preparing.set(true);
    feedback.set(None);
    spawn(async move {
        let normalized = async {
            for definition in &mut definitions {
                if let Some(source) = definition
                    .generator
                    .as_ref()
                    .map(|generator| generator.source.clone())
                    && crate::bundled_models::is_generator_source(&source).await?
                {
                    *definition =
                        super::super::normalize_matrix_definition(definition.clone()).await?;
                }
            }
            Ok::<_, String>(())
        }
        .await;
        if abort_if_view_changed(
            editor_generation,
            view_generation,
            workspace,
            &runtime,
            &scope,
            preparing,
        ) {
            return;
        }
        if let Err(error) = normalized {
            feedback.set(Some(error));
            return;
        }

        if accepted_if_current(&runtime, workspace, &scope, &project_id, session_epoch).is_none() {
            feedback.set(Some(
                "The project, a component definition, or board scope changed while the assembly was being prepared. Reopen the editor and try again.".into(),
            ));
            return;
        }
        // The commit is synchronous: the placed parts are exactly the board
        // parts added between this read and the accepted result.
        let placed_before: std::collections::BTreeSet<String> = runtime
            .model()
            .accepted
            .as_ref()
            .and_then(|accepted| {
                accepted
                    .document
                    .boards
                    .iter()
                    .find(|board| board.id == scope.board_id)
            })
            .map(|board| board.part_ids.iter().cloned().collect())
            .unwrap_or_default();
        match runtime.commit(
            "parts-assembly-place",
            assembly_place_resolver(
                scope.clone(),
                definitions,
                parts,
                draft.assets,
                placement_id.clone(),
            ),
        ) {
            Ok(accepted) => {
                let part_ids: Vec<String> = accepted
                    .document
                    .boards
                    .iter()
                    .find(|board| board.id == scope.board_id)
                    .into_iter()
                    .flat_map(|board| board.part_ids.iter())
                    .filter(|part_id| !placed_before.contains(*part_id))
                    .cloned()
                    .collect();
                if let Some(first) = part_ids.first() {
                    let model = runtime.model();
                    selected_context.set(
                        super::super::super::objects::context_for_part(&model, first).map(
                            |context| super::super::super::objects::ScopedTreeContext {
                                scope,
                                context,
                            },
                        ),
                    );
                    editing.set(None);
                    on_board_placed.call(());
                    workspace.set(Workspace::Layout);
                    runtime.submit(Event::SelectParts {
                        operation_id: runtime.operation(),
                        part_ids,
                        range_part_ids: Vec::new(),
                        mode: boardstudio_application::SelectionMode::Replace,
                    });
                }
            }
            Err(message) => feedback.set(Some(message)),
        }
    });
}

#[allow(clippy::too_many_arguments)]
pub(super) fn begin_apply_assembly_to_matrix(
    runtime: Rc<crate::runtime::Runtime>,
    workspace: Signal<Workspace>,
    scope: Option<Scope>,
    source: AcceptedSnapshot,
    selected_context: Signal<Option<super::super::super::objects::ScopedTreeContext>>,
    catalogue_definitions: Vec<PartDefinition>,
    assembly: AssemblyDefinition,
    mut preparing: Signal<bool>,
    editor_generation: Signal<u64>,
    mut feedback: Signal<Option<String>>,
) {
    let view_generation = editor_generation();
    if preparing() || workspace() != Workspace::Parts {
        return;
    }
    let Some(scope) = scope else {
        feedback.set(Some(
            "Open a saved board before applying an assembly to a matrix.".into(),
        ));
        return;
    };
    let Some(selected) = selected_context.peek().clone() else {
        feedback.set(Some("Select a matrix before applying an assembly.".into()));
        return;
    };
    if selected.scope != scope {
        feedback.set(Some("The selected matrix belongs to a different board. Select a matrix on the active board.".into()));
        return;
    }
    let super::super::super::objects::TreeContext::Matrix { matrix_id } = &selected.context else {
        feedback.set(Some("Select a matrix before applying an assembly.".into()));
        return;
    };
    let matrix_id = matrix_id.clone();
    let Some(accepted) = accepted_for(&runtime, &scope, &source, feedback, "applying") else {
        return;
    };
    let Some(matrix) = accepted
        .document
        .matrices
        .iter()
        .find(|matrix| matrix.id == matrix_id)
        .cloned()
    else {
        feedback.set(Some("The selected matrix is no longer available.".into()));
        return;
    };
    if matrix.board_id.as_deref() != Some(scope.board_id.as_str()) {
        feedback.set(Some("Select a matrix on the active board.".into()));
        return;
    }
    let session_epoch = scope.session_epoch;
    let document_id = scope.document_id.clone();
    let operation_id = runtime.operation();
    preparing.set(true);
    feedback.set(None);
    spawn(async move {
        let prepared = async {
            let (matrix, mut definitions) = matrix_with_assembly(
                &matrix,
                &assembly,
                &catalogue_definitions,
                &accepted.document,
                &operation_id.0.to_string(),
            )?;
            for definition in &mut definitions {
                *definition = super::super::normalize_matrix_definition(definition.clone()).await?;
            }
            Ok::<_, String>((matrix, definitions))
        }
        .await;
        if abort_if_view_changed(
            editor_generation,
            view_generation,
            workspace,
            &runtime,
            &scope,
            preparing,
        ) {
            return;
        }
        let (_matrix, definitions) = match prepared {
            Ok(prepared) => prepared,
            Err(error) => {
                feedback.set(Some(error));
                return;
            }
        };
        let still_selected = selected_context.peek().as_ref() == Some(&selected);
        if !still_selected
            || accepted_if_current(&runtime, workspace, &scope, &document_id, session_epoch)
                .is_none()
        {
            feedback.set(Some("The project or matrix selection changed while the assembly was being prepared. Reopen the editor and try again.".into()));
            return;
        }
        let result = runtime.commit(
            "parts-assembly-matrix-apply",
            assembly_matrix_resolver(
                scope,
                matrix_id,
                assembly,
                definitions,
                operation_id.0.to_string(),
            ),
        );
        feedback.set(Some(match result {
            Ok(_) => "Assembly applied to the selected matrix.".into(),
            Err(message) => message,
        }));
    });
}
