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
    let document = boardstudio_application::resolved_document(&snapshot.document);
    let document = document.as_ref();
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
    let mut scene_order = HashMap::new();
    for (index, scene) in matrix_scenes.iter().enumerate() {
        scene_order
            .entry(scene.matrix_id.as_str())
            .or_insert((index, scene));
    }
    matrices.sort_by_key(|matrix| {
        scene_order
            .get(matrix.id.as_str())
            .map_or(usize::MAX, |&(index, _)| index)
    });
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
    let board_key = format!("board:{board_id}");
    let board_open = expanded.contains(&board_key);
    let live_board_count = board
        .part_ids
        .iter()
        .filter(|id| live_parts.contains_key(id.as_str()))
        .count();
    let mut tree = vec![TreeItem {
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
    }];
    if !board_open {
        return tree;
    }

    append_outline(
        &mut tree,
        document,
        board_id,
        expanded,
        outline_scenes,
        &matrices,
    );
    let (components, standalone) =
        layout_components(&document.parts, &board_parts, &matrices, &layouts);
    let layout_tree = LayoutTree {
        document,
        grouping,
        expanded,
        outline_scenes,
        board_id,
        matrices,
        scenes: scene_order,
        live_parts,
        definitions,
    };
    layout_tree.append_unowned(&mut tree, &layouts, &standalone);
    layout_tree.append_layouts(&mut tree, &layouts, &components);
    tree
}

fn append_outline(
    tree: &mut Vec<TreeItem>,
    document: &ProjectDoc,
    board_id: &str,
    expanded: &BTreeSet<String>,
    outline_scenes: &[BoardOutlineScene],
    matrices: &[&Matrix],
) {
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
    if !outline_open {
        return;
    }
    let versions = outline.into_iter().flat_map(|state| &state.versions);
    for version in std::iter::once(None).chain(versions.map(Some)) {
        let version_id = version.map(|version| version.id.as_str());
        let detail = match version_id {
            id if id == active_version => "Active",
            None => "Follows layout",
            Some(_) => "Fixed",
        };
        tree.push(TreeItem {
            id: match version_id {
                Some(id) => format!("outline:{board_id}:version:{id}"),
                None => format!("outline:{board_id}:generated"),
            },
            label: version.map_or_else(|| "Generated".into(), |version| version.name.clone()),
            detail: Some(detail.into()),
            level: 2,
            kind: TreeKind::OutlineVersion,
            context: Some(TreeContext::OutlineVersion {
                board_id: board_id.into(),
                version_id: version_id.map(str::to_owned),
            }),
            expanded: None,
            primary_id: None,
            expandable: false,
        });
    }
    if let Some(scene) = outline_scenes
        .iter()
        .find(|scene| scene.board_id == board_id)
    {
        append_outline_bridges(tree, board_id, scene, matrices);
    }
}

fn append_outline_bridges(
    tree: &mut Vec<TreeItem>,
    board_id: &str,
    scene: &BoardOutlineScene,
    matrices: &[&Matrix],
) {
    let parent = format!("outline:{board_id}");
    for (index, bridge) in scene.bridges.iter().enumerate() {
        let mut occurrences: Vec<_> = bridge
            .matrix_ids
            .iter()
            .filter_map(|id| matrices.iter().find(|matrix| matrix.id == *id))
            .map(|matrix| {
                matrix
                    .name
                    .as_deref()
                    .filter(|name| !name.trim().is_empty())
                    .unwrap_or(&matrix.id)
            })
            .collect();
        occurrences.sort_unstable();
        occurrences.dedup();
        let detail = if occurrences.is_empty() {
            "Bridge".to_owned()
        } else {
            occurrences.join(", ")
        };
        tree.push(bridge_item(
            board_id,
            &parent,
            &bridge.id,
            index + 1,
            2,
            format!("{} mm · {detail}", bridge.width),
        ));
    }
}

type LayoutComponents<'a> = HashMap<&'a str, Vec<&'a Part>>;

fn layout_components<'a>(
    parts: &'a [Part],
    board_parts: &HashSet<&str>,
    matrices: &[&Matrix],
    layouts: &[&'a Layout],
) -> (LayoutComponents<'a>, Vec<&'a Part>) {
    let matrix_part_ids: HashSet<&str> = matrices
        .iter()
        .flat_map(|matrix| matrix.part_ids.iter().map(String::as_str))
        .collect();
    let mut components_by_layout = HashMap::<_, Vec<_>>::new();
    let mut standalone = Vec::new();
    for part in parts {
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
    (components_by_layout, standalone)
}

fn layout_halves<'a>(
    document: &ProjectDoc,
    layouts: &[&'a Layout],
    matrices: &[&Matrix],
) -> Vec<(&'static str, Vec<&'a Layout>)> {
    let axis = layouts
        .iter()
        .find_map(|layout| same_board_pair_axis(document, layout))
        .or_else(|| {
            layouts
                .iter()
                .find_map(|layout| layout.mirror_link.as_ref().map(|link| link.axis_x))
        });
    let Some(axis) = axis else {
        return vec![("", layouts.to_vec())];
    };
    let mut left = Vec::new();
    let mut right = Vec::new();
    for layout in layouts.iter().copied() {
        let origin = layout
            .matrix_ids
            .iter()
            .map(|id| matrix_origin_x(matrices, id))
            .sum::<f64>()
            / layout.matrix_ids.len().max(1) as f64;
        if origin < axis {
            left.push(layout);
        } else if origin >= axis {
            right.push(layout);
        }
    }
    vec![("Left half", left), ("Right half", right)]
}

fn layout_is_linked(document: &ProjectDoc, layout: &Layout, layouts: &[&Layout]) -> bool {
    layout.mirror_link.is_some()
        || layouts.iter().any(|other| {
            other
                .mirror_link
                .as_ref()
                .is_some_and(|link| link.source_id == layout.id)
        })
        || same_board_pair_axis(document, layout).is_some()
}

fn same_board_pair_axis(document: &ProjectDoc, layout: &Layout) -> Option<f64> {
    use boardstudio_model::mirrored_half::MirrorDestination::SameBoardLayout;

    document.mirrored_pairs.iter().find_map(|pair| {
        [&pair.source, &pair.destination]
            .iter()
            .any(|end| matches!(end, SameBoardLayout(id) if id == &layout.id))
            .then_some(pair.mapping.at.x / 2.0)
    })
}

struct LayoutTree<'a> {
    document: &'a ProjectDoc,
    grouping: Grouping,
    expanded: &'a BTreeSet<String>,
    outline_scenes: &'a [BoardOutlineScene],
    board_id: &'a str,
    matrices: Vec<&'a Matrix>,
    scenes: HashMap<&'a str, (usize, &'a MatrixScene)>,
    live_parts: HashMap<&'a str, &'a Part>,
    definitions: HashMap<&'a str, &'a str>,
}

impl LayoutTree<'_> {
    fn append_unowned(&self, tree: &mut Vec<TreeItem>, layouts: &[&Layout], standalone: &[&Part]) {
        let owned: HashSet<_> = layouts
            .iter()
            .flat_map(|layout| &layout.matrix_ids)
            .collect();
        let unowned: Vec<_> = self
            .matrices
            .iter()
            .copied()
            .filter(|matrix| !owned.contains(&matrix.id))
            .collect();
        if unowned.is_empty() && standalone.is_empty() && !layouts.is_empty() {
            return;
        }
        let id = format!("layout:{}", self.board_id);
        let is_open = self.expanded.contains(&id);
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
                    self,
                    tree,
                    matrix,
                    2,
                    true,
                    &format!("matrix:{}", matrix.id),
                );
            }
            for part in standalone {
                append_component(tree, part, &self.definitions, 2);
            }
        }
    }

    fn append_layouts(
        &self,
        tree: &mut Vec<TreeItem>,
        layouts: &[&Layout],
        components: &LayoutComponents<'_>,
    ) {
        for (label, half) in layout_halves(self.document, layouts, &self.matrices) {
            let split = !label.is_empty();
            if split && (half.is_empty() || !self.append_half(tree, label, &half, components)) {
                continue;
            }
            for layout in half {
                self.append_layout(tree, layout, layouts, components, split);
            }
        }
    }

    fn append_half(
        &self,
        tree: &mut Vec<TreeItem>,
        label: &str,
        layouts: &[&Layout],
        components: &LayoutComponents<'_>,
    ) -> bool {
        let id = format!("half-group:{}:{label}", self.board_id);
        let open = !self.expanded.contains(&id);
        tree.push(TreeItem {
            id,
            label: label.into(),
            detail: None,
            level: 1,
            kind: TreeKind::HalfGroup,
            context: Some(TreeContext::LayoutGroup {
                board_id: self.board_id.to_owned(),
                layout_ids: layouts.iter().map(|layout| layout.id.clone()).collect(),
            }),
            expanded: Some(open),
            primary_id: None,
            expandable: true,
        });
        if open {
            let parts: Vec<_> = layouts
                .iter()
                .filter_map(|layout| components.get(layout.id.as_str()))
                .flatten()
                .copied()
                .collect();
            self.append_components_group(tree, &parts, label);
        }
        open
    }

    fn append_components_group(&self, tree: &mut Vec<TreeItem>, parts: &[&Part], half: &str) {
        if parts.is_empty() {
            return;
        }
        let id = format!("components:{}:{half}", self.board_id);
        let is_open = !self.expanded.contains(&id);
        tree.push(TreeItem {
            id,
            label: "Components".into(),
            detail: Some(format!("{} parts", parts.len())),
            level: 2,
            kind: TreeKind::Components,
            context: None,
            expanded: Some(is_open),
            primary_id: None,
            expandable: true,
        });
        if is_open {
            for part in parts {
                append_component(tree, part, &self.definitions, 3);
            }
        }
    }

    fn append_layout(
        &self,
        tree: &mut Vec<TreeItem>,
        layout: &Layout,
        layouts: &[&Layout],
        components: &LayoutComponents<'_>,
        split: bool,
    ) {
        let id = format!("half:{}", layout.id);
        let open = self.expanded.contains(&id);
        let level = 1 + usize::from(split);
        let linked = layout_is_linked(self.document, layout, layouts);
        tree.push(TreeItem {
            id: id.clone(),
            label: layout.name.clone(),
            detail: Some(if linked { "Linked" } else { "Independent" }.into()),
            level,
            kind: TreeKind::Layout,
            context: Some(if let [matrix_id] = layout.matrix_ids.as_slice() {
                TreeContext::Matrix {
                    matrix_id: matrix_id.clone(),
                }
            } else {
                TreeContext::LayoutGroup {
                    board_id: self.board_id.to_owned(),
                    layout_ids: vec![layout.id.clone()],
                }
            }),
            expanded: Some(open),
            primary_id: None,
            expandable: true,
        });
        if !open {
            return;
        }
        for matrix in self
            .matrices
            .iter()
            .copied()
            .filter(|matrix| layout.matrix_ids.contains(&matrix.id))
        {
            append_matrix(
                self,
                tree,
                matrix,
                level + 1,
                layout.matrix_ids.len() > 1,
                &id,
            );
        }
        if !split && let Some(parts) = components.get(layout.id.as_str()) {
            for part in parts {
                append_component(tree, part, &self.definitions, level + 1);
            }
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

fn append_matrix(
    layout: &LayoutTree<'_>,
    tree: &mut Vec<TreeItem>,
    matrix: &Matrix,
    level: usize,
    include_header: bool,
    occurrence_id: &str,
) {
    let projected = layout
        .scenes
        .get(matrix.id.as_str())
        .map(|(_, scene)| scene.cells.as_slice())
        .unwrap_or_default();
    let actual_member = |row: u32, column: u32| {
        projected
            .iter()
            .find(|cell| cell.row == row && cell.column == column && cell.enabled)
            .and_then(|cell| cell.member_id.as_deref())
            .filter(|id| layout.live_parts.contains_key(id))
    };
    let assembly_member = |row: u32, column: u32, assembly_id: &str| {
        let primary = actual_member(row, column)?;
        let candidate = format!("{primary}/{assembly_id}");
        (matrix.part_ids.contains(&candidate) && layout.live_parts.contains_key(candidate.as_str()))
            .then_some(candidate)
    };
    let index = layout
        .document
        .matrices
        .iter()
        .position(|candidate| candidate.id == matrix.id)
        .unwrap_or(0)
        + 1;
    let item_id = format!("matrix:{}", matrix.id);
    let is_open = !include_header || layout.expanded.contains(&item_id);
    if include_header {
        let primary_count = projected
            .iter()
            .filter(|cell| cell.enabled)
            .filter_map(|cell| cell.member_id.as_deref())
            .filter(|id| layout.live_parts.contains_key(id))
            .count();
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
                let target = layout
                    .document
                    .matrices
                    .iter()
                    .find(|matrix| matrix.id == anchor.matrix_id);
                format!(
                    "{} keys · Follows {} · row {}, column {}",
                    primary_count,
                    target
                        .and_then(|matrix| matrix.name.as_deref())
                        .unwrap_or(&anchor.matrix_id),
                    anchor.row + 1,
                    anchor.column + 1
                )
            } else {
                format!("{} keys", primary_count)
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
        layout.board_id,
        matrix,
        layout.outline_scenes,
        occurrence_id,
        child_level,
    );
    if matrix.rows == 0 || matrix.columns == 0 {
        return;
    }

    let (group_count, key_count) = match layout.grouping {
        Grouping::Column => (matrix.columns, matrix.rows),
        Grouping::Row => (matrix.rows, matrix.columns),
    };
    for group in 0..group_count {
        let (kind, label, context, prefix) = match layout.grouping {
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
        let coordinates = |index| match layout.grouping {
            Grouping::Column => (index, group),
            Grouping::Row => (group, index),
        };
        let is_open = layout.expanded.contains(&id);
        let key_count_live = (0..key_count)
            .filter(|&index| {
                let (row, column) = coordinates(index);
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
            let (row, column) = coordinates(index);
            let cell = matrix
                .cells
                .iter()
                .find(|cell| cell.row == row && cell.column == column);
            if cell.is_some_and(|cell| cell.deleted) {
                continue;
            }
            let enabled = cell.is_none_or(|cell| cell.enabled);
            let primary = enabled.then(|| actual_member(row, column)).flatten();
            let assemblies = cell.map_or(&[][..], |cell| cell.assemblies.as_slice());
            let key_id = format!("key:{}:{row}:{column}", matrix.id);
            let key_open = layout.expanded.contains(&key_id);
            tree.push(TreeItem {
                id: key_id,
                label: format!("Key {}.{}", column + 1, row + 1),
                detail: Some(
                    primary
                        .and_then(|id| layout.live_parts.get(id).map(|part| part.reference.clone()))
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
                for assembly in assemblies {
                    let actual = enabled
                        .then(|| assembly_member(row, column, &assembly.id))
                        .flatten();
                    let definition = actual
                        .as_deref()
                        .and_then(|id| layout.live_parts.get(id))
                        .and_then(|part| {
                            layout.definitions.get(part.definition_id.as_str()).copied()
                        })
                        .or_else(|| {
                            layout
                                .definitions
                                .get(assembly.definition_id.as_str())
                                .copied()
                        })
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
        tree.push(bridge_item(
            board_id,
            occurrence_id,
            &bridge.id,
            index + 1,
            level,
            format!("{} mm", bridge.width),
        ));
    }
}

fn bridge_item(
    board_id: &str,
    parent: &str,
    bridge_id: &str,
    number: usize,
    level: usize,
    detail: String,
) -> TreeItem {
    TreeItem {
        id: format!("{parent}:bridge:{bridge_id}"),
        label: format!("Bridge {number}"),
        detail: Some(detail),
        level,
        kind: TreeKind::Bridge,
        context: Some(TreeContext::Bridge {
            board_id: board_id.into(),
            bridge_id: bridge_id.into(),
        }),
        expanded: None,
        primary_id: None,
        expandable: false,
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
