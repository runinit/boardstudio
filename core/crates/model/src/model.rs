pub use crate::hardware::{HardwareGate, HardwareOutput, HardwareProfile, HardwareSource, VikRole};
pub use crate::inputs::{EncoderDriver, InputProfile, PressContacts, PressScanMode, RotaryProfile};
pub use crate::keymap::{
    EncoderBinding, EncoderDirection, KeyBinding, KeymapChange, KeymapConfiguration, KeymapLayer,
    KeymapMacro, MacroChange, MacroStep,
};
pub use crate::modules::{
    EmbeddedCircuit, ModuleAttachment, ModuleBoard, ModuleCircuit, ModuleCircuitRepair,
    ModuleConnection, ModuleConstituent, ModuleDefinition, ModuleElectrical, ModuleInterface,
    ModuleModelCandidate, ModuleModelPlacement, ModuleProtocol, ModuleResolution, ModuleSupport,
    ModuleSupportGeometry, ModuleVolume, MountedModule, ResolvedModule, ResolvedModuleFootprint,
    ResolvedModulePad, VikSignal,
};
pub use boardstudio_contracts::{
    AffineTransformError, CadStepImportOptions, MAX_COORDINATE_MM, is_sha256_hex,
    validate_affine_row_transform,
};
pub use boardstudio_contracts::{
    CaseAssemblyIR, CaseBody, CaseFeature, CaseIR, CaseKind, CaseOpening, CasePcbCircularSupport,
    CasePcbSlot, CasePcbSource, CasePcbSourceContour, CasePcbSourcePrimitive, Contour, Gasket,
    Mount, MountKind, PreparedCaseAssemblyIR, PreparedCaseIR, PreparedCaseRegion,
    PreparedGasketRegion,
};
pub use boardstudio_contracts::{
    KeycapBoardSettings, KeycapConfiguration, KeycapKeySettings, KeycapMatrixSettings, KeycapMount,
    KeycapProfile, KeycapSpec, Pose2, Side, Vec2,
};
pub use boardstudio_contracts::{MechanicalDrillShape, MechanicalShape};
pub use boardstudio_contracts::{polygon_contains, polygon_signed_area};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[path = "model/components.rs"]
mod components;
#[path = "model/document.rs"]
mod document;
#[path = "model/mechanical.rs"]
mod mechanical;

pub use self::components::*;
pub use self::document::*;
pub use self::mechanical::*;
