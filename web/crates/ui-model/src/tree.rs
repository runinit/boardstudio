use boardstudio_model::model::{
    Board, BoardOutlineScene, Layout, Matrix, MatrixScene, Part, ProjectDoc, SceneDelta,
};
use std::collections::{BTreeSet, HashMap, HashSet};

#[cfg(test)]
thread_local! {
    static CONTEXT_LOOKUPS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static CELL_CONFIG_VISITS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TreeContext {
    Outline {
        board_id: String,
    },
    OutlineVersion {
        board_id: String,
        version_id: Option<String>,
    },
    Bridge {
        board_id: String,
        bridge_id: String,
    },
    Board {
        board_id: String,
    },
    /// A source module selected from its actual PCB overlay. It deliberately
    /// carries no host-part identity: daughterboard source parts stay module-owned.
    MountedModule {
        board_id: String,
        module_id: String,
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
pub enum TreeKind {
    Outline,
    OutlineVersion,
    Bridge,
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
pub struct TreeItem {
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
pub enum Grouping {
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

pub fn visible_matrices<'a>(
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

/// Host-compatible objects-tree visibility shared by every presentation crate.
/// The Layout objects tree owns the presentation call sites; this predicate is
/// the single production implementation used on both targets and in native tests.
pub fn matrix_visible_on_board(document: &ProjectDoc, board_id: &str, matrix_id: &str) -> bool {
    let board_parts = document
        .boards
        .iter()
        .find(|board| board.id == board_id)
        .map(|board| board.part_ids.iter().map(String::as_str).collect())
        .unwrap_or_default();
    visible_matrices(document, board_id, &board_parts)
        .iter()
        .any(|matrix| matrix.id == matrix_id)
}

pub fn default_disclosures(document: &ProjectDoc, board_id: &str) -> BTreeSet<String> {
    let Some(board) = document.boards.iter().find(|board| board.id == board_id) else {
        return BTreeSet::new();
    };
    let board_parts = board.part_ids.iter().map(String::as_str).collect();
    let mut defaults = BTreeSet::from([format!("board:{board_id}"), format!("layout:{board_id}")]);
    defaults.extend(
        visible_matrices(document, board_id, &board_parts)
            .iter()
            .map(|matrix| format!("matrix:{}", matrix.id)),
    );
    defaults
}

/// PCB inventory shows each actual board part as an explicit component, even when
/// that part is also a primary matrix key in the Layout workspace.
pub fn build_pcb_tree(
    model: &boardstudio_application::ReadModel,
    expanded: &BTreeSet<String>,
) -> Vec<TreeItem> {
    let Some(snapshot) = model.accepted.as_ref() else {
        return Vec::new();
    };
    let document = &snapshot.document;
    let Some(board) = document
        .boards
        .iter()
        .find(|board| board.id == model.active_board_id)
    else {
        return Vec::new();
    };
    let mut rows: Vec<_> = build_tree(
        document,
        &board.id,
        Grouping::Column,
        expanded,
        &snapshot.scene.matrix_scenes,
        &snapshot.scene.board_outline_scenes,
    )
    .into_iter()
    .filter(|row| {
        matches!(
            row.kind,
            TreeKind::Board | TreeKind::Outline | TreeKind::OutlineVersion | TreeKind::Bridge
        )
    })
    .collect();
    if !expanded.contains(&format!("board:{}", board.id)) {
        return rows;
    }
    for row in &mut rows {
        if let Some(TreeContext::Bridge { bridge_id, .. }) = row.context.as_ref()
            && let Some(bridge) = snapshot
                .scene
                .board_outline_scenes
                .iter()
                .find(|scene| scene.board_id == board.id)
                .and_then(|scene| scene.bridges.iter().find(|bridge| bridge.id == *bridge_id))
        {
            row.detail = Some(format!("{} mm", bridge.width));
        }
    }
    for part in document
        .parts
        .iter()
        .filter(|part| board.part_ids.contains(&part.id))
    {
        let name = document
            .definitions
            .iter()
            .find(|definition| definition.id == part.definition_id)
            .map_or("Component", |definition| definition.name.as_str());
        let side = match part.side {
            boardstudio_model::model::Side::Front => "front",
            boardstudio_model::model::Side::Back => "back",
        };
        rows.push(TreeItem {
            id: format!("pcb:{}", part.id),
            label: part.reference.clone(),
            detail: Some(format!("{name} · {side}")),
            level: 1,
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
    rows
}

pub fn build_tree(
    document: &ProjectDoc,
    board_id: &str,
    grouping: Grouping,
    expanded: &BTreeSet<String>,
    matrix_scenes: &[MatrixScene],
    outline_scenes: &[BoardOutlineScene],
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
    let mut matrices = visible_matrices(document, board_id, &board_parts);
    matrices.sort_by_key(|matrix| {
        matrix_scenes
            .iter()
            .position(|scene| scene.matrix_id == matrix.id)
            .unwrap_or(usize::MAX)
    });
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
                && matrices
                    .iter()
                    .any(|matrix| layout.matrix_ids.contains(&matrix.id))
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

    let outline = document
        .board_outlines
        .iter()
        .find(|state| state.board_id == board_id);
    let active_version = outline.and_then(|state| state.active_version_id.as_deref());
    let active_name = outline
        .and_then(|state| {
            state
                .versions
                .iter()
                .find(|version| Some(version.id.as_str()) == active_version)
        })
        .map_or("Generated", |version| version.name.as_str());
    let outline_key = format!("outline:{board_id}");
    let outline_open = expanded.contains(&outline_key);
    tree.push(TreeItem {
        id: outline_key,
        label: "Outline".into(),
        detail: Some(active_name.into()),
        level: 1,
        kind: TreeKind::Outline,
        context: Some(TreeContext::Outline {
            board_id: board_id.into(),
        }),
        expanded: Some(outline_open),
        primary_id: None,
        expandable: true,
    });
    if outline_open {
        tree.push(TreeItem {
            id: format!("outline:{board_id}:generated"),
            label: "Generated".into(),
            detail: Some(
                if active_version.is_none() {
                    "Active"
                } else {
                    "Follows layout"
                }
                .into(),
            ),
            level: 2,
            kind: TreeKind::OutlineVersion,
            context: Some(TreeContext::OutlineVersion {
                board_id: board_id.into(),
                version_id: None,
            }),
            expanded: None,
            primary_id: None,
            expandable: false,
        });
        if let Some(outline) = outline {
            for version in &outline.versions {
                tree.push(TreeItem {
                    id: format!("outline:{board_id}:version:{}", version.id),
                    label: version.name.clone(),
                    detail: Some(
                        if active_version == Some(version.id.as_str()) {
                            "Active"
                        } else {
                            "Fixed"
                        }
                        .into(),
                    ),
                    level: 2,
                    kind: TreeKind::OutlineVersion,
                    context: Some(TreeContext::OutlineVersion {
                        board_id: board_id.into(),
                        version_id: Some(version.id.clone()),
                    }),
                    expanded: None,
                    primary_id: None,
                    expandable: false,
                });
            }
        }
        if let Some(scene) = outline_scenes
            .iter()
            .find(|scene| scene.board_id == board_id)
        {
            for (index, bridge) in scene.bridges.iter().enumerate() {
                let mut occurrences = Vec::new();
                for matrix_id in &bridge.matrix_ids {
                    if let Some(matrix) = matrices.iter().find(|matrix| matrix.id == *matrix_id) {
                        occurrences.push(
                            matrix
                                .name
                                .as_deref()
                                .filter(|name| !name.trim().is_empty())
                                .unwrap_or(&matrix.id)
                                .to_owned(),
                        );
                    }
                }
                occurrences.sort();
                occurrences.dedup();
                let detail = if occurrences.is_empty() {
                    "Bridge".to_owned()
                } else {
                    occurrences.join(", ")
                };
                tree.push(TreeItem {
                    id: format!("outline:{board_id}:bridge:{}", bridge.id),
                    label: format!("Bridge {}", index + 1),
                    detail: Some(format!("{} mm · {detail}", bridge.width)),
                    level: 2,
                    kind: TreeKind::Bridge,
                    context: Some(TreeContext::Bridge {
                        board_id: board_id.into(),
                        bridge_id: bridge.id.clone(),
                    }),
                    expanded: None,
                    primary_id: None,
                    expandable: false,
                });
            }
        }
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
        .flat_map(|layout| {
            layout
                .matrix_ids
                .iter()
                .map(move |id| (id.as_str(), *layout))
        })
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
                    .filter(|layout| {
                        layout
                            .matrix_ids
                            .iter()
                            .map(|id| matrix_origin_x(&matrices, id))
                            .sum::<f64>()
                            / (layout.matrix_ids.len().max(1) as f64)
                            < axis
                    })
                    .collect(),
            ),
            (
                "Right half",
                layouts
                    .iter()
                    .copied()
                    .filter(|layout| {
                        layout
                            .matrix_ids
                            .iter()
                            .map(|id| matrix_origin_x(&matrices, id))
                            .sum::<f64>()
                            / (layout.matrix_ids.len().max(1) as f64)
                            >= axis
                    })
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
                    true,
                    board_id,
                    outline_scenes,
                    &format!("matrix:{}", matrix.id),
                );
            }
            for part in &standalone {
                append_component(&mut tree, part, &definitions, 2);
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
        if has_split {
            append_components_group(
                &mut tree,
                &components,
                &definitions,
                board_id,
                half_label,
                expanded,
                if half_level == 0 { 1 } else { 2 },
            );
        }
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
                context: Some(if layout.matrix_ids.len() == 1 {
                    TreeContext::Matrix {
                        matrix_id: layout.matrix_ids[0].clone(),
                    }
                } else {
                    TreeContext::LayoutGroup {
                        board_id: board_id.to_owned(),
                        layout_ids: vec![layout.id.clone()],
                    }
                }),
                expanded: Some(is_open),
                primary_id: None,
                expandable: true,
            });
            if is_open {
                let child_level = if half_level == 0 { 2 } else { 3 };
                for matrix in matrices
                    .iter()
                    .copied()
                    .filter(|matrix| layout.matrix_ids.contains(&matrix.id))
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
                        layout.matrix_ids.len() > 1,
                        board_id,
                        outline_scenes,
                        &layout_key,
                    );
                }
                if !has_split {
                    for part in components_by_layout
                        .get(layout.id.as_str())
                        .into_iter()
                        .flatten()
                    {
                        append_component(&mut tree, part, &definitions, child_level);
                    }
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
    include_header: bool,
    board_id: &str,
    outline_scenes: &[BoardOutlineScene],
    occurrence_id: &str,
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
    let is_open = !include_header || expanded.contains(&item_id);
    if include_header {
        tree.push(TreeItem {
            id: item_id,
            label: matrix
                .name
                .as_deref()
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .map(str::to_owned)
                .unwrap_or_else(|| format!("Matrix {index}")),
            detail: Some(if let Some(anchor) = &matrix.anchor {
                let target = document
                    .matrices
                    .iter()
                    .find(|matrix| matrix.id == anchor.matrix_id);
                format!(
                    "{} keys · Follows {} · row {}, column {}",
                    primary_ids.len(),
                    target
                        .and_then(|matrix| matrix.name.as_deref())
                        .unwrap_or(&anchor.matrix_id),
                    anchor.row + 1,
                    anchor.column + 1
                )
            } else {
                format!("{} keys", primary_ids.len())
            }),
            level,
            kind: TreeKind::Matrix,
            context: Some(TreeContext::Matrix {
                matrix_id: matrix.id.clone(),
            }),
            expanded: Some(is_open),
            primary_id: None,
            expandable: true,
        });
    }
    let child_level = level + usize::from(include_header);
    if !is_open {
        return;
    }
    append_matrix_bridge_occurrences(
        tree,
        board_id,
        matrix,
        outline_scenes,
        occurrence_id,
        child_level,
    );
    if matrix.rows == 0 || matrix.columns == 0 {
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
            level: child_level,
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
            if cell.is_some_and(|cell| cell.deleted) {
                continue;
            }
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
                level: child_level + 1,
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
                        level: child_level + 2,
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

fn append_matrix_bridge_occurrences(
    tree: &mut Vec<TreeItem>,
    board_id: &str,
    matrix: &Matrix,
    outline_scenes: &[BoardOutlineScene],
    occurrence_id: &str,
    level: usize,
) {
    let Some(scene) = outline_scenes
        .iter()
        .find(|scene| scene.board_id == board_id)
    else {
        return;
    };
    for (index, bridge) in scene.bridges.iter().enumerate() {
        if !bridge.matrix_ids.iter().any(|id| id == &matrix.id) {
            continue;
        }
        tree.push(TreeItem {
            id: format!("{occurrence_id}:bridge:{}", bridge.id),
            label: format!("Bridge {}", index + 1),
            detail: Some(format!("{} mm", bridge.width)),
            level,
            kind: TreeKind::Bridge,
            context: Some(TreeContext::Bridge {
                board_id: board_id.into(),
                bridge_id: bridge.id.clone(),
            }),
            expanded: None,
            primary_id: None,
            expandable: false,
        });
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
    if let Some(owner) = item.board_id.as_deref() {
        if owner != board.id {
            return None;
        }
    } else {
        let board_parts: HashSet<_> = board.part_ids.iter().map(String::as_str).collect();
        if !visible_matrices(document, &board.id, &board_parts)
            .iter()
            .any(|matrix| matrix.id == matrix_id)
        {
            return None;
        }
    }
    let projected = scene
        .matrix_scenes
        .iter()
        .find(|scene| scene.matrix_id == matrix_id)?;
    Some((item, projected))
}

mod selection;
#[cfg(test)]
use selection::component_context_for_explicit_part;
#[cfg(test)]
pub(super) use selection::context_is_resolvable;
pub(super) use selection::eligible_live_ids;
pub use selection::{
    active_board_scope_matches, component_context_for_finding_part, context_for_cell,
    context_for_part, context_label, resolve_selection, selection_context_is_current,
};
#[cfg(test)]
#[path = "tree_tests.rs"]
mod tests;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScopedTreeContext {
    pub scope: boardstudio_application::Scope,
    pub context: TreeContext,
}
