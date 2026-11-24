//! Integration tests for `boardstudio_core::mechanical::edit::apply`.
//!
//! These are the mechanical-settings rules exercised through the public operation:
//! a typed [`MechanicalEdit`] against a [`ProjectDoc`], asserting the stored
//! configuration, the reported changed ids and the rejected values. They were
//! ported from the Case controller's `battery_patch_tests` (its behavioural cases)
//! and cover the settings families the operation owns: battery, closure hardware,
//! critical fits, hardware, mount collections, openings, gasket supports,
//! method/mount/frame switches, dimensions, processes, profiles, stabilizers and
//! the switch family.

use crate::support::fixture;
use crate::support::session;
use boardstudio_core::mechanical::change::{
    MechanicalChange, MechanicalCriticalFitTextField, MechanicalDimension, MechanicalEdit,
    MechanicalHardwareTextField, MechanicalMountCollection, MechanicalTarget,
};
use boardstudio_core::mechanical::edit::{affects_outline, apply, requires_projection};
use boardstudio_core::mechanical::{
    default_gasket_layout, default_internal_gasket, default_plate_foam_thickness,
};
use boardstudio_core::model::{
    Board, GasketPlacement, HardwareConfiguration, HardwareTopology, HardwareTransport,
    InsertInstallation, KicadSource, MechanicalBattery, MechanicalBottomStyle,
    MechanicalBuiltinProfile, MechanicalConfiguration, MechanicalConstructionOwnership,
    MechanicalGasketAnchor, MechanicalMount, MechanicalPartProcess, MechanicalPartProfile,
    MechanicalProfileSource, MechanicalStabilizerKind, MechanicalStabilizerOverride,
    MechanicalSwitchFamily, MountKind, Operation, Part, PartDefinition, PartGenerator, PartKind,
    PhysicalBoardInstance, PlateMethod, ProjectDoc, ScrewDrive, ScrewHeadProfile, ScrewLengthDatum,
    Vec2, Vec3,
};
use std::collections::BTreeMap;

fn board(id: &str, part_ids: Vec<String>) -> Board {
    Board {
        part_ids,
        ..fixture::board(id, id)
    }
}

/// The same configuration with absent closure mounts, for initialization tests.
fn configuration_without_closures() -> MechanicalConfiguration {
    let mut configuration = fixture::mechanical("board");
    configuration.closure_mounts = None;
    configuration
}

fn document() -> ProjectDoc {
    let mut document = ProjectDoc::empty("doc", "Fixture");
    document.boards.push(board("board", vec![]));
    document.mechanical = Some(fixture::mechanical("board"));
    document
}

/// A board whose outline yields at least one suggested closure mount.
fn document_with_outline() -> ProjectDoc {
    let mut document = ProjectDoc::empty("doc", "Fixture");
    document.outline.push(fixture::rect(
        "outline",
        0.0,
        0.0,
        80.0,
        60.0,
        Operation::Add,
    ));
    document.boards.push(board("board", vec![]));
    document.boards[0].outline_ids = vec!["outline".into()];
    document.mechanical = Some(configuration_without_closures());
    document
}

fn apply_change(doc: &mut ProjectDoc, change: MechanicalChange) -> Result<Vec<String>, String> {
    apply(
        doc,
        &[],
        &session::mechanical_edit(session::board_target(), change, true),
    )
}

fn apply_instance(doc: &mut ProjectDoc, change: MechanicalChange) -> Result<Vec<String>, String> {
    apply(
        doc,
        &[],
        &session::mechanical_edit(session::instance_target("left"), change, true),
    )
}

fn stored(doc: &ProjectDoc) -> &MechanicalConfiguration {
    doc.mechanical.as_ref().expect("stored configuration")
}

/// Apply a board change and return the stored configuration, collapsing the
/// ubiquitous apply-then-read assertion pairs in the settings suites.
fn set(doc: &mut ProjectDoc, change: MechanicalChange) -> MechanicalConfiguration {
    apply_change(doc, change).unwrap();
    stored(doc).clone()
}

fn part_definition(id: &str, kind: PartKind, generator: Option<PartGenerator>) -> PartDefinition {
    PartDefinition {
        generator,
        ..fixture::part_definition(id, id, kind)
    }
}

fn part(id: &str, definition_id: &str, keycap: Option<Vec2>) -> Part {
    Part {
        keycap,
        ..fixture::part(id, definition_id, id)
    }
}

fn mx_generator() -> PartGenerator {
    PartGenerator {
        source: "ceoloide/switch_mx".into(),
        version: "1".into(),
        parameters: BTreeMap::new(),
    }
}

fn anchor(id: &str, anchor: f64) -> MechanicalGasketAnchor {
    MechanicalGasketAnchor {
        length: Some(12.0),
        width: Some(3.0),
        placement: Some(GasketPlacement::User),
        id: id.into(),
        region_id: id.into(),
        outline_key: id.into(),
        anchor,
        unlinked: false,
    }
}

#[path = "mechanical_settings_edits/settings_dimensions.rs"]
mod settings_dimensions;
#[path = "mechanical_settings_edits/settings_mounts.rs"]
mod settings_mounts;
#[path = "mechanical_settings_edits/settings_profiles.rs"]
mod settings_profiles;
