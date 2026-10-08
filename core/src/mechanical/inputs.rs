//! Core effective mechanical inputs.
//!
//! The single read path that resolves the configuration a board or physical half
//! actually uses: canonical settings, the legacy shared overlay with
//! first-instance fallback, instance settings, board thickness, wireless battery
//! defaults and flipped-instance handling. Apply, evaluation, findings and
//! presentation all reuse it. Filled in by the effective-inputs slice.
use crate::geometry::board_contours;
use crate::model::*;

use super::change::MechanicalTarget;

/// Fields a shared construction supplies over an instance's own configuration.
///
/// Authored openings, mounts, closure mounts, batteries, stabilizers, gasket
/// supports and board identity stay local to the instance.
const SHARED_OVERLAY_FIELDS: [&str; 20] = [
    "method",
    "mount",
    "integratedPlateFrame",
    "bottomStyle",
    "middleFrame",
    "plateThickness",
    "plateFoamThickness",
    "bottomFoamThickness",
    "bottomThickness",
    "plateToPcb",
    "wallThickness",
    "clearance",
    "gasket",
    "gasketTravel",
    "openingAllowance",
    "partProcesses",
    "internalGasket",
    "hardware",
    "criticalFits",
    "profiles",
];

/// The resolved mechanical configuration a target actually uses.
#[derive(Clone, Debug, PartialEq)]
pub struct MechanicalInputs {
    pub board_id: String,
    pub configuration: MechanicalConfiguration,
}

/// Resolves the effective mechanical configuration for a board or physical half.
pub fn effective_inputs(
    document: &ProjectDoc,
    target: &MechanicalTarget,
) -> Result<MechanicalInputs, String> {
    match target {
        MechanicalTarget::Board { board_id } => {
            let configuration = document
                .mechanical
                .as_ref()
                .filter(|configuration| configuration.board_id == *board_id)
                .cloned()
                .ok_or_else(|| {
                    format!("board {board_id:?} has no canonical mechanical configuration")
                })?;
            let configuration = apply_mechanical_defaults(document, configuration, board_id, false);
            Ok(MechanicalInputs {
                board_id: board_id.clone(),
                configuration,
            })
        }
        MechanicalTarget::Instance { instance_id } => {
            let hardware = document.hardware.as_ref().ok_or_else(|| {
                format!("physical instance {instance_id:?} is unavailable: no physical hardware")
            })?;
            let instance = hardware
                .instances
                .iter()
                .find(|instance| instance.id == *instance_id)
                .ok_or_else(|| format!("physical instance {instance_id:?} is unavailable"))?;
            let thickness = board_thickness(document, &instance.board_id)?;
            let configuration = resolve_instance_configuration(hardware, instance, thickness)?;
            let configuration = apply_mechanical_defaults(
                document,
                configuration,
                &instance.board_id,
                instance.flipped,
            );
            Ok(MechanicalInputs {
                board_id: instance.board_id.clone(),
                configuration,
            })
        }
    }
}

fn board_thickness(document: &ProjectDoc, board_id: &str) -> Result<f64, String> {
    document
        .boards
        .iter()
        .find(|board| board.id == board_id)
        .map(|board| board.thickness)
        .ok_or_else(|| format!("physical instance board {board_id:?} is not in the document"))
}

/// The shared construction source: the explicit shared construction, or the
/// first instance that carries a configuration.
fn shared_source(hardware: &HardwareConfiguration) -> Option<&MechanicalConfiguration> {
    hardware.shared_construction.as_ref().or_else(|| {
        hardware
            .instances
            .iter()
            .find_map(|entry| entry.mechanical.as_ref())
    })
}

fn resolve_instance_configuration(
    hardware: &HardwareConfiguration,
    instance: &PhysicalBoardInstance,
    board_thickness: f64,
) -> Result<MechanicalConfiguration, String> {
    match instance.construction_ownership {
        MechanicalConstructionOwnership::Legacy => {
            legacy_configuration(hardware, instance, board_thickness)
        }
        MechanicalConstructionOwnership::Local => local_configuration(instance, board_thickness),
        MechanicalConstructionOwnership::Pair => match owning_pair(hardware, &instance.id) {
            Some(pair) => pair_configuration(instance, pair, board_thickness),
            None => local_configuration(instance, board_thickness),
        },
    }
}

fn local_configuration(
    instance: &PhysicalBoardInstance,
    board_thickness: f64,
) -> Result<MechanicalConfiguration, String> {
    let mut configuration = instance.mechanical.clone().ok_or_else(|| {
        format!(
            "physical instance {:?} has no local mechanical configuration",
            instance.id
        )
    })?;
    configuration.board_id = instance.board_id.clone();
    configuration.pcb_thickness = board_thickness;
    Ok(configuration)
}

fn legacy_configuration(
    hardware: &HardwareConfiguration,
    instance: &PhysicalBoardInstance,
    board_thickness: f64,
) -> Result<MechanicalConfiguration, String> {
    let common = shared_source(hardware);
    let mut configuration = instance
        .mechanical
        .clone()
        .or_else(|| common.cloned())
        .ok_or_else(|| {
            format!(
                "physical instance {:?} has no mechanical configuration or shared construction",
                instance.id
            )
        })?;
    if instance.mechanical.is_none() {
        configuration.openings = None;
        configuration.mounts.clear();
        configuration.closure_mounts = None;
        configuration.battery = None;
        configuration.battery_height = 0.0;
    }
    if let Some(common) = common {
        configuration = overlay_shared(configuration, common, instance_supports(instance))?;
    }
    configuration.board_id = instance.board_id.clone();
    configuration.pcb_thickness = board_thickness;
    Ok(configuration)
}

fn pair_configuration(
    instance: &PhysicalBoardInstance,
    pair: &MechanicalMirrorPair,
    board_thickness: f64,
) -> Result<MechanicalConfiguration, String> {
    let configuration = instance.mechanical.clone().ok_or_else(|| {
        format!(
            "physical instance {:?} has no local mechanical configuration",
            instance.id
        )
    })?;
    let mut configuration = overlay_shared(
        configuration,
        &pair.construction,
        instance_supports(instance),
    )?;
    configuration.board_id = instance.board_id.clone();
    configuration.pcb_thickness = board_thickness;
    Ok(configuration)
}

fn owning_pair<'a>(
    hardware: &'a HardwareConfiguration,
    instance_id: &str,
) -> Option<&'a MechanicalMirrorPair> {
    hardware
        .pairs
        .iter()
        .find(|pair| pair.members.iter().any(|member| member == instance_id))
}

fn instance_supports(instance: &PhysicalBoardInstance) -> Vec<MechanicalGasketAnchor> {
    instance
        .mechanical
        .as_ref()
        .and_then(|configuration| configuration.gasket_layout.as_ref())
        .map(|layout| layout.supports.clone())
        .unwrap_or_default()
}

/// Overlays a shared construction's fields and its gasket layout (keeping the
/// instance's own supports) onto an instance configuration.
fn overlay_shared<S: serde::Serialize>(
    configuration: MechanicalConfiguration,
    shared: &S,
    supports: Vec<MechanicalGasketAnchor>,
) -> Result<MechanicalConfiguration, String> {
    let mut output = serde_json::to_value(configuration)
        .map_err(|error| error.to_string())?
        .as_object()
        .cloned()
        .ok_or_else(|| "mechanical configuration did not serialize to an object".to_string())?;
    let shared = serde_json::to_value(shared).map_err(|error| error.to_string())?;
    let shared = shared
        .as_object()
        .ok_or_else(|| "shared construction did not serialize to an object".to_string())?;
    for field in SHARED_OVERLAY_FIELDS {
        if let Some(value) = shared.get(field) {
            output.insert(field.into(), value.clone());
        }
    }
    if let Some(mut gasket_layout) = shared.get("gasketLayout").cloned() {
        if let Some(layout) = gasket_layout.as_object_mut() {
            layout.insert(
                "supports".into(),
                serde_json::to_value(supports).map_err(|error| error.to_string())?,
            );
        }
        output.insert("gasketLayout".into(), gasket_layout);
    }
    serde_json::from_value(serde_json::Value::Object(output)).map_err(|error| error.to_string())
}

/// Applies the defaults every effective configuration needs on top of its stored value.
///
/// Ports the adapter's remaining `mechanical_defaults` computation so `effective_inputs`
/// is the single source of the fully defaulted configuration:
///
/// - the wireless battery default sits at the board-local part center, mirrored across
///   X for a flipped instance, and drives `battery_height`;
/// - the stabilizer defaults propose a footprint for every wide MX switch keycap that
///   has no existing `STAB_MX_` footprint, rounding the key units and deriving the
///   rotation. Explicit stabilizer overrides keep every field except the recomputed
///   units and rotation.
///
/// The host `cad_jobs::mechanical_defaults` is now a thin delegate to this function.
pub fn apply_mechanical_defaults(
    document: &ProjectDoc,
    mut configuration: MechanicalConfiguration,
    board_id: &str,
    flipped: bool,
) -> MechanicalConfiguration {
    let board = document.boards.iter().find(|board| board.id == board_id);
    let parts = document
        .parts
        .iter()
        .filter(|part| board.is_some_and(|board| board.part_ids.contains(&part.id)))
        .collect::<Vec<_>>();
    fn center(parts: &[&Part], coordinate: impl Fn(&Part) -> f64) -> f64 {
        let values = parts
            .iter()
            .map(|part| coordinate(part))
            .collect::<Vec<_>>();
        values
            .iter()
            .copied()
            .reduce(f64::min)
            .zip(values.iter().copied().reduce(f64::max))
            .map_or(0.0, |(min, max)| (min + max) / 2.0)
    }
    let at = Vec2 {
        x: center(&parts, |part| part.pose.at.x) * if flipped { -1.0 } else { 1.0 },
        y: center(&parts, |part| part.pose.at.y),
    };
    if configuration.battery.is_none()
        && document
            .hardware
            .as_ref()
            .is_some_and(|hardware| hardware.transport == HardwareTransport::Wireless)
    {
        configuration.battery = Some(MechanicalBattery {
            cable_width: Some(2.0),
            size: Vec3 {
                x: 30.0,
                y: 20.0,
                z: 6.0,
            },
            at,
            cable_exit: Vec2 {
                x: at.x + 17.0,
                y: at.y,
            },
        });
    }
    if let Some(battery) = &configuration.battery {
        configuration.battery_height = battery.size.z;
    }

    let explicit = configuration.stabilizers.clone().unwrap_or_default();
    let mut stabilizers = Vec::new();
    for part in &parts {
        let Some(definition) = document
            .definitions
            .iter()
            .find(|definition| definition.id == part.definition_id)
        else {
            continue;
        };
        let size = part.keycap.as_ref().or(definition.keycap.as_ref());
        let Some(size) = size.filter(|size| size.x.max(size.y) >= 37.0) else {
            continue;
        };
        if definition
            .generator
            .as_ref()
            .is_none_or(|generator| generator.source.to_lowercase() != "ceoloide/switch_mx")
        {
            continue;
        }
        let has_existing = document.parts.iter().any(|candidate| {
            let is_stabilizer = document
                .definitions
                .iter()
                .find(|entry| entry.id == candidate.definition_id)
                .and_then(|entry| entry.kicad_source.as_ref())
                .is_some_and(|source| source.source.contains("(footprint \"STAB_MX_"));
            is_stabilizer
                && (candidate.pose.at.x - part.pose.at.x)
                    .hypot(candidate.pose.at.y - part.pose.at.y)
                    < 0.01
        });
        if has_existing {
            continue;
        }
        let key_units = (((size.x.max(size.y) + 1.0) / 19.05) * 4.0).round() / 4.0;
        let units = if key_units <= 2.75 { 2.0 } else { key_units };
        let rotation = if size.y > size.x { 90.0 } else { 0.0 };
        let mut stabilizer = explicit
            .iter()
            .find(|entry| entry.part_id == part.id)
            .cloned()
            .unwrap_or(MechanicalStabilizerOverride {
                profile: None,
                part_id: part.id.clone(),
                kind: MechanicalStabilizerKind::PcbMount,
                units,
                rotation: Some(rotation),
            });
        stabilizer.units = units;
        stabilizer.rotation = Some(rotation);
        stabilizers.push(stabilizer);
    }
    configuration.stabilizers = Some(stabilizers);
    configuration
}

/// Projects the effective document a target's mechanical resolution sees.
///
/// Installs the resolved effective configuration as `document.mechanical` and, for a
/// physical instance, records `document.physical_instance_id`. The remaining
/// defaulting (battery placement and stabilizer proposals) lives in
/// [`apply_mechanical_defaults`], which [`effective_inputs`] already applies, so the
/// projected document carries the fully defaulted configuration.
///
/// A flipped instance is mirrored exactly as the host `effective_case_inputs` does:
/// the target board's parts get `pose.at.x = -x`, `pose.rotation = -rotation`, a
/// swapped `side` and a rewritten `side` generator parameter; that board's
/// `board_references` are disabled; and the board's contours are mirrored by negating
/// each point's X and reversing the point order. The mirrored contours are returned
/// so callers can resolve the assembly and replace the scene's target-board contours
/// without recomputing them.
///
/// A missing canonical or instance configuration is not an error here: the structural
/// projection (instance identity, part mirroring, contours) is still meaningful, and
/// callers that require a configuration use [`effective_inputs`] directly. This matches
/// the host projection's previous leniency.
pub fn effective_document(
    document: &ProjectDoc,
    target: &MechanicalTarget,
) -> Result<(ProjectDoc, Vec<Contour>), String> {
    let mut effective = document.clone();
    let (board_id, flipped) = match target {
        MechanicalTarget::Board { board_id } => {
            // A canonical configuration that belongs to another board is still
            // projected with this board's defaults, matching the host's previous
            // unconditional `document.mechanical.clone()` behaviour.
            effective.mechanical = document.mechanical.clone().map(|configuration| {
                effective_inputs(document, target)
                    .map(|inputs| inputs.configuration)
                    .unwrap_or_else(|_| {
                        apply_mechanical_defaults(document, configuration, board_id, false)
                    })
            });
            (board_id.clone(), false)
        }
        MechanicalTarget::Instance { instance_id } => {
            let instance = document
                .hardware
                .as_ref()
                .and_then(|hardware| {
                    hardware
                        .instances
                        .iter()
                        .find(|instance| instance.id == *instance_id)
                })
                .ok_or("The selected physical instance is unavailable")?;
            effective.physical_instance_id = Some(instance_id.clone());
            effective.mechanical = effective_inputs(document, target)
                .ok()
                .map(|inputs| inputs.configuration);
            (instance.board_id.clone(), instance.flipped)
        }
    };

    if flipped {
        let selected_parts = document
            .boards
            .iter()
            .find(|board| board.id == board_id)
            .map(|board| {
                board
                    .part_ids
                    .iter()
                    .collect::<std::collections::HashSet<_>>()
            })
            .unwrap_or_default();
        for part in &mut effective.parts {
            if !selected_parts.contains(&part.id) {
                continue;
            }
            part.pose.at.x = -part.pose.at.x;
            part.pose.rotation = -part.pose.rotation;
            part.side = match part.side {
                Side::Front => Side::Back,
                Side::Back => Side::Front,
            };
            let parameters = part
                .generator_parameters
                .get_or_insert_with(Default::default);
            parameters.insert(
                "side".into(),
                serde_json::Value::String(if part.side == Side::Front { "F" } else { "B" }.into()),
            );
        }
        for reference in &mut effective.board_references {
            if reference.board_id == board_id {
                reference.enabled = false;
            }
        }
    }

    // `geometry::board_contours` reads the outline path cache, so build it from the
    // source document first; a default (empty) cache would resolve no contours. The
    // contours are taken before mirroring so a flipped instance reflects the same
    // accepted geometry the host scene carried.
    let (cache, _, _) = crate::geometry::outlines(document, None, &[]);
    let (per_board, _, _) = board_contours(document, &cache);
    let mut contours = per_board
        .into_iter()
        .find(|entry| entry.board_id == board_id)
        .map(|entry| entry.contours)
        .unwrap_or_default();
    if flipped {
        for contour in &mut contours {
            for point in &mut contour.points {
                point.x = -point.x;
            }
            contour.points.reverse();
        }
    }
    Ok((effective, contours))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        Board, HardwareConfiguration, HardwareTopology, HardwareTransport,
        MechanicalConstructionOwnership, MechanicalGasketAnchor, MechanicalGasketLayout,
        MechanicalMirrorPair, Part, PhysicalBoardInstance, Pose2, Side, Vec2,
    };

    fn board(id: &str, thickness: f64) -> Board {
        Board {
            id: id.into(),
            name: id.into(),
            outline_ids: vec![],
            part_ids: vec![],
            net_ids: vec![],
            thickness,
            traces: vec![],
            vias: vec![],
        }
    }

    fn configuration(board_id: &str) -> MechanicalConfiguration {
        serde_json::from_value(serde_json::json!({
            "boardId": board_id,
            "method": "printed",
            "mount": "tray",
            "plateThickness": 1.5,
            "plateFoamThickness": 1.0,
            "pcbThickness": 1.6,
            "bottomFoamThickness": 0.5,
            "batteryHeight": 0.0,
            "bottomThickness": 2.0,
            "plateToPcb": 3.5,
            "wallThickness": 2.0,
            "clearance": 0.2,
            "profiles": []
        }))
        .unwrap()
    }

    fn part(id: &str, x: f64) -> Part {
        Part {
            id: id.into(),
            definition_id: "switch".into(),
            reference: id.into(),
            pose: Pose2 {
                at: Vec2 { x, y: 2.0 },
                rotation: 0.0,
            },
            side: Side::Front,
            locked: None,
            keycap: None,
            outline: None,
            properties: None,
            generator_parameters: None,
        }
    }

    fn instance(
        id: &str,
        board_id: &str,
        ownership: MechanicalConstructionOwnership,
        mechanical: Option<MechanicalConfiguration>,
    ) -> PhysicalBoardInstance {
        PhysicalBoardInstance {
            id: id.into(),
            name: id.into(),
            board_id: board_id.into(),
            half: "left".into(),
            role: "controller".into(),
            flipped: false,
            controller_part_id: None,
            mechanical,
            construction_ownership: ownership,
        }
    }

    fn hardware(
        instances: Vec<PhysicalBoardInstance>,
        shared: Option<MechanicalConfiguration>,
        transport: HardwareTransport,
    ) -> HardwareConfiguration {
        HardwareConfiguration {
            topology: HardwareTopology::Unibody,
            transport,
            instances,
            boards: vec![],
            shared_construction: shared,
            pairs: vec![],
        }
    }

    fn document_with(board: Board, hardware: HardwareConfiguration) -> ProjectDoc {
        let mut document = ProjectDoc::empty("doc", "Fixture");
        document.boards.push(board);
        document.hardware = Some(hardware);
        document
    }

    fn mount(id: &str) -> Mount {
        serde_json::from_value(serde_json::json!({
            "id": id, "at": { "x": 1.0, "y": 2.0 }, "kind": "hole", "holeDiameter": 2.2
        }))
        .unwrap()
    }

    fn opening(id: &str) -> MechanicalOpening {
        serde_json::from_value(serde_json::json!({
            "id": id,
            "vertices": [{ "id": "vertex", "point": { "x": 0.0, "y": 0.0 } }],
            "z": 0.0,
            "height": 1.0
        }))
        .unwrap()
    }

    fn battery() -> MechanicalBattery {
        serde_json::from_value(serde_json::json!({
            "size": { "x": 30.0, "y": 20.0, "z": 6.0 },
            "at": { "x": 1.0, "y": 2.0 },
            "cableExit": { "x": 3.0, "y": 2.0 }
        }))
        .unwrap()
    }

    fn stabilizer() -> MechanicalStabilizerOverride {
        serde_json::from_value(serde_json::json!({
            "partId": "part", "kind": "pcb-mount", "units": 2.0
        }))
        .unwrap()
    }

    fn profile(id: &str) -> MechanicalPartProfile {
        serde_json::from_value(serde_json::json!({
            "definitionId": id, "source": "test", "cutouts": [], "plateToPcb": 3.5
        }))
        .unwrap()
    }

    fn hardware_specification(id: &str) -> MechanicalHardwareSpecification {
        serde_json::from_value(serde_json::json!({
            "id": id, "partId": "part", "featureId": "feature", "designation": "M2",
            "thread": "M2", "length": 8.0, "quantity": 1
        }))
        .unwrap()
    }

    fn critical_fit(id: &str) -> MechanicalCriticalFit {
        serde_json::from_value(serde_json::json!({
            "id": id, "partId": "part", "label": "fit",
            "from": { "x": 0.0, "y": 0.0 }, "to": { "x": 1.0, "y": 1.0 },
            "tolerance": "0.1"
        }))
        .unwrap()
    }

    fn anchor(id: &str) -> MechanicalGasketAnchor {
        serde_json::from_value(serde_json::json!({
            "id": id, "regionId": "region", "outlineKey": "outline", "anchor": 0.0
        }))
        .unwrap()
    }

    fn mirror_pair(
        id: &str,
        members: [&str; 2],
        construction: serde_json::Value,
    ) -> MechanicalMirrorPair {
        serde_json::from_value(serde_json::json!({
            "id": id,
            "members": members,
            "mapping": { "at": { "x": 0.0, "y": 0.0 }, "rotation": 0.0 },
            "construction": construction
        }))
        .unwrap()
    }

    fn gasket_layout(id: &str, supports: Vec<MechanicalGasketAnchor>) -> MechanicalGasketLayout {
        let _ = id;
        serde_json::from_value(serde_json::json!({
            "length": 99.0,
            "width": 3.0,
            "thickness": 2.0,
            "compression": 0.15,
            "supports": supports
        }))
        .unwrap()
    }

    #[test]
    fn canonical_configuration_resolves_to_itself() {
        let mut document = ProjectDoc::empty("doc", "Fixture");
        document.boards.push(board("board", 1.6));
        let mut canonical = configuration("board");
        canonical.clearance = 0.4;
        // The defaulting pass always materializes the stabilizer proposal list.
        canonical.stabilizers = Some(vec![]);
        document.mechanical = Some(canonical.clone());

        let inputs = effective_inputs(
            &document,
            &MechanicalTarget::Board {
                board_id: "board".into(),
            },
        )
        .unwrap();

        assert_eq!(inputs.board_id, "board");
        assert_eq!(inputs.configuration, canonical);
    }

    #[test]
    fn legacy_instance_overlays_the_shared_construction() {
        let mut common = configuration("shared");
        common.method = PlateMethod::Cnc;
        common.mount = MechanicalMount::Rigid;
        common.plate_thickness = 4.0;
        common.profiles = vec![profile("shared")];
        common.hardware = Some(vec![hardware_specification("shared")]);
        common.critical_fits = Some(vec![critical_fit("shared")]);
        common.gasket_layout = Some(gasket_layout("shared", vec![anchor("shared-support")]));

        let mut local = configuration("board");
        local.method = PlateMethod::Printed;
        local.plate_thickness = 1.0;
        local.openings = Some(vec![opening("local-opening")]);
        local.mounts = vec![mount("local-mount")];
        local.closure_mounts = Some(vec![mount("local-closure")]);
        local.battery = Some(battery());
        local.stabilizers = Some(vec![stabilizer()]);
        local.gasket_layout = Some(gasket_layout("local", vec![anchor("local-support")]));

        let document = document_with(
            board("board", 1.6),
            hardware(
                vec![instance(
                    "left",
                    "board",
                    MechanicalConstructionOwnership::Legacy,
                    Some(local),
                )],
                Some(common.clone()),
                HardwareTransport::None,
            ),
        );

        let inputs = effective_inputs(
            &document,
            &MechanicalTarget::Instance {
                instance_id: "left".into(),
            },
        )
        .unwrap();
        let configuration = inputs.configuration;

        // Overlaid shared fields.
        assert_eq!(configuration.method, PlateMethod::Cnc);
        assert_eq!(configuration.mount, MechanicalMount::Rigid);
        assert_eq!(configuration.plate_thickness, 4.0);
        assert_eq!(configuration.profiles, common.profiles);
        assert_eq!(configuration.hardware, common.hardware);
        assert_eq!(configuration.critical_fits, common.critical_fits);
        // The shared layout keeps the instance's own supports.
        let layout = configuration.gasket_layout.clone().unwrap();
        assert_eq!(layout.length, 99.0);
        assert_eq!(layout.supports, vec![anchor("local-support")]);
        // Local-only fields survive the overlay.
        assert_eq!(configuration.openings, Some(vec![opening("local-opening")]));
        assert_eq!(configuration.mounts, vec![mount("local-mount")]);
        assert_eq!(
            configuration.closure_mounts,
            Some(vec![mount("local-closure")])
        );
        assert_eq!(configuration.battery, Some(battery()));
        // The defaulting pass recomputes the stabilizer proposal list; this board
        // has no wide MX keycap part, so the list is empty.
        assert_eq!(configuration.stabilizers, Some(vec![]));
        assert_eq!(configuration.board_id, "board");
        assert_eq!(configuration.pcb_thickness, 1.6);
    }

    #[test]
    fn legacy_instance_without_a_shared_source_falls_back_to_the_first_instance() {
        let mut primary = configuration("board");
        primary.method = PlateMethod::Cnc;
        primary.plate_thickness = 3.0;
        primary.openings = Some(vec![opening("primary-opening")]);
        primary.mounts = vec![mount("primary-mount")];

        let document = document_with(
            board("board", 1.6),
            hardware(
                vec![
                    instance(
                        "primary",
                        "board",
                        MechanicalConstructionOwnership::Legacy,
                        Some(primary),
                    ),
                    instance(
                        "secondary",
                        "board",
                        MechanicalConstructionOwnership::Legacy,
                        None,
                    ),
                ],
                None,
                HardwareTransport::None,
            ),
        );

        let inputs = effective_inputs(
            &document,
            &MechanicalTarget::Instance {
                instance_id: "secondary".into(),
            },
        )
        .unwrap();
        let configuration = inputs.configuration;

        assert_eq!(configuration.method, PlateMethod::Cnc);
        assert_eq!(configuration.plate_thickness, 3.0);
        // The target had no local configuration, so its board-local features clear.
        assert!(configuration.openings.is_none());
        assert!(configuration.mounts.is_empty());
        assert!(configuration.closure_mounts.is_none());
        assert!(configuration.battery.is_none());
        assert_eq!(configuration.board_id, "board");
        assert_eq!(configuration.pcb_thickness, 1.6);
    }

    #[test]
    fn local_instance_ignores_the_shared_overlay() {
        let mut common = configuration("shared");
        common.method = PlateMethod::Cnc;
        common.plate_thickness = 9.0;

        let mut local = configuration("board");
        local.method = PlateMethod::Printed;
        local.plate_thickness = 1.0;

        let document = document_with(
            board("board", 1.6),
            hardware(
                vec![instance(
                    "left",
                    "board",
                    MechanicalConstructionOwnership::Local,
                    Some(local),
                )],
                Some(common),
                HardwareTransport::None,
            ),
        );

        let inputs = effective_inputs(
            &document,
            &MechanicalTarget::Instance {
                instance_id: "left".into(),
            },
        )
        .unwrap();

        assert_eq!(inputs.configuration.method, PlateMethod::Printed);
        assert_eq!(inputs.configuration.plate_thickness, 1.0);
    }

    #[test]
    fn pair_instance_uses_the_pairs_shared_construction() {
        let mut local = configuration("board");
        local.method = PlateMethod::Printed;
        local.plate_thickness = 1.0;
        local.openings = Some(vec![opening("local-opening")]);

        let mut hardware_with_pair = hardware(
            vec![instance(
                "left",
                "board",
                MechanicalConstructionOwnership::Pair,
                Some(local),
            )],
            None,
            HardwareTransport::None,
        );
        hardware_with_pair.pairs.push(mirror_pair(
            "pair-left-right",
            ["left", "right"],
            serde_json::json!({
                "method": "cnc", "mount": "rigid", "integratedPlateFrame": true,
                "plateThickness": 7.0, "plateFoamThickness": 0.5,
                "bottomFoamThickness": 0.25, "bottomThickness": 3.0,
                "plateToPcb": 4.0, "wallThickness": 2.5, "clearance": 0.3
            }),
        ));
        let document = document_with(board("board", 1.6), hardware_with_pair);

        let inputs = effective_inputs(
            &document,
            &MechanicalTarget::Instance {
                instance_id: "left".into(),
            },
        )
        .unwrap();

        assert_eq!(inputs.configuration.method, PlateMethod::Cnc);
        assert_eq!(inputs.configuration.plate_thickness, 7.0);
        assert_eq!(
            inputs.configuration.openings,
            Some(vec![opening("local-opening")])
        );
        assert_eq!(inputs.configuration.board_id, "board");
        assert_eq!(inputs.configuration.pcb_thickness, 1.6);

        // A `Pair` instance with no owning pair behaves local.
        let mut hardware_with_pair = hardware(
            vec![instance(
                "solo",
                "board",
                MechanicalConstructionOwnership::Pair,
                Some(configuration("board")),
            )],
            None,
            HardwareTransport::None,
        );
        hardware_with_pair.pairs.push(mirror_pair(
            "pair-a-b",
            ["a", "b"],
            serde_json::json!({
                "method": "cnc", "mount": "rigid", "integratedPlateFrame": true,
                "plateThickness": 7.0, "plateFoamThickness": 0.5,
                "bottomFoamThickness": 0.25, "bottomThickness": 3.0,
                "plateToPcb": 4.0, "wallThickness": 2.5, "clearance": 0.3
            }),
        ));
        let document = document_with(board("board", 1.6), hardware_with_pair);
        let inputs = effective_inputs(
            &document,
            &MechanicalTarget::Instance {
                instance_id: "solo".into(),
            },
        )
        .unwrap();
        assert_eq!(inputs.configuration.method, PlateMethod::Printed);
    }

    #[test]
    fn flipped_instance_keeps_its_board_local_reflection() {
        let mut document = ProjectDoc::empty("doc", "Fixture");
        let mut board = board("board", 1.6);
        board.part_ids = vec!["part-a".into(), "part-b".into()];
        document.boards.push(board);
        document.parts = vec![part("part-a", 3.0), part("part-b", 8.0)];
        let mut flipped = instance(
            "left",
            "board",
            MechanicalConstructionOwnership::Legacy,
            Some(configuration("board")),
        );
        flipped.flipped = true;
        document.hardware = Some(hardware(vec![flipped], None, HardwareTransport::Wireless));

        let inputs = effective_inputs(
            &document,
            &MechanicalTarget::Instance {
                instance_id: "left".into(),
            },
        )
        .unwrap();
        let battery = inputs.configuration.battery.expect("wireless default");

        // Board-local part center x is (3 + 8) / 2 = 5.5; the flip reflects it.
        assert!((battery.at.x + 5.5).abs() < 1e-9, "{:?}", battery.at);
        assert!((battery.at.y - 2.0).abs() < 1e-9, "{:?}", battery.at);
        assert!((battery.cable_exit.x - (-5.5 + 17.0)).abs() < 1e-9);
        assert_eq!(inputs.configuration.battery_height, 6.0);
    }

    #[test]
    fn wireless_transport_supplies_the_battery_default_when_none_is_stored() {
        let mut document = ProjectDoc::empty("doc", "Fixture");
        let mut board = board("board", 1.6);
        board.part_ids = vec!["part-a".into(), "part-b".into()];
        document.boards.push(board);
        document.parts = vec![part("part-a", 3.0), part("part-b", 8.0)];
        document.mechanical = Some(configuration("board"));
        document.hardware = Some(hardware(vec![], None, HardwareTransport::Wireless));

        let inputs = effective_inputs(
            &document,
            &MechanicalTarget::Board {
                board_id: "board".into(),
            },
        )
        .unwrap();
        let default_battery = inputs.configuration.battery.expect("wireless default");

        assert!((default_battery.at.x - 5.5).abs() < 1e-9);
        assert_eq!(default_battery.size.z, 6.0);
        assert_eq!(inputs.configuration.battery_height, 6.0);

        // A stored battery is never replaced by the default.
        document.mechanical.as_mut().unwrap().battery = Some(battery());
        let inputs = effective_inputs(
            &document,
            &MechanicalTarget::Board {
                board_id: "board".into(),
            },
        )
        .unwrap();
        assert_eq!(inputs.configuration.battery, Some(battery()));
    }

    #[test]
    fn ineligible_targets_return_an_error() {
        // No canonical configuration at all.
        let empty = ProjectDoc::empty("doc", "Fixture");
        assert!(
            effective_inputs(
                &empty,
                &MechanicalTarget::Board {
                    board_id: "board".into(),
                },
            )
            .is_err()
        );

        // Canonical configuration belongs to another board.
        let mut document = ProjectDoc::empty("doc", "Fixture");
        document.boards.push(board("board", 1.6));
        document.mechanical = Some(configuration("other"));
        assert!(
            effective_inputs(
                &document,
                &MechanicalTarget::Board {
                    board_id: "board".into(),
                },
            )
            .is_err()
        );

        // No hardware configuration at all.
        assert!(
            effective_inputs(
                &document,
                &MechanicalTarget::Instance {
                    instance_id: "left".into(),
                },
            )
            .is_err()
        );

        // Hardware without the requested instance.
        document.hardware = Some(hardware(vec![], None, HardwareTransport::None));
        assert!(
            effective_inputs(
                &document,
                &MechanicalTarget::Instance {
                    instance_id: "left".into(),
                },
            )
            .is_err()
        );

        // Instance whose board is not in the document.
        document.hardware = Some(hardware(
            vec![instance(
                "left",
                "ghost",
                MechanicalConstructionOwnership::Legacy,
                Some(configuration("ghost")),
            )],
            None,
            HardwareTransport::None,
        ));
        assert!(
            effective_inputs(
                &document,
                &MechanicalTarget::Instance {
                    instance_id: "left".into(),
                },
            )
            .is_err()
        );
    }

    #[test]
    fn a_flipped_instance_mirrors_parts_references_and_contours() {
        let mut document = ProjectDoc::empty("doc", "Fixture");
        document.outline.push(OutlineFeature::Rect {
            anchor_part_id: None,
            id: "outline".into(),
            center: Vec2 { x: 0.0, y: 0.0 },
            size: Vec2 { x: 80.0, y: 60.0 },
            radius: 0.0,
            rotation: None,
            operation: Operation::Add,
        });
        let mut board = board("board", 1.6);
        board.outline_ids = vec!["outline".into()];
        board.part_ids = vec!["left-switch".into(), "right-switch".into()];
        document.boards.push(board);
        document.parts.push(part("left-switch", 3.0));
        document.parts.push(part("right-switch", 40.0));
        document.board_references.push(
            serde_json::from_value(serde_json::json!({
                "id": "reference", "boardId": "board", "assetId": "asset",
                "enabled": true,
                "pose": { "at": { "x": 0.0, "y": 0.0 }, "rotation": 0.0 },
                "elevation": 0.0, "modelAssets": {}
            }))
            .unwrap(),
        );
        let mut flipped = instance(
            "left",
            "board",
            MechanicalConstructionOwnership::Local,
            Some(configuration("board")),
        );
        flipped.flipped = true;
        document.hardware = Some(hardware(vec![flipped], None, HardwareTransport::None));

        let (effective, contours) = effective_document(
            &document,
            &MechanicalTarget::Instance {
                instance_id: "left".into(),
            },
        )
        .unwrap();

        assert_eq!(effective.physical_instance_id.as_deref(), Some("left"));
        let mirrored = effective
            .parts
            .iter()
            .find(|part| part.id == "left-switch")
            .unwrap();
        assert_eq!(mirrored.pose.at.x, -3.0);
        assert_eq!(mirrored.side, Side::Back);
        assert_eq!(
            mirrored.generator_parameters.as_ref().unwrap()["side"],
            serde_json::Value::String("B".into())
        );
        assert_eq!(
            effective
                .parts
                .iter()
                .find(|part| part.id == "right-switch")
                .unwrap()
                .pose
                .at
                .x,
            -40.0
        );
        assert!(!effective.board_references[0].enabled);

        // The returned target contours negate X and reverse the point order.
        let (cache, _, _) = crate::geometry::outlines(&document, None, &[]);
        let (per_board, _, _) = crate::geometry::board_contours(&document, &cache);
        let mut expected = per_board
            .into_iter()
            .find(|entry| entry.board_id == "board")
            .unwrap()
            .contours;
        for contour in &mut expected {
            for point in &mut contour.points {
                point.x = -point.x;
            }
            contour.points.reverse();
        }
        assert_eq!(contours, expected);
        assert!(!contours.is_empty(), "the board outline resolves");
    }
}
