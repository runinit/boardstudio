use boardstudio_core::model::{Board, Layout, Matrix, MatrixScene, Part, ProjectDoc, SceneDelta};
use std::collections::{BTreeSet, HashMap, HashSet};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum TreeContext {
    Board {
        board_id: String,
    },
    LayoutGroup {
        board_id: String,
        layout_ids: Vec<String>,
    },
    Matrix {
        matrix_id: String,
    },
    Row {
        matrix_id: String,
        row: u32,
    },
    Column {
        matrix_id: String,
        column: u32,
    },
    Key {
        matrix_id: String,
        row: u32,
        column: u32,
    },
    Component {
        part_id: Option<String>,
        matrix_id: Option<String>,
        row: Option<u32>,
        column: Option<u32>,
        assembly_id: Option<String>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum TreeKind {
    Board,
    Layout,
    Matrix,
    Row,
    Column,
    Key,
    Component,
    Components,
    HalfGroup,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct TreeItem {
    pub id: String,
    pub label: String,
    pub detail: Option<String>,
    pub level: usize,
    pub kind: TreeKind,
    pub context: Option<TreeContext>,
    pub expanded: Option<bool>,
    pub primary_id: Option<String>,
    pub expandable: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Grouping {
    Column,
    Row,
}

impl Grouping {
    pub fn from_storage(value: Option<String>) -> Self {
        if value.as_deref() == Some("row") {
            Self::Row
        } else {
            Self::Column
        }
    }
}

pub(super) fn visible_matrices<'a>(
    document: &'a ProjectDoc,
    board_id: &str,
    board_parts: &HashSet<&str>,
) -> Vec<&'a Matrix> {
    document
        .matrices
        .iter()
        .filter(|matrix| {
            if let Some(owner) = matrix.board_id.as_deref() {
                owner == board_id
            } else {
                matrix
                    .part_ids
                    .iter()
                    .any(|id| board_parts.contains(id.as_str()))
                    || (matrix.part_ids.is_empty() && document.boards.len() <= 1)
            }
        })
        .collect()
}

pub(super) fn build_tree(
    document: &ProjectDoc,
    board_id: &str,
    grouping: Grouping,
    expanded: &BTreeSet<String>,
    matrix_scenes: &[MatrixScene],
) -> Vec<TreeItem> {
    let Some(board) = document.boards.iter().find(|board| board.id == board_id) else {
        return Vec::new();
    };
    let board_parts: HashSet<_> = board.part_ids.iter().map(String::as_str).collect();
    let live_parts: HashMap<_, _> = document
        .parts
        .iter()
        .filter(|part| board_parts.contains(part.id.as_str()))
        .map(|part| (part.id.as_str(), part))
        .collect();
    let definitions: HashMap<_, _> = document
        .definitions
        .iter()
        .map(|definition| (definition.id.as_str(), definition.name.as_str()))
        .collect();
    let matrices = visible_matrices(document, board_id, &board_parts);
    let scenes: HashMap<_, _> = matrices
        .iter()
        .filter_map(|matrix| {
            matrix_scenes
                .iter()
                .find(|scene| scene.matrix_id == matrix.id)
                .map(|scene| (matrix.id.as_str(), scene))
        })
        .collect();
    let layouts: Vec<_> = document
        .layouts
        .iter()
        .filter(|layout| {
            layout.board_id == board_id
                && matrices.iter().any(|matrix| matrix.id == layout.matrix_id)
        })
        .collect();
    let mut tree = Vec::new();

    let board_key = format!("board:{board_id}");
    let board_open = expanded.contains(&board_key);
    let live_board_count = board
        .part_ids
        .iter()
        .filter(|id| live_parts.contains_key(id.as_str()))
        .count();
    tree.push(TreeItem {
        id: board_key,
        label: board.name.clone(),
        detail: Some(format!("{live_board_count} parts")),
        level: 0,
        kind: TreeKind::Board,
        context: Some(TreeContext::Board {
            board_id: board_id.to_owned(),
        }),
        expanded: Some(board_open),
        primary_id: None,
        expandable: true,
    });
    if !board_open {
        return tree;
    }

    let matrix_part_ids: HashSet<&str> = matrices
        .iter()
        .flat_map(|matrix| matrix.part_ids.iter().map(String::as_str))
        .collect();
    let mut components_by_layout: HashMap<&str, Vec<&Part>> = HashMap::new();
    let mut standalone = Vec::new();
    for part in &document.parts {
        if !board_parts.contains(part.id.as_str()) || matrix_part_ids.contains(part.id.as_str()) {
            continue;
        }
        if let Some(layout) = layouts
            .iter()
            .find(|layout| layout.part_ids.contains(&part.id))
        {
            components_by_layout
                .entry(layout.id.as_str())
                .or_default()
                .push(part);
        } else {
            standalone.push(part);
        }
    }
    let owner_layout: HashMap<&str, &Layout> = layouts
        .iter()
        .map(|layout| (layout.matrix_id.as_str(), *layout))
        .collect();
    let has_split = layouts.iter().any(|layout| layout.mirror_link.is_some())
        || layouts.iter().any(|layout| {
            layouts.iter().any(|other| {
                other
                    .mirror_link
                    .as_ref()
                    .is_some_and(|link| link.source_id == layout.id)
            })
        });
    let split_x = layouts
        .iter()
        .find_map(|layout| layout.mirror_link.as_ref().map(|link| link.axis_x));
    let halves: Vec<(&str, Vec<&Layout>)> = if has_split {
        let axis = split_x.unwrap_or_default();
        vec![
            (
                "Left half",
                layouts
                    .iter()
                    .copied()
                    .filter(|layout| matrix_origin_x(&matrices, &layout.matrix_id) < axis)
                    .collect(),
            ),
            (
                "Right half",
                layouts
                    .iter()
                    .copied()
                    .filter(|layout| matrix_origin_x(&matrices, &layout.matrix_id) >= axis)
                    .collect(),
            ),
        ]
    } else {
        vec![("", layouts.clone())]
    };

    let unowned: Vec<_> = matrices
        .iter()
        .copied()
        .filter(|matrix| !owner_layout.contains_key(matrix.id.as_str()))
        .collect();
    if !unowned.is_empty() || !standalone.is_empty() || layouts.is_empty() {
        let id = format!("layout:{board_id}");
        let is_open = expanded.contains(&id);
        tree.push(TreeItem {
            id,
            label: "Layout".into(),
            detail: None,
            level: 1,
            kind: TreeKind::Layout,
            context: None,
            expanded: Some(is_open),
            primary_id: None,
            expandable: true,
        });
        if is_open {
            for matrix in unowned {
                append_matrix(
                    &mut tree,
                    document,
                    matrix,
                    scenes.get(matrix.id.as_str()).copied(),
                    &live_parts,
                    &definitions,
                    grouping,
                    expanded,
                    2,
                );
            }
            if layouts.is_empty() {
                for part in &standalone {
                    append_component(&mut tree, part, &definitions, 2);
                }
            }
        }
    }

    for (half_label, half_layouts) in halves {
        let half_key = format!("half-group:{board_id}:{half_label}");
        let half_open = half_label.is_empty() || !expanded.contains(&half_key);
        if !half_label.is_empty() && !half_layouts.is_empty() {
            tree.push(TreeItem {
                id: half_key,
                label: half_label.into(),
                detail: None,
                level: 1,
                kind: TreeKind::HalfGroup,
                context: Some(TreeContext::LayoutGroup {
                    board_id: board_id.to_owned(),
                    layout_ids: half_layouts
                        .iter()
                        .map(|layout| layout.id.clone())
                        .collect(),
                }),
                expanded: Some(half_open),
                primary_id: None,
                expandable: true,
            });
        }
        if !half_open {
            continue;
        }
        let half_level = usize::from(!half_label.is_empty());
        let components: Vec<_> = half_layouts
            .iter()
            .flat_map(|layout| {
                components_by_layout
                    .get(layout.id.as_str())
                    .into_iter()
                    .flatten()
                    .copied()
            })
            .collect();
        append_components_group(
            &mut tree,
            &components,
            &definitions,
            board_id,
            half_label,
            expanded,
            if half_level == 0 { 1 } else { 2 },
        );
        for layout in half_layouts {
            let layout_key = format!("half:{}", layout.id);
            let is_open = expanded.contains(&layout_key);
            let linked = layout.mirror_link.is_some()
                || layouts.iter().any(|other| {
                    other
                        .mirror_link
                        .as_ref()
                        .is_some_and(|link| link.source_id == layout.id)
                });
            tree.push(TreeItem {
                id: layout_key.clone(),
                label: layout.name.clone(),
                detail: Some(if linked { "Linked" } else { "Independent" }.into()),
                level: if half_level == 0 { 1 } else { 2 },
                kind: TreeKind::Layout,
                context: Some(TreeContext::Matrix {
                    matrix_id: layout.matrix_id.clone(),
                }),
                expanded: Some(is_open),
                primary_id: None,
                expandable: true,
            });
            if is_open {
                let child_level = if half_level == 0 { 2 } else { 3 };
                if let Some(matrix) = matrices
                    .iter()
                    .copied()
                    .find(|matrix| matrix.id == layout.matrix_id)
                {
                    append_matrix(
                        &mut tree,
                        document,
                        matrix,
                        scenes.get(matrix.id.as_str()).copied(),
                        &live_parts,
                        &definitions,
                        grouping,
                        expanded,
                        child_level,
                    );
                }
            }
        }
    }
    tree
}

fn append_components_group(
    tree: &mut Vec<TreeItem>,
    parts: &[&Part],
    definitions: &HashMap<&str, &str>,
    board_id: &str,
    label_suffix: &str,
    expanded: &BTreeSet<String>,
    level: usize,
) {
    if parts.is_empty() {
        return;
    }
    let id = format!(
        "components:{board_id}:{}",
        if label_suffix.is_empty() {
            "layout"
        } else {
            label_suffix
        }
    );
    let is_open = !expanded.contains(&id);
    tree.push(TreeItem {
        id,
        label: "Components".into(),
        detail: Some(format!("{} parts", parts.len())),
        level,
        kind: TreeKind::Components,
        context: None,
        expanded: Some(is_open),
        primary_id: None,
        expandable: true,
    });
    if is_open {
        for part in parts {
            append_component(tree, part, definitions, level + 1);
        }
    }
}

fn append_component(
    tree: &mut Vec<TreeItem>,
    part: &Part,
    definitions: &HashMap<&str, &str>,
    level: usize,
) {
    let name = definitions
        .get(part.definition_id.as_str())
        .copied()
        .unwrap_or("Part");
    tree.push(TreeItem {
        id: format!("component:{}", part.id),
        label: part.reference.clone(),
        detail: Some(format!(
            "{name} · {:.1}, {:.1}",
            part.pose.at.x, part.pose.at.y
        )),
        level,
        kind: TreeKind::Component,
        context: Some(TreeContext::Component {
            part_id: Some(part.id.clone()),
            matrix_id: None,
            row: None,
            column: None,
            assembly_id: None,
        }),
        expanded: None,
        primary_id: Some(part.id.clone()),
        expandable: false,
    });
}

fn matrix_origin_x(matrices: &[&Matrix], id: &str) -> f64 {
    matrices
        .iter()
        .find(|matrix| matrix.id == id)
        .map_or(0.0, |matrix| matrix.origin.x)
}

#[allow(clippy::too_many_arguments)]
fn append_matrix(
    tree: &mut Vec<TreeItem>,
    document: &ProjectDoc,
    matrix: &Matrix,
    scene: Option<&MatrixScene>,
    live_parts: &HashMap<&str, &Part>,
    definitions: &HashMap<&str, &str>,
    grouping: Grouping,
    expanded: &BTreeSet<String>,
    level: usize,
) {
    let projected = scene
        .map(|scene| scene.cells.as_slice())
        .unwrap_or_default();
    let actual_member = |row: u32, column: u32| {
        projected
            .iter()
            .find(|cell| cell.row == row && cell.column == column && cell.enabled)
            .and_then(|cell| cell.member_id.as_deref())
            .filter(|id| live_parts.contains_key(id))
    };
    let assembly_member = |row: u32, column: u32, assembly_id: &str| {
        let primary = actual_member(row, column)?;
        let candidate = format!("{primary}/{assembly_id}");
        (matrix.part_ids.contains(&candidate) && live_parts.contains_key(candidate.as_str()))
            .then_some(candidate)
    };
    let primary_ids = projected
        .iter()
        .filter(|cell| cell.enabled)
        .filter_map(|cell| cell.member_id.as_deref())
        .filter(|id| live_parts.contains_key(id))
        .collect::<Vec<_>>();
    let index = document
        .matrices
        .iter()
        .position(|candidate| candidate.id == matrix.id)
        .unwrap_or(0)
        + 1;
    let item_id = format!("matrix:{}", matrix.id);
    let is_open = expanded.contains(&item_id);
    tree.push(TreeItem {
        id: item_id,
        label: matrix
            .name
            .as_deref()
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(str::to_owned)
            .unwrap_or_else(|| format!("Matrix {index}")),
        detail: Some(format!("{} keys", primary_ids.len())),
        level,
        kind: TreeKind::Matrix,
        context: Some(TreeContext::Matrix {
            matrix_id: matrix.id.clone(),
        }),
        expanded: Some(is_open),
        primary_id: None,
        expandable: true,
    });
    if !is_open || matrix.rows == 0 || matrix.columns == 0 {
        return;
    }

    let (group_count, key_count) = match grouping {
        Grouping::Column => (matrix.columns, matrix.rows),
        Grouping::Row => (matrix.rows, matrix.columns),
    };
    for group in 0..group_count {
        let (kind, label, context, prefix) = match grouping {
            Grouping::Column => (
                TreeKind::Column,
                format!("Column {}", group + 1),
                TreeContext::Column {
                    matrix_id: matrix.id.clone(),
                    column: group,
                },
                "column",
            ),
            Grouping::Row => (
                TreeKind::Row,
                format!("Row {}", group + 1),
                TreeContext::Row {
                    matrix_id: matrix.id.clone(),
                    row: group,
                },
                "row",
            ),
        };
        let id = format!("{prefix}:{}:{group}", matrix.id);
        let is_open = expanded.contains(&id);
        let key_count_live = (0..key_count)
            .filter(|index| {
                let (row, column) = match grouping {
                    Grouping::Column => (*index, group),
                    Grouping::Row => (group, *index),
                };
                actual_member(row, column).is_some()
            })
            .count();
        tree.push(TreeItem {
            id: id.clone(),
            label,
            detail: Some(format!("{key_count_live} keys")),
            level: level + 1,
            kind,
            context: Some(context),
            expanded: Some(is_open),
            primary_id: None,
            expandable: true,
        });
        if !is_open {
            continue;
        }
        for index in 0..key_count {
            let (row, column) = match grouping {
                Grouping::Column => (index, group),
                Grouping::Row => (group, index),
            };
            let cell = matrix
                .cells
                .iter()
                .find(|cell| cell.row == row && cell.column == column);
            let enabled = cell.is_none_or(|cell| cell.enabled);
            let primary = enabled.then(|| actual_member(row, column)).flatten();
            let assemblies: Vec<_> = cell
                .into_iter()
                .flat_map(|cell| cell.assemblies.iter())
                .map(|assembly| {
                    (
                        assembly,
                        enabled
                            .then(|| assembly_member(row, column, &assembly.id))
                            .flatten(),
                    )
                })
                .collect();
            let key_id = format!("key:{}:{row}:{column}", matrix.id);
            let key_open = expanded.contains(&key_id);
            tree.push(TreeItem {
                id: key_id,
                label: format!("Key {}.{}", column + 1, row + 1),
                detail: Some(
                    primary
                        .and_then(|id| live_parts.get(id).map(|part| part.reference.clone()))
                        .unwrap_or_else(|| "Empty slot".into()),
                ),
                level: level + 2,
                kind: TreeKind::Key,
                context: Some(TreeContext::Key {
                    matrix_id: matrix.id.clone(),
                    row,
                    column,
                }),
                expanded: Some(key_open),
                primary_id: primary.map(str::to_owned),
                expandable: !assemblies.is_empty(),
            });
            if key_open {
                for (assembly, actual) in assemblies {
                    let definition = actual
                        .as_deref()
                        .and_then(|id| live_parts.get(id))
                        .and_then(|part| definitions.get(part.definition_id.as_str()).copied())
                        .or_else(|| definitions.get(assembly.definition_id.as_str()).copied())
                        .unwrap_or("Component");
                    tree.push(TreeItem {
                        id: format!(
                            "component:matrix/{}/r{row}c{column}/{}",
                            matrix.id, assembly.id
                        ),
                        label: definition.into(),
                        detail: Some(
                            if assembly.id == "diode" {
                                "Automatic companion"
                            } else {
                                "Cell component"
                            }
                            .into(),
                        ),
                        level: level + 3,
                        kind: TreeKind::Component,
                        context: Some(TreeContext::Component {
                            part_id: actual.clone(),
                            matrix_id: Some(matrix.id.clone()),
                            row: Some(row),
                            column: Some(column),
                            assembly_id: Some(assembly.id.clone()),
                        }),
                        expanded: None,
                        primary_id: actual,
                        expandable: false,
                    });
                }
            }
        }
    }
}

fn matrix_for_board<'a>(
    document: &'a ProjectDoc,
    scene: &'a SceneDelta,
    board: &Board,
    matrix_id: &str,
) -> Option<(&'a Matrix, &'a MatrixScene)> {
    let item = document
        .matrices
        .iter()
        .find(|matrix| matrix.id == matrix_id)?;
    let board_parts: HashSet<_> = board.part_ids.iter().map(String::as_str).collect();
    if !visible_matrices(document, &board.id, &board_parts)
        .iter()
        .any(|matrix| matrix.id == matrix_id)
    {
        return None;
    }
    let projected = scene
        .matrix_scenes
        .iter()
        .find(|scene| scene.matrix_id == matrix_id)?;
    Some((item, projected))
}

pub(super) fn resolve_selection(
    model: &boardstudio_application::ReadModel,
    context: &TreeContext,
) -> Option<Vec<String>> {
    let snapshot = model.accepted.as_ref()?;
    let document = &snapshot.document;
    let board = document
        .boards
        .iter()
        .find(|board| board.id == model.active_board_id)?;
    let board_parts: HashSet<_> = board.part_ids.iter().map(String::as_str).collect();
    let live_parts: HashSet<_> = document
        .parts
        .iter()
        .filter(|part| board_parts.contains(part.id.as_str()))
        .map(|part| part.id.as_str())
        .collect();
    let enabled_primary =
        |item: &Matrix, scene: &MatrixScene, row: u32, column: u32| -> Option<String> {
            let id = scene
                .cells
                .iter()
                .find(|cell| cell.row == row && cell.column == column && cell.enabled)?
                .member_id
                .as_ref()?;
            (live_parts.contains(id.as_str()) && item.part_ids.contains(id)).then(|| id.clone())
        };
    let matrix_ordered_members =
        |item: &Matrix, scene: &MatrixScene, filter: Option<(bool, u32)>| -> Vec<String> {
            let mut coord_for_id: HashMap<String, (u32, u32)> = HashMap::new();
            for cell in scene.cells.iter().filter(|cell| cell.enabled) {
                let Some(primary) = cell
                    .member_id
                    .as_deref()
                    .filter(|id| live_parts.contains(id))
                else {
                    continue;
                };
                if item.part_ids.iter().any(|id| id == primary) {
                    coord_for_id.insert(primary.to_owned(), (cell.row, cell.column));
                }
                if let Some(config) = item
                    .cells
                    .iter()
                    .find(|config| config.row == cell.row && config.column == cell.column)
                {
                    for assembly in &config.assemblies {
                        let id = format!("{primary}/{}", assembly.id);
                        if live_parts.contains(id.as_str()) && item.part_ids.contains(&id) {
                            // Keep IDs owned by the document; row/column filtering still uses its primary cell.
                            coord_for_id.insert(id, (cell.row, cell.column));
                        }
                    }
                }
            }
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
                        && live_parts.contains(id.as_str())
                })
                .cloned()
                .collect()
        };
    match context {
        TreeContext::Board { board_id } if board_id == &model.active_board_id => Some(Vec::new()),
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
                ids.extend(
                    layout
                        .part_ids
                        .iter()
                        .filter(|id| live_parts.contains(id.as_str()))
                        .cloned(),
                );
                let (matrix, scene) =
                    matrix_for_board(document, &snapshot.scene, board, &layout.matrix_id)?;
                ids.extend(matrix_ordered_members(matrix, scene, None));
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
                } else if actual.as_ref().is_some_and(|id| {
                    matrix.part_ids.contains(id) && live_parts.contains(id.as_str())
                }) {
                    return None;
                }
                Some(
                    actual
                        .filter(|id| {
                            matrix.part_ids.contains(id) && live_parts.contains(id.as_str())
                        })
                        .into_iter()
                        .collect(),
                )
            } else {
                let id = part_id.as_ref()?;
                if !document.parts.iter().any(|part| part.id == *id) {
                    return None;
                }
                Some(
                    live_parts
                        .contains(id.as_str())
                        .then(|| id.clone())
                        .into_iter()
                        .collect(),
                )
            }
        }
        _ => None,
    }
}

pub(super) fn context_for_part(
    model: &boardstudio_application::ReadModel,
    part_id: &str,
) -> Option<TreeContext> {
    let snapshot = model.accepted.as_ref()?;
    let document = &snapshot.document;
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
            if primary == part_id && matrix.part_ids.iter().any(|id| id == part_id) {
                return Some(TreeContext::Key {
                    matrix_id: matrix.id.clone(),
                    row: cell.row,
                    column: cell.column,
                });
            }
            if matrix
                .cells
                .iter()
                .find(|item| item.row == cell.row && item.column == cell.column)
                .is_some_and(|config| {
                    config
                        .assemblies
                        .iter()
                        .any(|assembly| format!("{primary}/{}", assembly.id) == part_id)
                })
                && matrix.part_ids.iter().any(|id| id == part_id)
            {
                let assembly_id = part_id.strip_prefix(&format!("{primary}/"))?.to_owned();
                return Some(TreeContext::Component {
                    part_id: Some(part_id.to_owned()),
                    matrix_id: Some(matrix.id.clone()),
                    row: Some(cell.row),
                    column: Some(cell.column),
                    assembly_id: Some(assembly_id),
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

pub(super) fn context_for_cell(
    model: &boardstudio_application::ReadModel,
    matrix_id: &str,
    row: u32,
    column: u32,
) -> Option<TreeContext> {
    let snapshot = model.accepted.as_ref()?;
    let document = &snapshot.document;
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

pub(super) fn context_label(
    model: &boardstudio_application::ReadModel,
    context: &TreeContext,
) -> Option<String> {
    let snapshot = model.accepted.as_ref()?;
    let document = &snapshot.document;
    match context {
        TreeContext::Board { board_id } => document
            .boards
            .iter()
            .find(|board| board.id == *board_id)
            .map(|board| board.name.clone()),
        TreeContext::LayoutGroup { layout_ids, .. } => Some(if layout_ids.len() > 1 {
            "Linked halves".into()
        } else {
            "Layout".into()
        }),
        TreeContext::Matrix { matrix_id } => document
            .matrices
            .iter()
            .find(|matrix| matrix.id == *matrix_id)
            .map(|matrix| {
                matrix
                    .name
                    .as_deref()
                    .filter(|name| !name.trim().is_empty())
                    .map(str::to_owned)
                    .unwrap_or_else(|| {
                        format!(
                            "Matrix {}",
                            document
                                .matrices
                                .iter()
                                .position(|item| item.id == *matrix_id)
                                .unwrap_or(0)
                                + 1
                        )
                    })
            }),
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
            if let Some(part_id) = part_id {
                if let Some(part) = document.parts.iter().find(|part| part.id == *part_id) {
                    return Some(part.reference.clone());
                }
            }
            if let (Some(matrix_id), Some(row), Some(column), Some(assembly_id)) =
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
