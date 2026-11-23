//! Selection resolution and tree-context lookup for the PCB tree.
use super::*;

/// Every named member of one matrix (primaries plus declared companions)
/// mapped to its projected cell.
fn member_coords(
    matrix: &Matrix,
    scene: &MatrixScene,
    is_live: &impl Fn(&str) -> bool,
) -> HashMap<String, (u32, u32)> {
    let matrix_parts: HashSet<_> = matrix.part_ids.iter().map(String::as_str).collect();
    let mut configurations = HashMap::new();
    for config in &matrix.cells {
        #[cfg(test)]
        CELL_CONFIG_VISITS.set(CELL_CONFIG_VISITS.get() + 1);
        configurations
            .entry((config.row, config.column))
            .or_insert(config);
    }
    let mut coords = HashMap::new();
    for cell in scene.cells.iter().filter(|cell| cell.enabled) {
        let Some(primary) = cell.member_id.as_deref().filter(|id| is_live(id)) else {
            continue;
        };
        if matrix_parts.contains(primary) {
            coords.insert(primary.to_owned(), (cell.row, cell.column));
        }
        if let Some(config) = configurations.get(&(cell.row, cell.column)) {
            for assembly in &config.assemblies {
                let id = format!("{primary}/{}", assembly.id);
                if is_live(&id) && matrix_parts.contains(id.as_str()) {
                    // Keep IDs owned by the document; row/column filtering still uses its primary cell.
                    coords.insert(id, (cell.row, cell.column));
                }
            }
        }
    }
    coords
}

pub fn resolve_selection(
    model: &boardstudio_application::ReadModel,
    context: &TreeContext,
) -> Option<Vec<String>> {
    let snapshot = model.accepted.as_ref()?;
    let document = boardstudio_application::resolved_document(&snapshot.document);
    let document = document.as_ref();
    let board = document
        .boards
        .iter()
        .find(|board| board.id == model.active_board_id)?;
    let live_parts = matches!(
        context,
        TreeContext::LayoutGroup { .. }
            | TreeContext::Matrix { .. }
            | TreeContext::Row { .. }
            | TreeContext::Column { .. }
    )
    .then(|| {
        let board_parts: HashSet<_> = board.part_ids.iter().map(String::as_str).collect();
        document
            .parts
            .iter()
            .filter(|part| board_parts.contains(part.id.as_str()))
            .map(|part| part.id.as_str())
            .collect::<HashSet<_>>()
    });
    let is_live = |id: &str| {
        live_parts.as_ref().map_or_else(
            || {
                board.part_ids.iter().any(|owned| owned == id)
                    && document.parts.iter().any(|part| part.id == id)
            },
            |parts| parts.contains(id),
        )
    };
    let enabled_primary =
        |item: &Matrix, scene: &MatrixScene, row: u32, column: u32| -> Option<String> {
            let id = scene
                .cells
                .iter()
                .find(|cell| cell.row == row && cell.column == column && cell.enabled)?
                .member_id
                .as_ref()?;
            (is_live(id) && item.part_ids.contains(id)).then(|| id.clone())
        };
    let matrix_ordered_members =
        |item: &Matrix, scene: &MatrixScene, filter: Option<(bool, u32)>| -> Vec<String> {
            let coord_for_id = member_coords(item, scene, &is_live);
            item.part_ids
                .iter()
                .filter(|id| {
                    coord_for_id
                        .get(id.as_str())
                        .is_some_and(|(row, column)| match filter {
                            Some((true, wanted)) => *row == wanted,
                            Some((false, wanted)) => *column == wanted,
                            None => true,
                        })
                        && is_live(id)
                })
                .cloned()
                .collect()
        };
    match context {
        TreeContext::Outline { board_id } => (board.id == *board_id).then(Vec::new),
        TreeContext::OutlineVersion {
            board_id,
            version_id,
        } => (board.id == *board_id
            && version_id.as_ref().is_none_or(|id| {
                document
                    .board_outlines
                    .iter()
                    .find(|state| state.board_id == *board_id)
                    .is_some_and(|state| state.versions.iter().any(|version| version.id == *id))
            }))
        .then(Vec::new),
        TreeContext::Bridge {
            board_id,
            bridge_id,
        } => (board.id == *board_id
            && snapshot
                .scene
                .board_outline_scenes
                .iter()
                .find(|scene| scene.board_id == *board_id)
                .is_some_and(|scene| scene.bridges.iter().any(|bridge| bridge.id == *bridge_id)))
        .then(Vec::new),
        TreeContext::Board { board_id } if board_id == &model.active_board_id => Some(Vec::new()),
        TreeContext::MountedModule {
            board_id,
            module_id,
        } if board_id == &model.active_board_id
            && document
                .modules
                .iter()
                .any(|module| module.id == *module_id && module.host_board_id == *board_id) =>
        {
            Some(Vec::new())
        }
        TreeContext::LayoutGroup {
            board_id,
            layout_ids,
        } if board_id == &model.active_board_id => {
            let current_layouts: Vec<_> = layout_ids
                .iter()
                .map(|id| {
                    document
                        .layouts
                        .iter()
                        .find(|layout| layout.id == *id && layout.board_id == board.id)
                })
                .collect();
            if layout_ids.is_empty() || current_layouts.iter().any(Option::is_none) {
                return None;
            }
            let mut ids = Vec::new();
            for layout in current_layouts.into_iter().flatten() {
                ids.extend(layout.part_ids.iter().filter(|id| is_live(id)).cloned());
                for matrix_id in &layout.matrix_ids {
                    let (matrix, scene) =
                        matrix_for_board(document, &snapshot.scene, board, matrix_id)?;
                    ids.extend(matrix_ordered_members(matrix, scene, None));
                }
            }
            let mut seen = HashSet::new();
            ids.retain(|id| seen.insert(id.clone()));
            Some(ids)
        }
        TreeContext::Matrix { matrix_id } => {
            let (matrix, scene) = matrix_for_board(document, &snapshot.scene, board, matrix_id)?;
            Some(matrix_ordered_members(matrix, scene, None))
        }
        TreeContext::Row { matrix_id, row } => {
            let (matrix, scene) = matrix_for_board(document, &snapshot.scene, board, matrix_id)?;
            if *row >= matrix.rows {
                return None;
            }
            Some(matrix_ordered_members(matrix, scene, Some((true, *row))))
        }
        TreeContext::Column { matrix_id, column } => {
            let (matrix, scene) = matrix_for_board(document, &snapshot.scene, board, matrix_id)?;
            if *column >= matrix.columns {
                return None;
            }
            Some(matrix_ordered_members(
                matrix,
                scene,
                Some((false, *column)),
            ))
        }
        TreeContext::Key {
            matrix_id,
            row,
            column,
        } => {
            let (matrix, scene) = matrix_for_board(document, &snapshot.scene, board, matrix_id)?;
            if *row >= matrix.rows || *column >= matrix.columns {
                return None;
            }
            Some(
                enabled_primary(matrix, scene, *row, *column)
                    .into_iter()
                    .collect(),
            )
        }
        TreeContext::Component {
            part_id,
            matrix_id,
            row,
            column,
            assembly_id,
        } => {
            if let Some(matrix_id) = matrix_id {
                let (matrix, scene) =
                    matrix_for_board(document, &snapshot.scene, board, matrix_id)?;
                let (row, column, assembly_id) = (*row, *column, assembly_id.as_deref());
                let (Some(row), Some(column), Some(assembly_id)) = (row, column, assembly_id)
                else {
                    return None;
                };
                if row >= matrix.rows || column >= matrix.columns {
                    return None;
                }
                let config = matrix
                    .cells
                    .iter()
                    .find(|cell| cell.row == row && cell.column == column);
                let declared = config.is_some_and(|cell| {
                    cell.assemblies
                        .iter()
                        .any(|assembly| assembly.id == assembly_id)
                });
                if !declared {
                    return None;
                }
                let primary = enabled_primary(matrix, scene, row, column);
                let actual = primary.map(|id| format!("{id}/{assembly_id}"));
                if let Some(part_id) = part_id {
                    if actual.as_ref() != Some(part_id) {
                        return None;
                    }
                } else if actual
                    .as_ref()
                    .is_some_and(|id| matrix.part_ids.contains(id) && is_live(id))
                {
                    return None;
                }
                Some(
                    actual
                        .filter(|id| matrix.part_ids.contains(id) && is_live(id))
                        .into_iter()
                        .collect(),
                )
            } else {
                let id = part_id.as_ref()?;
                if !document.parts.iter().any(|part| part.id == *id) {
                    return None;
                }
                Some(is_live(id).then(|| id.clone()).into_iter().collect())
            }
        }
        _ => None,
    }
}

pub(crate) fn context_is_resolvable(
    model: &boardstudio_application::ReadModel,
    context: &TreeContext,
) -> bool {
    let (matrix_id, row, column) = match context {
        TreeContext::Matrix { matrix_id } => (matrix_id, None, None),
        TreeContext::Row { matrix_id, row } => (matrix_id, Some(*row), None),
        TreeContext::Column { matrix_id, column } => (matrix_id, None, Some(*column)),
        TreeContext::Key {
            matrix_id,
            row,
            column,
        } => (matrix_id, Some(*row), Some(*column)),
        _ => return resolve_selection(model, context).is_some(),
    };
    let Some(snapshot) = model.accepted.as_ref() else {
        return false;
    };
    let document = boardstudio_application::resolved_document(&snapshot.document);
    let Some(board) = document
        .boards
        .iter()
        .find(|board| board.id == model.active_board_id)
    else {
        return false;
    };
    let Some((matrix, _)) = matrix_for_board(&document, &snapshot.scene, board, matrix_id) else {
        return false;
    };
    row.is_none_or(|row| row < matrix.rows) && column.is_none_or(|column| column < matrix.columns)
}

pub fn context_for_part(
    model: &boardstudio_application::ReadModel,
    part_id: &str,
) -> Option<TreeContext> {
    #[cfg(test)]
    CONTEXT_LOOKUPS.set(CONTEXT_LOOKUPS.get() + 1);
    let snapshot = model.accepted.as_ref()?;
    let document = boardstudio_application::resolved_document(&snapshot.document);
    let document = document.as_ref();
    let board = document
        .boards
        .iter()
        .find(|board| board.id == model.active_board_id)?;
    if !board.part_ids.iter().any(|id| id == part_id)
        || !document.parts.iter().any(|part| part.id == part_id)
    {
        return None;
    }
    let board_parts: HashSet<_> = board.part_ids.iter().map(String::as_str).collect();
    for matrix in visible_matrices(document, &board.id, &board_parts) {
        if !matrix.part_ids.iter().any(|id| id == part_id) {
            continue;
        }
        let Some(scene) = snapshot
            .scene
            .matrix_scenes
            .iter()
            .find(|scene| scene.matrix_id == matrix.id)
        else {
            continue;
        };
        for cell in scene.cells.iter().filter(|cell| cell.enabled) {
            let Some(primary) = cell.member_id.as_deref() else {
                continue;
            };
            if primary == part_id {
                return Some(TreeContext::Key {
                    matrix_id: matrix.id.clone(),
                    row: cell.row,
                    column: cell.column,
                });
            }
            // Only the matching primary can own this companion; avoid scanning every cell's assemblies.
            let Some(assembly_id) = part_id
                .strip_prefix(primary)
                .and_then(|suffix| suffix.strip_prefix('/'))
            else {
                continue;
            };
            if matrix
                .cells
                .iter()
                .find(|item| item.row == cell.row && item.column == cell.column)
                .is_some_and(|config| {
                    config
                        .assemblies
                        .iter()
                        .any(|assembly| assembly.id == assembly_id)
                })
            {
                return Some(TreeContext::Component {
                    part_id: Some(part_id.to_owned()),
                    matrix_id: Some(matrix.id.clone()),
                    row: Some(cell.row),
                    column: Some(cell.column),
                    assembly_id: Some(assembly_id.to_owned()),
                });
            }
        }
    }
    if visible_matrices(document, &board.id, &board_parts)
        .iter()
        .any(|matrix| matrix.part_ids.iter().any(|id| id == part_id))
    {
        return None;
    }
    Some(TreeContext::Component {
        part_id: Some(part_id.to_owned()),
        matrix_id: None,
        row: None,
        column: None,
        assembly_id: None,
    })
}

pub(crate) fn eligible_live_ids(model: &boardstudio_application::ReadModel) -> Vec<String> {
    let Some(snapshot) = model.accepted.as_ref() else {
        return Vec::new();
    };
    let document = boardstudio_application::resolved_document(&snapshot.document);
    let document = document.as_ref();
    let Some(board) = document
        .boards
        .iter()
        .find(|board| board.id == model.active_board_id)
    else {
        return Vec::new();
    };
    let board_parts = board.part_ids.iter().map(String::as_str).collect();
    let live: HashSet<_> = document.parts.iter().map(|part| part.id.as_str()).collect();
    let matrices = visible_matrices(document, &board.id, &board_parts);
    let owned: HashSet<_> = matrices
        .iter()
        .flat_map(|matrix| matrix.part_ids.iter().map(String::as_str))
        .collect();
    let mut eligible: HashSet<String> = HashSet::new();
    for matrix in matrices {
        let Some(scene) = snapshot
            .scene
            .matrix_scenes
            .iter()
            .find(|scene| scene.matrix_id == matrix.id)
        else {
            continue;
        };
        // Eligibility projects every member the scene names, live or not: the
        // board-level filter below still removes dead ids, but a live companion
        // of a removed primary stays eligible exactly as `context_for_part`
        // resolves it.
        eligible.extend(member_coords(matrix, scene, &(|_: &str| true)).into_keys());
    }
    board
        .part_ids
        .iter()
        .filter(|id| {
            live.contains(id.as_str())
                && (!owned.contains(id.as_str()) || eligible.contains(id.as_str()))
        })
        .cloned()
        .collect()
}

/// Build the explicit component/Part Inspector context used by a finding action.
/// Ordinary hit testing still calls `context_for_part` and therefore keeps primary
/// matrix members in Key mode; a finding has already resolved a Part target and
/// React deliberately opens that part's component Inspector instead.
pub fn component_context_for_finding_part(
    model: &boardstudio_application::ReadModel,
    part_id: &str,
) -> Option<TreeContext> {
    let context = context_for_part(model, part_id)?;
    component_context_for_explicit_part(part_id, &context)
}

pub(crate) fn component_context_for_explicit_part(
    part_id: &str,
    ordinary: &TreeContext,
) -> Option<TreeContext> {
    matches!(
        ordinary,
        TreeContext::Key { .. } | TreeContext::Component { .. }
    )
    .then(|| TreeContext::Component {
        part_id: Some(part_id.to_owned()),
        matrix_id: None,
        row: None,
        column: None,
        assembly_id: None,
    })
}

pub fn context_for_cell(
    model: &boardstudio_application::ReadModel,
    matrix_id: &str,
    row: u32,
    column: u32,
) -> Option<TreeContext> {
    let snapshot = model.accepted.as_ref()?;
    let document = boardstudio_application::resolved_document(&snapshot.document);
    let document = document.as_ref();
    let board = document
        .boards
        .iter()
        .find(|board| board.id == model.active_board_id)?;
    let board_parts: HashSet<_> = board.part_ids.iter().map(String::as_str).collect();
    let matrix = document
        .matrices
        .iter()
        .find(|matrix| matrix.id == matrix_id)?;
    let scene = snapshot
        .scene
        .matrix_scenes
        .iter()
        .find(|scene| scene.matrix_id == matrix_id)?;
    if !visible_matrices(document, &board.id, &board_parts)
        .iter()
        .any(|visible| visible.id == matrix.id)
        || row >= matrix.rows
        || column >= matrix.columns
        || !scene
            .cells
            .iter()
            .any(|cell| cell.row == row && cell.column == column)
    {
        return None;
    }
    Some(TreeContext::Key {
        matrix_id: matrix.id.clone(),
        row,
        column,
    })
}

pub fn context_label(
    model: &boardstudio_application::ReadModel,
    context: &TreeContext,
) -> Option<String> {
    let snapshot = model.accepted.as_ref()?;
    let document = boardstudio_application::resolved_document(&snapshot.document);
    let document = document.as_ref();
    match context {
        TreeContext::Outline { .. } | TreeContext::OutlineVersion { .. } => {
            Some("Board outline".into())
        }
        TreeContext::Bridge { bridge_id, .. } => Some(format!("Bridge {bridge_id}")),
        TreeContext::Board { board_id } => document
            .boards
            .iter()
            .find(|board| board.id == *board_id)
            .map(|board| board.name.clone()),
        TreeContext::MountedModule { module_id, .. } => document
            .modules
            .iter()
            .find(|module| module.id == *module_id)
            .and_then(|module| {
                document
                    .module_definitions
                    .iter()
                    .find(|definition| definition.id == module.definition_id)
                    .map(|definition| definition.name.clone())
            }),
        TreeContext::LayoutGroup { layout_ids, .. } => Some(if layout_ids.len() > 1 {
            "Linked halves".into()
        } else {
            "Layout".into()
        }),
        TreeContext::Matrix { matrix_id } => document
            .matrices
            .iter()
            .find(|matrix| matrix.id == *matrix_id)
            .map(|matrix| matrix_display_name(document, matrix)),
        TreeContext::Row { row, .. } => Some(format!("Row {}", row + 1)),
        TreeContext::Column { column, .. } => Some(format!("Column {}", column + 1)),
        TreeContext::Key {
            matrix_id,
            row,
            column,
        } => {
            let cell = snapshot
                .scene
                .matrix_scenes
                .iter()
                .find(|scene| scene.matrix_id == *matrix_id)?
                .cells
                .iter()
                .find(|cell| cell.row == *row && cell.column == *column)?;
            let member = cell.enabled.then_some(cell.member_id.as_deref()).flatten();
            Some(
                member
                    .and_then(|id| {
                        document
                            .parts
                            .iter()
                            .find(|part| part.id == id)
                            .map(|part| part.reference.clone())
                    })
                    .unwrap_or_else(|| format!("Empty slot · Key {}.{}", column + 1, row + 1)),
            )
        }
        TreeContext::Component {
            part_id,
            matrix_id,
            row,
            column,
            assembly_id,
        } => {
            if let Some(part_id) = part_id
                && let Some(part) = document.parts.iter().find(|part| part.id == *part_id)
            {
                return Some(part.reference.clone());
            }
            if let (Some(_), Some(row), Some(column), Some(assembly_id)) =
                (matrix_id, row, column, assembly_id)
            {
                return Some(format!(
                    "Cell component · Key {}.{} · {assembly_id}",
                    column + 1,
                    row + 1
                ));
            }
            None
        }
    }
}

/// Host-compatible scope currency shared by every presentation crate. The wasm
/// selection module owns the Dioxus adapter, but this predicate is the single
/// production implementation used on both targets and in native tests.
/// Full behavior: the accepted board, instance, session and document still match
/// `scope`, and the tree context still resolves against the accepted document.
/// Stale scopes and unresolvable selections both return `false`.
pub fn active_board_scope_matches(
    model: &boardstudio_application::ReadModel,
    scope: &boardstudio_application::Scope,
) -> bool {
    model.active_board_id == scope.board_id
        && model.active_instance_id == scope.instance_id
        && model.accepted.as_ref().is_some_and(|snapshot| {
            snapshot.session_epoch == scope.session_epoch
                && snapshot.document.id == scope.document_id
        })
}

/// Host-compatible selection currency shared by every presentation crate. See
/// [`active_board_scope_matches`]: scope currency plus tree resolvability.
pub fn selection_context_is_current(
    model: &boardstudio_application::ReadModel,
    scope: &boardstudio_application::Scope,
    context: &TreeContext,
) -> bool {
    active_board_scope_matches(model, scope) && context_is_resolvable(model, context)
}
