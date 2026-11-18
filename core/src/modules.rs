//! Daughterboard snapshots, physical attachment frames and editable circuit ownership.
use crate::model::*;
use std::collections::{BTreeMap, BTreeSet};

#[path = "modules/host_connector.rs"]
mod host_connector;

/// Allocate a unique component reference for an embedded circuit member.
pub(crate) fn allocate_reference(source: &str, used: &mut BTreeSet<String>) -> String {
    if !source.is_empty() && used.insert(source.to_owned()) {
        return source.to_owned();
    }
    let prefix = source.trim_end_matches(|c: char| c.is_ascii_digit());
    let prefix = if prefix.is_empty() { "U" } else { prefix };
    for index in 1.. {
        let candidate = format!("{prefix}{index}");
        if used.insert(candidate.clone()) {
            return candidate;
        }
    }
    unreachable!("A finite circuit cannot exhaust component references")
}

pub use boardstudio_model::modules::*;
#[derive(Clone, Debug)]
pub(crate) struct ResolvedModuleSupport {
    pub mount_id: String,
    pub at: Vec2,
    pub outer_diameter: f64,
    pub hole_diameter: f64,
    pub z: f64,
    pub height: f64,
}

mod circuit;
mod electrical;
mod occupancy;
mod placement;
pub(crate) use placement::placed;
mod preview;
pub(crate) use circuit::{embed, embedded_findings, remove_circuit};
pub(crate) use electrical::{connection_findings, connection_locks, host_requirements};
pub(crate) use occupancy::case_findings;
pub(crate) use occupancy::{attach_board_supports, attach_case_supports};
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
