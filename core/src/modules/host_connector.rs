//! Atomic creation of the source-backed physical host-side VIK receptacle.
use super::{ModuleDefinition, MountedModule};
use crate::model::{Part, PartDefinition, PartKind, Pose2, ProjectDoc, Side, Vec2};

const MODEL_ASSET: &str =
    "bundled-model:vik/sadekbaroudi-vik/kicad/3dmodels/vik-connector-horizontal.stp";

pub(super) fn add_for_mount(
    doc: &mut ProjectDoc,
    instance: &mut MountedModule,
    module_definition: &ModuleDefinition,
    connector_definition: &PartDefinition,
) -> Result<(), String> {
    let Some(connection) = instance.connection.as_ref() else {
        return Ok(());
    };
    let part_id = format!("{}/vik-host-connector", instance.id);
    if !connection.host_connector_part_id.is_empty() && connection.host_connector_part_id != part_id
    {
        if doc
            .parts
            .iter()
            .any(|part| part.id == connection.host_connector_part_id)
            && doc
                .boards
                .iter()
                .find(|board| board.id == instance.host_board_id)
                .is_some_and(|board| board.part_ids.contains(&connection.host_connector_part_id))
        {
            return Ok(());
        }
        return Err("The selected VIK host connector is missing from the host PCB".into());
    }
    validate_source_definition(connector_definition)?;
    if doc.parts.iter().any(|part| part.id == part_id) {
        return Err("The generated VIK host connector identity is already in use".into());
    }
    if let Some(existing) = doc
        .definitions
        .iter()
        .find(|definition| definition.id == connector_definition.id)
        && existing != connector_definition
    {
        return Err(
            "The VIK host connector definition identity conflicts with a project definition".into(),
        );
    }

    let pose = nearby_pose(doc, instance, module_definition, connector_definition)?;
    let reference = next_reference(doc, &instance.host_board_id);
    let part = Part {
        keycap: None,
        outline: None,
        id: part_id.clone(),
        definition_id: connector_definition.id.clone(),
        reference,
        pose,
        side: instance.host_face.clone(),
        locked: None,
        properties: None,
        generator_parameters: None,
    };
    if !doc
        .definitions
        .iter()
        .any(|definition| definition.id == connector_definition.id)
    {
        doc.definitions.push(connector_definition.clone());
    }
    let outline_ids = {
        let board = doc
            .boards
            .iter_mut()
            .find(|board| board.id == instance.host_board_id)
            .ok_or("The module host PCB is missing")?;
        board.part_ids.push(part_id.clone());
        board.outline_ids.clone()
    };
    for outline in &mut doc.outline {
        if outline_ids.iter().any(|id| id == outline.id())
            && let crate::model::OutlineFeature::PartEnvelope { part_ids, .. } = outline
            && !part_ids.contains(&part_id)
        {
            part_ids.push(part_id.clone());
        }
    }
    doc.parts.push(part);
    instance
        .connection
        .as_mut()
        .expect("connection was checked")
        .host_connector_part_id = part_id;
    Ok(())
}

fn validate_source_definition(definition: &PartDefinition) -> Result<(), String> {
    if !matches!(definition.kind, PartKind::Connector)
        || !definition
            .hardware_profile
            .as_ref()
            .is_some_and(|profile| profile.vik_role == Some(crate::hardware::VikRole::Host))
    {
        return Err(
            "Automatic VIK placement requires a source-backed host-role connector definition"
                .into(),
        );
    }
    let source = &definition
        .hardware_profile
        .as_ref()
        .expect("validated profile")
        .source;
    if source.repository != "https://github.com/sadekbaroudi/vik"
        || source.revision.len() != 40
        || source.sha256.as_deref().is_none_or(|hash| hash.len() != 64)
        || definition
            .kicad_source
            .as_ref()
            .is_none_or(|source| source.source.is_empty())
    {
        return Err("Automatic VIK placement requires pinned KiCad footprint provenance".into());
    }
    let contacts = (1..=12).all(|number| {
        definition
            .pads
            .iter()
            .any(|pad| pad.number == number.to_string())
    });
    if !contacts || definition.courtyard.len() < 3 {
        return Err("The VIK host connector must contain source-backed contacts 1–12 and a usable courtyard".into());
    }
    if !definition
        .models
        .as_ref()
        .is_some_and(|models| models.iter().any(|model| model.asset_id == MODEL_ASSET))
    {
        return Err(
            "The VIK host connector must reference the bundled horizontal connector model".into(),
        );
    }
    Ok(())
}

#[derive(Clone, Copy)]
struct Bounds {
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
}

impl Bounds {
    fn from_points(points: impl IntoIterator<Item = Vec2>) -> Option<Self> {
        let mut values = points.into_iter();
        let first = values.next()?;
        let mut result = Self {
            min_x: first.x,
            min_y: first.y,
            max_x: first.x,
            max_y: first.y,
        };
        for point in values {
            result.min_x = result.min_x.min(point.x);
            result.min_y = result.min_y.min(point.y);
            result.max_x = result.max_x.max(point.x);
            result.max_y = result.max_y.max(point.y);
        }
        Some(result)
    }
    fn overlaps(self, other: Self) -> bool {
        self.min_x < other.max_x
            && self.max_x > other.min_x
            && self.min_y < other.max_y
            && self.max_y > other.min_y
    }
}

fn transform(point: Vec2, pose: Pose2, flip_x: bool) -> Vec2 {
    let x = if flip_x { -point.x } else { point.x };
    let (sin, cos) = pose.rotation.to_radians().sin_cos();
    Vec2 {
        x: pose.at.x + x * cos - point.y * sin,
        y: pose.at.y + x * sin + point.y * cos,
    }
}

fn module_world_bounds(instance: &MountedModule, definition: &ModuleDefinition) -> Option<Bounds> {
    let flip = (instance.host_face == Side::Front) == (instance.facing_face == Side::Front);
    Bounds::from_points(
        definition
            .board
            .contours
            .iter()
            .flat_map(|contour| contour.points.iter().copied())
            .map(|point| {
                transform(
                    point,
                    Pose2 {
                        at: instance.at,
                        rotation: instance.rotation,
                    },
                    flip,
                )
            }),
    )
}

fn part_bounds(doc: &ProjectDoc, part: &Part) -> Option<Bounds> {
    let definition = doc
        .definitions
        .iter()
        .find(|definition| definition.id == part.definition_id)?;
    let flip = part.side == Side::Back;
    Bounds::from_points(
        definition
            .courtyard
            .iter()
            .copied()
            .map(|point| transform(point, part.pose, flip)),
    )
}

fn nearby_pose(
    doc: &ProjectDoc,
    instance: &MountedModule,
    module: &ModuleDefinition,
    connector: &PartDefinition,
) -> Result<Pose2, String> {
    let module_bounds = module_world_bounds(instance, module)
        .ok_or("Cannot place a VIK connector beside a module with no source board contour")?;
    let connector_bounds = Bounds::from_points(connector.courtyard.iter().copied())
        .ok_or("The VIK host connector has no source courtyard")?;
    let half_width = (connector_bounds.max_x - connector_bounds.min_x) / 2.0;
    let half_height = (connector_bounds.max_y - connector_bounds.min_y) / 2.0;
    let center_x = (connector_bounds.min_x + connector_bounds.max_x) / 2.0;
    let center_y = (connector_bounds.min_y + connector_bounds.max_y) / 2.0;
    let footprint = doc
        .boards
        .iter()
        .find(|board| board.id == instance.host_board_id)
        .ok_or("The module host PCB is missing")?;
    let mut blockers = Vec::new();
    for part in doc
        .parts
        .iter()
        .filter(|part| footprint.part_ids.contains(&part.id))
    {
        if let Some(bounds) = part_bounds(doc, part) {
            blockers.push(bounds);
        }
    }
    for other in doc
        .modules
        .iter()
        .filter(|other| other.host_board_id == instance.host_board_id && other.id != instance.id)
    {
        if let Some(definition) = doc
            .module_definitions
            .iter()
            .find(|definition| definition.id == other.definition_id)
            && let Some(bounds) = module_world_bounds(other, definition)
        {
            blockers.push(bounds);
        }
    }
    let gap = 1.5;
    let outline_features = board_outline_features(doc, &instance.host_board_id);
    for step in 0..16 {
        let extra = f64::from(step) * 2.0;
        let candidates = [
            Vec2 {
                x: module_bounds.max_x + gap + half_width + extra - center_x,
                y: (module_bounds.min_y + module_bounds.max_y) / 2.0 - center_y,
            },
            Vec2 {
                x: module_bounds.min_x - gap - half_width - extra - center_x,
                y: (module_bounds.min_y + module_bounds.max_y) / 2.0 - center_y,
            },
            Vec2 {
                x: (module_bounds.min_x + module_bounds.max_x) / 2.0 - center_x,
                y: module_bounds.min_y - gap - half_height - extra - center_y,
            },
            Vec2 {
                x: (module_bounds.min_x + module_bounds.max_x) / 2.0 - center_x,
                y: module_bounds.max_y + gap + half_height + extra - center_y,
            },
        ];
        for at in candidates {
            let pose = Pose2 { at, rotation: 0.0 };
            let bounds = Bounds::from_points(
                connector
                    .courtyard
                    .iter()
                    .copied()
                    .map(|point| transform(point, pose, instance.host_face == Side::Back)),
            )
            .expect("validated connector courtyard");
            let footprint_points: Vec<_> = connector
                .courtyard
                .iter()
                .copied()
                .map(|point| transform(point, pose, instance.host_face == Side::Back))
                .collect();
            if !bounds.overlaps(module_bounds)
                && blockers.iter().all(|blocker| !bounds.overlaps(*blocker))
                && fits_board_outline(&outline_features, &footprint_points)
            {
                return Ok(pose);
            }
        }
    }
    Err("No clear nearby host-PCB position is available for the VIK connector; move existing geometry and retry".into())
}

fn board_outline_features(doc: &ProjectDoc, board_id: &str) -> Vec<crate::model::OutlineFeature> {
    if let Some(state) = doc
        .board_outlines
        .iter()
        .find(|state| state.board_id == board_id)
        && let Some(version) = state
            .active_version_id
            .as_ref()
            .and_then(|id| state.versions.iter().find(|version| &version.id == id))
    {
        return version.geometry.features.clone();
    }
    let Some(board) = doc.boards.iter().find(|board| board.id == board_id) else {
        return Vec::new();
    };
    doc.outline
        .iter()
        .filter(|feature| board.outline_ids.iter().any(|id| id == feature.id()))
        .cloned()
        .collect()
}

fn fits_board_outline(features: &[crate::model::OutlineFeature], points: &[Vec2]) -> bool {
    use crate::model::{Operation, OutlineFeature};
    let additions: Vec<_> = features
        .iter()
        .filter(|feature| feature.operation() == Operation::Add)
        .filter(|feature| !matches!(feature, OutlineFeature::PartEnvelope { .. }))
        .collect();
    if additions.is_empty() {
        return true;
    }
    let subtracts: Vec<_> = features
        .iter()
        .filter(|feature| feature.operation() == Operation::Subtract)
        .filter(|feature| !matches!(feature, OutlineFeature::PartEnvelope { .. }))
        .collect();
    let samples = edge_samples(points);
    samples.iter().all(|point| {
        additions
            .iter()
            .any(|feature| inside_feature(feature, *point))
    }) && samples.iter().all(|point| {
        subtracts
            .iter()
            .all(|feature| !inside_feature(feature, *point))
    })
}

fn edge_samples(points: &[Vec2]) -> Vec<Vec2> {
    let mut result = Vec::new();
    for (index, start) in points.iter().enumerate() {
        let end = points[(index + 1) % points.len()];
        for sample in 0..=4 {
            let t = f64::from(sample) / 4.0;
            result.push(Vec2 {
                x: start.x + (end.x - start.x) * t,
                y: start.y + (end.y - start.y) * t,
            });
        }
    }
    result
}

fn inside_feature(feature: &crate::model::OutlineFeature, point: Vec2) -> bool {
    use crate::model::OutlineFeature;
    match feature {
        OutlineFeature::Polygon { points, .. } => point_in_polygon(points, point),
        OutlineFeature::Rect {
            rotation,
            center,
            size,
            ..
        } => {
            let angle = -rotation.unwrap_or(0.0).to_radians();
            let (sin, cos) = angle.sin_cos();
            let dx = point.x - center.x;
            let dy = point.y - center.y;
            let x = dx * cos - dy * sin;
            let y = dx * sin + dy * cos;
            x.abs() <= size.x / 2.0 && y.abs() <= size.y / 2.0
        }
        OutlineFeature::PartEnvelope { .. } => true,
    }
}

fn point_in_polygon(points: &[Vec2], point: Vec2) -> bool {
    if points.len() < 3 {
        return false;
    }
    let mut inside = false;
    let mut previous = points.len() - 1;
    for current in 0..points.len() {
        let first = points[current];
        let second = points[previous];
        if (first.y > point.y) != (second.y > point.y)
            && point.x < (second.x - first.x) * (point.y - first.y) / (second.y - first.y) + first.x
        {
            inside = !inside;
        }
        previous = current;
    }
    inside
}

fn next_reference(doc: &ProjectDoc, board_id: &str) -> String {
    let used: std::collections::BTreeSet<_> = doc
        .boards
        .iter()
        .find(|board| board.id == board_id)
        .into_iter()
        .flat_map(|board| board.part_ids.iter())
        .filter_map(|id| {
            doc.parts
                .iter()
                .find(|part| &part.id == id)
                .map(|part| part.reference.as_str())
        })
        .collect();
    (1..=999)
        .map(|number| format!("J_VIK{number}"))
        .find(|reference| !used.contains(reference.as_str()))
        .unwrap_or_else(|| "J_VIK_AUTO".into())
}
