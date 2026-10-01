//! Daughterboard snapshots, physical attachment frames and editable circuit ownership.
use crate::model::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
pub enum ModuleCircuitRepair {
    #[serde(rename = "drv2605l-pullups3v3")]
    Drv2605lPullups3v3,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "lowercase")]
pub enum VikSignal {
    Sclk,
    Miso,
    Cs,
    Gpio2,
    Mosi,
    Gpio1,
    V5,
    Rgb,
    Scl,
    Sda,
    Gnd,
    V3v3,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModuleBoard {
    pub contours: Vec<Contour>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "export-types", ts(as = "Option<Vec<Vec<Vec2>>>", optional))]
    pub holes: Vec<Vec<Vec2>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "export-types", ts(optional))]
    pub thickness: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModuleVolume {
    pub id: String,
    pub geometry: CaseOpening,
    pub purpose: String,
    pub source: String,
    pub qualified: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModuleInterface {
    pub id: String,
    pub role: VikRole,
    /// Contacts 1 through 12, in order. Type A cable reverses host/module numbering.
    pub signals: Vec<VikSignal>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum ModuleProtocol {
    PassThrough,
    Spi,
    I2c,
    Nonstandard,
    Gpio,
    MatrixExpansion,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "export-types", ts(optional_fields))]
pub struct ModuleElectrical {
    pub protocol: ModuleProtocol,
    #[serde(default)]
    pub required_signals: Vec<VikSignal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub logic_voltage: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_ma: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub i2c_address: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pullup_ohms: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub driver: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rotary_profile: Option<RotaryProfile>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModuleConstituent {
    pub reference: String,
    pub name: String,
    pub footprint: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "export-types", ts(optional))]
    pub definition_id: Option<String>,
    #[serde(default)]
    pub purchased: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModuleModelCandidate {
    pub asset_id: String,
    pub name: String,
    pub source: HardwareSource,
    pub bounds_min: Vec3,
    pub bounds_max: Vec3,
    /// A candidate can be inspected without claiming it is aligned or complete.
    pub verified_alignment: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModuleCircuit {
    pub definitions: Vec<PartDefinition>,
    pub parts: Vec<Part>,
    pub nets: Vec<Net>,
    /// Named circuit ports refer to local net identities; joining host copper is explicit.
    pub ports: BTreeMap<String, String>,
    pub adaptations: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModuleDefinition {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "export-types", ts(optional))]
    pub catalogue_row: Option<String>,
    pub name: String,
    pub family: String,
    pub variant: String,
    pub source: HardwareSource,
    pub board: ModuleBoard,
    #[serde(default)]
    pub mounts: Vec<MechanicalPcbHole>,
    #[serde(default)]
    pub volumes: Vec<ModuleVolume>,
    #[serde(default)]
    pub openings: Vec<ModuleVolume>,
    #[serde(default)]
    pub models: Vec<PartModel>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(
        feature = "export-types",
        ts(as = "Option<Vec<ModuleModelCandidate>>", optional)
    )]
    pub candidate_models: Vec<ModuleModelCandidate>,
    #[serde(default)]
    pub gates: Vec<HardwareGate>,
    #[serde(default)]
    pub interfaces: Vec<ModuleInterface>,
    pub electrical: ModuleElectrical,
    #[serde(default)]
    pub constituents: Vec<ModuleConstituent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "export-types", ts(optional))]
    pub circuit: Option<ModuleCircuit>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum ModuleAttachment {
    Board,
    Case,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "export-types", ts(optional_fields))]
pub struct ModuleConnection {
    pub host_connector_part_id: String,
    pub module_port_id: String,
    pub bus_id: String,
    /// Assignments are actual MCU terminal names, validated against the placed controller.
    pub assignments: BTreeMap<VikSignal, String>,
    pub cable_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supply_current_ma: Option<f64>,
    #[serde(default)]
    pub rail_voltages: BTreeMap<VikSignal, f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upstream_module_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upstream_port_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "export-types", ts(optional_fields))]
pub struct MountedModule {
    pub id: String,
    pub definition_id: String,
    pub host_board_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host_instance_id: Option<String>,
    pub host_face: Side,
    pub facing_face: Side,
    pub at: Vec2,
    pub rotation: f64,
    pub gap: f64,
    pub attachment: ModuleAttachment,
    #[serde(default)]
    pub detached: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connection: Option<ModuleConnection>,
    /// Deliberate engineering clearance, never a substitute for missing component dimensions.
    #[serde(default)]
    pub service_clearance: f64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(
        feature = "export-types",
        ts(as = "Option<Vec<ModuleSupport>>", optional)
    )]
    pub mount_supports: Vec<ModuleSupport>,
}

/// Designer-selected annular support around one source module mounting hole.
/// Z is relative to the module PCB midplane; it is converted with the module frame.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModuleSupport {
    pub mount_id: String,
    pub outer_diameter: f64,
    pub hole_diameter: f64,
    pub z: f64,
    pub height: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EmbeddedCircuit {
    pub id: String,
    pub definition_id: String,
    pub host_board_id: String,
    pub part_ids: Vec<String>,
    pub net_ids: Vec<String>,
    pub ports: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct ResolvedModule {
    pub id: String,
    pub definition_id: String,
    pub at: Vec2,
    pub rotation: f64,
    /// Midplane in the explicit host-midplane frame.
    pub midplane_z: f64,
    pub flipped: bool,
    /// Solids below use the existing host-top-zero mechanical frame.
    pub board: Vec<CaseOpening>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "export-types", ts(as = "Option<Vec<Contour>>", optional))]
    pub board_holes: Vec<Contour>,
    pub volumes: Vec<ModuleVolume>,
    pub openings: Vec<ModuleVolume>,
    pub mounts: Vec<MechanicalPcbHole>,
    pub models: Vec<PartModel>,
    pub gates: Vec<HardwareGate>,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedModuleSupport {
    pub mount_id: String,
    pub at: Vec2,
    pub outer_diameter: f64,
    pub hole_diameter: f64,
    pub z: f64,
    pub height: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct ModuleModelPlacement {
    pub id: String,
    pub asset_id: String,
    /// Column-major transform into the requested presentation frame, in millimetres.
    pub matrix: [f64; 16],
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct ModuleResolution {
    pub revision: u64,
    pub modules: Vec<ResolvedModule>,
    pub findings: Vec<Finding>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(
        feature = "export-types",
        ts(as = "Option<Vec<FindingMarker>>", optional)
    )]
    pub markers: Vec<FindingMarker>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "export-types", ts(optional))]
    pub preview: Option<PreparedCaseAssemblyIR>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(
        feature = "export-types",
        ts(as = "Option<Vec<ModuleModelPlacement>>", optional)
    )]
    pub model_placements: Vec<ModuleModelPlacement>,
}

mod circuit;
mod electrical;
mod occupancy;
mod placement;
mod preview;
pub(crate) use circuit::{embed, embedded_findings, remove_circuit};
pub(crate) use electrical::{connection_findings, connection_locks, host_requirements};
pub(crate) use occupancy::attach_case_supports;
pub(crate) use occupancy::case_findings;
pub(crate) use placement::finding_markers;
pub(crate) use placement::resolve;
pub(crate) use placement::resolved_mount_supports;
pub(crate) use placement::{remove, set, set_definition};
pub(crate) use preview::prepare_preview;

fn active_instances<'a>(
    doc: &'a ProjectDoc,
    board_id: &'a str,
) -> impl Iterator<Item = &'a MountedModule> {
    doc.modules.iter().filter(move |module| {
        module.host_board_id == board_id
            && !module.detached
            && doc.physical_instance_id.as_ref().is_none_or(|id| {
                module
                    .host_instance_id
                    .as_ref()
                    .is_none_or(|host| host == id)
            })
    })
}
