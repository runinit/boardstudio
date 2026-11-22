use super::*;

#[allow(clippy::too_many_arguments)]
pub(super) fn begin_place_assembly_on_board(
    runtime: Rc<crate::runtime::Runtime>,
    mut workspace: Signal<&'static str>,
    scope: Option<Scope>,
    source: AcceptedSnapshot,
    definitions: Vec<PartDefinition>,
    draft: AssemblyDraft,
    origin_x: String,
    origin_y: String,
    mut preparing: Signal<bool>,
    mut selected_context: Signal<Option<super::super::super::objects::ScopedTreeContext>>,
    mut editing: Signal<Option<AssemblyDraft>>,
    on_board_placed: EventHandler<()>,
    editor_generation: Signal<u64>,
    mut feedback: Signal<Option<String>>,
) {
    let view_generation = editor_generation();
    if preparing() || workspace() != "Parts" {
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
    let origin = match (
        origin_x.trim().parse::<f64>(),
        origin_y.trim().parse::<f64>(),
    ) {
        (Ok(x), Ok(y)) if x.is_finite() && y.is_finite() => Vec2 { x, y },
        _ => {
            feedback.set(Some(
                "Enter finite X and Y coordinates for assembly placement.".into(),
            ));
            return;
        }
    };
    let model = runtime.model();
    let Some(accepted) = model.accepted.as_ref() else {
        feedback.set(Some("The accepted project is unavailable.".into()));
        return;
    };
    if runtime.scope().as_ref() != Some(&scope)
        || accepted.document.id != source.document.id
        || accepted.session_epoch != source.session_epoch
    {
        feedback.set(Some(
            "The project changed while this assembly draft was open. Reopen it before placing."
                .into(),
        ));
        return;
    }

    let operation_id = runtime.operation();
    let placement_id = format!("assembly-{}-{}", draft.value.id, operation_id.0);
    let (mut proposal, part_ids) = match document_with_assembly(
        &accepted.document,
        &draft.value,
        &definitions,
        &draft.assets,
        &scope.board_id,
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
    let definition_prefix = format!("{placement_id}/definition/");
    preparing.set(true);
    feedback.set(None);
    spawn(async move {
        let normalized = async {
            for definition in proposal
                .definitions
                .iter_mut()
                .filter(|definition| definition.id.starts_with(&definition_prefix))
            {
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
        if editor_generation() != view_generation
            || workspace() != "Parts"
            || runtime.scope().as_ref() != Some(&scope)
        {
            preparing.set(false);
            return;
        }
        preparing.set(false);
        if let Err(error) = normalized {
            feedback.set(Some(error));
            return;
        }

        let current = runtime.model();
        let Some(latest) = current.accepted.as_ref() else {
            feedback.set(Some("The accepted project is unavailable.".into()));
            return;
        };
        if workspace() != "Parts"
            || runtime.scope().as_ref() != Some(&scope)
            || latest.document.id != project_id
            || latest.session_epoch != session_epoch
        {
            feedback.set(Some(
                "The project, a component definition, or board scope changed while the assembly was being prepared. Reopen the editor and try again.".into(),
            ));
            return;
        }
        match runtime.commit(
            "parts-assembly-place",
            assembly_place_resolver(
                scope.clone(),
                proposal,
                draft.assets.clone(),
                placement_id.clone(),
                part_ids,
            ),
        ) {
            Ok(accepted) => {
                let part_ids =
                    placed_parts_for_submission(&accepted.document, &scope.board_id, &placement_id);
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
                    workspace.set("Layout");
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
    workspace: Signal<&'static str>,
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
    if preparing() || workspace() != "Parts" {
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
    let model = runtime.model();
    let Some(accepted) = model.accepted.as_ref() else {
        feedback.set(Some("The accepted project is unavailable.".into()));
        return;
    };
    if runtime.scope().as_ref() != Some(&scope)
        || accepted.document.id != source.document.id
        || accepted.session_epoch != source.session_epoch
    {
        feedback.set(Some(
            "The project changed while this assembly draft was open. Reopen it before applying."
                .into(),
        ));
        return;
    }
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
    let accepted = accepted.clone();
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
        if editor_generation() != view_generation
            || workspace() != "Parts"
            || runtime.scope().as_ref() != Some(&scope)
        {
            preparing.set(false);
            return;
        }
        preparing.set(false);
        let (_matrix, definitions) = match prepared {
            Ok(prepared) => prepared,
            Err(error) => {
                feedback.set(Some(error));
                return;
            }
        };
        let current_model = runtime.model();
        let still_selected = selected_context.peek().as_ref() == Some(&selected);
        if workspace() != "Parts"
            || runtime.scope().as_ref() != Some(&scope)
            || !still_selected
            || !current_model.accepted.as_ref().is_some_and(|current| {
                current.session_epoch == session_epoch && current.document.id == document_id
            })
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
