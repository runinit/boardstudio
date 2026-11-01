//! Forward side of a matrix added on a paired board (ticket 145).
//!
//! `inverse::canonicalize` (ticket 110) answers a `SetMatrix` that creates a matrix on a board a
//! pair relates with the source-equivalent request and a planned [`MatrixAddition`]. The request
//! is applied to the document first; this module then builds the other half, as a draft, from the
//! source matrix that now exists. `reconcile/prepare.rs` calls it; the edit transaction treats
//! the source request and this draft as one step for undo, redo, Open repair and preview.
//!
//! What a transition does, in order:
//! - refuses what cannot receive the pair (see [`mirror_added_matrix`]);
//! - reflects the source matrix through the pair map under the pair's default companion mode and
//!   places it, with its members under the planned identities, on the mirror board;
//! - copies the member attributes the creation path copies (keycap size, outline, lock,
//!   properties, generator parameters) and records one `Matrix` correspondence and one `Part`
//!   correspondence per member pair;
//! - carries what follows the matrix: its layouts, the nets its member pins use, the modules
//!   connected to its members, and its keycap settings.
//!
//! What it leaves alone, and why:
//! - outline envelopes: placing a matrix on a board already lists its members in that board's
//!   envelope features, so each half registers its own members and nothing is copied;
//! - keymap bindings, copper and the board's electrical configuration: never reflected by nature
//!   (ADR-0010), or owned by the wiring edits that follow;
//! - embedded circuits: their members and private nets live under `embedded/<id>/`, never on a
//!   matrix member, so a circuit net that a member pin joins is refused by name (ticket 112).

#[cfg(test)]
#[path = "matrix_addition/tests.rs"]
pub(super) mod tests;

use super::inverse::MatrixAddition;
use super::{
    GENERATED_CLOSURE_PREFIX, ModuleMirror, SYMMETRIC_DEFINITIONS, corresponds, derive, mint,
    mirror_id, mirror_module, mirror_net_name, mirror_reference, remap_wiring_key,
    substitute_token, taken_ids,
};
use crate::matrix::layout;
use crate::model::{
    Layout, Matrix, MirroredCompanionMode, MirroredCompanionTweak, MirroredCorrespondence,
    MirroredHalves, MirroredItemKind, Net, Pin, Pose2, ProjectDoc,
};
use std::collections::{BTreeMap, BTreeSet};

/// The document with the mirror matrix added, and every id the transition created or changed.
pub(super) struct MirroredMatrix {
    pub(super) draft: ProjectDoc,
    /// Sorted and unique.
    pub(super) changed_ids: Vec<String>,
}

/// What the pair says about the board pair, read once.
struct PairFacts {
    pair_index: usize,
    mapping: Pose2,
    halves: MirroredHalves,
    source_instance: Option<String>,
    mirror_instance: Option<String>,
}

fn pair_facts(document: &ProjectDoc, addition: &MatrixAddition) -> Result<PairFacts, String> {
    let hardware = document
        .hardware
        .as_ref()
        .ok_or("Physical instances are unavailable")?;
    let mut found = hardware.pairs.iter().enumerate().filter(|(_, pair)| {
        pair.halves.as_ref().is_some_and(|halves| {
            halves.source_board_id == addition.source_board_id
                && halves.mirror_board_id == addition.mirror_board_id
        })
    });
    let (pair_index, pair) = found.next().ok_or_else(|| {
        format!(
            "No mirrored pair relates board {:?} (source) to board {:?} (mirror), so the matrix \
             {:?} has no half to be mirrored onto",
            addition.source_board_id, addition.mirror_board_id, addition.source_id
        )
    })?;
    if found.next().is_some() {
        return Err(format!(
            "More than one mirrored pair relates {:?} to {:?}",
            addition.source_board_id, addition.mirror_board_id
        ));
    }
    let halves = pair.halves.clone().expect("filtered on halves");
    if halves.source_board_id == halves.mirror_board_id {
        return Err(format!(
            "The pair {:?} reuses one board for both halves; a matrix on it has no second board \
             to be mirrored onto",
            pair.id
        ));
    }
    let source_instance = pair.construction_source.clone();
    let mirror_instance = pair
        .members
        .iter()
        .find(|member| Some(member.as_str()) != source_instance.as_deref())
        .cloned();
    Ok(PairFacts {
        pair_index,
        mapping: pair.mapping,
        halves,
        source_instance,
        mirror_instance,
    })
}

/// Mirrors a matrix the source half now holds onto the mirror half of its pair.
///
/// `document` is the document after the source-equivalent request was applied: it holds the
/// source matrix and its members, and nothing of the mirror. The result is a draft; the input is
/// not changed and a refusal returns no draft.
///
/// Refusals, each naming the identity or record at fault:
/// - no pair relates the two boards, or the pair reuses one board for both halves;
/// - the board pair cannot receive the matrix: its electrical-board correspondence is Local or
///   Omitted (the mirror half has been detached);
/// - the matrix is also linked by a legacy layout mirror link, from either half;
/// - a planned identity has become taken: the mirror matrix, a mirror member, or any identity a
///   following layout, net or module would be minted under;
/// - the source matrix or one of its planned members is not on the source board, or the plan no
///   longer matches the members the matrix generates;
/// - a net a member pin joins belongs to an embedded circuit (ticket 112);
/// - a layout part or module connector has no mirror to follow.
pub(super) fn mirror_added_matrix(
    document: &ProjectDoc,
    addition: &MatrixAddition,
) -> Result<MirroredMatrix, String> {
    let facts = pair_facts(document, addition)?;
    for id in [&addition.source_id, &addition.mirror_id] {
        if layout::has_link(document, id) {
            return Err(format!(
                "The matrix {id:?} is linked by a legacy layout mirror link; unlink it before \
                 the matrix is mirrored by the native pair"
            ));
        }
    }
    let mut source = document
        .matrices
        .iter()
        .find(|matrix| matrix.id == addition.source_id)
        .cloned()
        .ok_or_else(|| format!("The source matrix {:?} is unavailable", addition.source_id))?;
    if source.board_id.as_deref() != Some(addition.source_board_id.as_str()) {
        return Err(format!(
            "The source matrix {:?} is not on board {:?}",
            addition.source_id, addition.source_board_id
        ));
    }
    // Ticket 29: an anchored add stores no cells (members generate with
    // defaults), but the reconciliation kernel needs dense coverage. Fill
    // the missing slots with defaults, exactly what `set_matrix` generated
    // members from; explicit cells are never touched.
    let have: BTreeSet<(u32, u32)> = source
        .cells
        .iter()
        .map(|cell| (cell.row, cell.column))
        .collect();
    if have.len() != source.cells.len() {
        return Err(format!(
            "The source matrix {:?} contains aliased cells",
            addition.source_id
        ));
    }
    let mut draft = document.clone();
    if have.len() != (source.rows as usize) * (source.columns as usize) {
        for row in 0..source.rows {
            for column in 0..source.columns {
                if have.contains(&(row, column)) {
                    continue;
                }
                source.cells.push(crate::model::MatrixCell {
                    shift: None,
                    bend: None,
                    deleted: false,
                    row,
                    column,
                    enabled: true,
                    definition_id: None,
                    variant: None,
                    offset: None,
                    rotation: None,
                    assemblies: Vec::new(),
                    assemblies_local: None,
                });
            }
        }
        let stored = draft
            .matrices
            .iter_mut()
            .find(|matrix| matrix.id == addition.source_id)
            .ok_or_else(|| format!("The source matrix {:?} is unavailable", addition.source_id))?;
        stored.cells = source.cells.clone();
    }
    let plan = plan_identities(document, addition, &source, &facts.halves)?;
    let companion = companion_defaults(&facts.halves)?;
    let reflected = reflected_matrix(document, &source, addition, &facts, companion)?;

    // Correspondences: the matrix, then each member pair in plan order.
    let mut added = vec![corresponds(
        MirroredItemKind::Matrix,
        &addition.source_id,
        &addition.mirror_id,
    )];
    for (source_id, mirror_id) in &plan.pairs {
        let mut link = corresponds(MirroredItemKind::Part, source_id, mirror_id);
        if plan.companions.contains(source_id) {
            link.companion_tweak = companion.tweak;
        }
        added.push(link);
    }
    let mut changed: BTreeSet<String> = BTreeSet::new();
    changed.insert(addition.mirror_id.clone());
    changed.insert(addition.mirror_board_id.clone());
    changed.extend(plan.pairs.iter().map(|(_, mirror)| mirror.clone()));

    let mut taken = taken_ids(&draft);
    let follow = Follow::new(&facts, addition, &plan);
    // An anchored mirror evaluates its anchor at placement, so its layout
    // must exist before `set_matrix`. The layout step skips recorded layouts,
    // so running it early is a no-op for the unanchored path below.
    if reflected.anchor.is_some() {
        follow.layouts(&mut draft, &mut taken, &mut added, &mut changed)?;
    }
    crate::matrix::set_matrix(&mut draft, &reflected)?;
    // The early layout step ran before placement; refresh the taken set with
    // the placed mirror identities, as the original post-placement
    // computation saw them.
    taken.extend(taken_ids(&draft));
    check_generated_members(&draft, addition, &source, &plan)?;
    copy_member_attributes(&mut draft, &plan.pairs)?;
    extend_electrical_configuration(&mut draft, document, addition, &facts.halves, &plan.pairs)?;

    follow.layouts(&mut draft, &mut taken, &mut added, &mut changed)?;
    follow.nets(&mut draft, &mut taken, &mut added, &mut changed)?;
    follow.modules(&mut draft, &mut taken, &mut added, &mut changed)?;
    follow.keycaps(&mut draft, &mut changed)?;

    let halves = draft
        .hardware
        .as_mut()
        .expect("the pair was read from it")
        .pairs[facts.pair_index]
        .halves
        .as_mut()
        .expect("the pair was read with halves");
    halves.correspondences.extend(added);

    layout::validate(&draft)?;
    crate::mechanical::halves::validate_document(&draft)?;
    Ok(MirroredMatrix {
        draft,
        changed_ids: changed.into_iter().collect(),
    })
}

/// Adds only the electrical functions that name this matrix's new members. Electrical-board
/// exceptions are overlays on the board correspondence: a Local board keeps its authored setup,
/// and an Omitted board receives no source-derived setup.
fn extend_electrical_configuration(
    draft: &mut ProjectDoc,
    source_document: &ProjectDoc,
    addition: &MatrixAddition,
    halves: &MirroredHalves,
    member_pairs: &[(String, String)],
) -> Result<(), String> {
    let Some(board_link) = halves.correspondences.iter().find(|item| {
        item.kind == MirroredItemKind::ElectricalBoard
            && item.source_id == addition.source_board_id
            && item.mirror_id == addition.mirror_board_id
    }) else {
        return Ok(());
    };
    if halves.overlay_of(board_link).is_some() {
        return Ok(());
    }
    let source = source_document.hardware.as_ref().and_then(|hardware| {
        hardware
            .boards
            .iter()
            .find(|board| board.board_id == addition.source_board_id)
    });
    let Some(source) = source else {
        return Ok(());
    };
    let mirror = draft
        .hardware
        .as_ref()
        .ok_or("Physical instances are unavailable")?;
    let Some(mirror) = mirror
        .boards
        .iter()
        .find(|board| board.board_id == addition.mirror_board_id)
    else {
        return Ok(());
    };
    let controller_id = mirror.controller_part_id.clone();
    let mut assignments = mirror.assignments.clone();
    let mut locks = mirror.locks.clone();
    let renames: BTreeMap<String, String> = member_pairs.iter().cloned().collect();
    let map_function = |function: &str| remap_wiring_key(function, &renames);

    for (function, pin) in &source.assignments {
        let mapped = map_function(function);
        if mapped == *function {
            continue;
        }
        if !controller_has_pin(draft, controller_id.as_deref(), pin) {
            return Err(format!(
                "The mirror controller {:?} has no pin {pin:?} for matrix assignment {function:?}",
                controller_id
            ));
        }
        if let Some(existing) = assignments.get(&mapped)
            && existing != pin
        {
            return Err(format!(
                "The mirror assignment {mapped:?} for matrix function {function:?} already uses \
                 pin {existing:?}, not {pin:?}"
            ));
        }
        assignments.insert(mapped, pin.clone());
    }
    for (function, pin) in &source.locks {
        let mapped = map_function(function);
        if mapped == *function {
            continue;
        }
        if !controller_has_pin(draft, controller_id.as_deref(), pin) {
            return Err(format!(
                "The mirror controller {:?} has no pin {pin:?} for matrix lock {function:?}",
                controller_id
            ));
        }
        if let Some(existing) = locks.get(&mapped)
            && existing != pin
        {
            return Err(format!(
                "The mirror lock {mapped:?} for matrix function {function:?} already uses \
                 pin {existing:?}, not {pin:?}"
            ));
        }
        locks.insert(mapped, pin.clone());
    }
    let target = draft
        .hardware
        .as_mut()
        .expect("the mirror board configuration was read from hardware")
        .boards
        .iter_mut()
        .find(|board| board.board_id == addition.mirror_board_id)
        .expect("the mirror board configuration remains present");
    target.assignments = assignments;
    target.locks = locks;
    Ok(())
}

/// Assignment values name a terminal or pad on the mirror board's controller. The electrical
/// planner accepts terminal names that resolve to one of the definition's physical pads.
fn controller_has_pin(document: &ProjectDoc, controller_id: Option<&str>, pin: &str) -> bool {
    let Some(controller) =
        controller_id.and_then(|id| document.parts.iter().find(|part| part.id == id))
    else {
        return false;
    };
    let Some(definition) = document
        .definitions
        .iter()
        .find(|item| item.id == controller.definition_id)
    else {
        return false;
    };
    let pad_ids = definition.terminals.get(pin).cloned().unwrap_or_else(|| {
        definition
            .pads
            .iter()
            .filter(|pad| pad.id == pin)
            .map(|pad| pad.id.clone())
            .collect()
    });
    pad_ids.into_iter().any(|id| {
        definition
            .pads
            .iter()
            .any(|pad| pad.id == id && (pad.plated != Some(false) || pad.drill.is_none()))
    })
}

/// The pair's default placement for a new companion.
#[derive(Clone, Copy)]
struct CompanionDefaults {
    mode: MirroredCompanionMode,
    tweak: Option<MirroredCompanionTweak>,
}

/// A pair whose default is a tweak stores the tweak per companion, so a new companion takes the
/// one the pair's existing companions share. Without one, or with several, there is no tweak to
/// choose and the transition refuses rather than guesses.
fn companion_defaults(halves: &MirroredHalves) -> Result<CompanionDefaults, String> {
    let mode = halves.companion_mode;
    let tweak = if mode == MirroredCompanionMode::LiteralMirrorWithTweak {
        let mut tweaks: Vec<MirroredCompanionTweak> = Vec::new();
        for tweak in halves
            .correspondences
            .iter()
            .filter_map(|item| item.companion_tweak)
        {
            if !tweaks.contains(&tweak) {
                tweaks.push(tweak);
            }
        }
        match tweaks.as_slice() {
            [only] => Some(*only),
            [] => {
                return Err(
                    "The pair places companions with a tweak, but no companion stores one \
                            to give the new matrix's companions"
                        .into(),
                );
            }
            _ => {
                return Err(
                    "The pair's companions store different tweaks, so a new matrix's \
                            companions have no one tweak to take"
                        .into(),
                );
            }
        }
    } else {
        None
    };
    Ok(CompanionDefaults { mode, tweak })
}

/// Whether the cell named by `slot` (`r<row>c<column>`) keeps its companions local to the source.
fn local_slot(matrix: &Matrix, slot: &str) -> bool {
    matrix.cells.iter().any(|cell| {
        cell.assemblies_local == Some(true) && slot == format!("r{}c{}", cell.row, cell.column)
    })
}

/// The member pairs the matrix mirrors, checked against the document.
struct Identities {
    /// `(source id, mirror id)` for each member that exists on both halves, in plan order.
    pairs: Vec<(String, String)>,
    /// Source ids of the pairs that are companions of a key.
    companions: BTreeSet<String>,
}

fn plan_identities(
    document: &ProjectDoc,
    addition: &MatrixAddition,
    source: &Matrix,
    halves: &MirroredHalves,
) -> Result<Identities, String> {
    let taken = taken_ids(document);
    if taken.contains(&addition.mirror_id) {
        return Err(format!(
            "The planned mirror matrix {:?} is already in use",
            addition.mirror_id
        ));
    }
    for correspondence in &halves.correspondences {
        let names = |id: &str| correspondence.source_id == id || correspondence.mirror_id == id;
        if correspondence.kind == MirroredItemKind::Matrix
            && (names(&addition.source_id) || names(&addition.mirror_id))
        {
            return Err(format!(
                "The matrix {:?} or {:?} is already named by a correspondence \
                 ({:?} to {:?})",
                addition.source_id,
                addition.mirror_id,
                correspondence.source_id,
                correspondence.mirror_id
            ));
        }
    }
    let source_prefix = format!("matrix/{}/", addition.source_id);
    let mirror_prefix = format!("matrix/{}/", addition.mirror_id);
    let named: BTreeSet<&str> = halves
        .correspondences
        .iter()
        .flat_map(|item| [item.source_id.as_str(), item.mirror_id.as_str()])
        .collect();
    let mut pairs = Vec::new();
    let mut companions = BTreeSet::new();
    let mut seen = BTreeSet::new();
    for (source_id, mirror_id) in &addition.members {
        let rest = source_id.strip_prefix(&source_prefix).ok_or_else(|| {
            format!(
                "The planned member {source_id:?} is not named under {source_prefix:?}; the plan \
                 no longer matches the matrix"
            )
        })?;
        if mirror_id != &format!("{mirror_prefix}{rest}") || !seen.insert(source_id.as_str()) {
            return Err(format!(
                "The planned mirror member {mirror_id:?} does not follow its source {source_id:?} \
                 under {mirror_prefix:?}"
            ));
        }
        if !source.part_ids.contains(source_id)
            || !document.parts.iter().any(|part| &part.id == source_id)
            || !document.boards.iter().any(|board| {
                board.id == addition.source_board_id && board.part_ids.contains(source_id)
            })
        {
            return Err(format!(
                "The planned source member {source_id:?} is not a part of the source matrix on \
                 board {:?}",
                addition.source_board_id
            ));
        }
        if taken.contains(mirror_id) || named.contains(mirror_id.as_str()) {
            return Err(format!(
                "The planned mirror member {mirror_id:?} is already in use"
            ));
        }
        if named.contains(source_id.as_str()) {
            return Err(format!(
                "The source member {source_id:?} already has a correspondence"
            ));
        }
        if let Some((slot, _)) = rest.split_once('/') {
            // A cell whose companions are local to the source has none on the mirror: the
            // reflection leaves them to the mirror half, so they are not a pair.
            if local_slot(source, slot) {
                continue;
            }
            companions.insert(source_id.clone());
        }
        pairs.push((source_id.clone(), mirror_id.clone()));
    }
    // Members the plan does not name would be left unlinked.
    for member in &source.part_ids {
        if seen.contains(member.as_str()) {
            continue;
        }
        let local = member
            .strip_prefix(&source_prefix)
            .and_then(|rest| rest.split_once('/'))
            .is_some_and(|(slot, _)| local_slot(source, slot));
        if !local {
            return Err(format!(
                "The source member {member:?} is not in the plan for {:?}",
                addition.source_id
            ));
        }
    }
    Ok(Identities { pairs, companions })
}

/// Map a mirrored matrix's anchor onto the mirror half: every anchor target
/// names its mirrored counterpart through the pair's matrix correspondences.
/// Row/column stay the physical key and Follow crosses unchanged. A target
/// with no counterpart refuses instead of pointing across halves.
fn remap_mirrored_anchor(
    reflected: &mut Matrix,
    halves: &MirroredHalves,
    mirror_id: &str,
) -> Result<(), String> {
    let Some(anchor) = reflected.anchor.as_mut() else {
        return Ok(());
    };
    let counterpart = |matrix_id: &str| {
        halves
            .correspondences
            .iter()
            .find(|record| record.kind == MirroredItemKind::Matrix && record.source_id == matrix_id)
            .map(|record| record.mirror_id.clone())
    };
    match &mut anchor.aggregate {
        Some(aggregate) => {
            for part in &mut aggregate.parts {
                part.matrix_id = counterpart(&part.matrix_id).ok_or_else(|| {
                    format!(
                        "The mirrored anchor of {mirror_id:?} names {part:?}, which has no \
                         mirrored counterpart to follow"
                    )
                })?;
            }
        }
        None => {
            anchor.matrix_id = counterpart(&anchor.matrix_id).ok_or_else(|| {
                format!(
                    "The mirrored anchor of {mirror_id:?} targets {:?}, which has no mirrored \
                     counterpart to follow",
                    anchor.matrix_id
                )
            })?;
        }
    }
    Ok(())
}

/// The reflection of the source matrix, with each companion placed under the pair's default.
fn reflected_matrix(
    document: &ProjectDoc,
    source: &Matrix,
    addition: &MatrixAddition,
    facts: &PairFacts,
    companion: CompanionDefaults,
) -> Result<Matrix, String> {
    let mut template = source.clone();
    template.id = addition.mirror_id.clone();
    template.board_id = Some(addition.mirror_board_id.clone());
    template.cells.clear();
    template.part_ids.clear();
    template.mirror = None;
    if let Some(name) = &source.name {
        template.name = Some(
            substitute_token(name, &addition.source_board_id, &addition.mirror_board_id)
                .unwrap_or_else(|| name.clone()),
        );
    }
    let mut reflected = layout::reflected(source, &template, facts.mapping, false, false)?;
    remap_mirrored_anchor(&mut reflected, &facts.halves, &addition.mirror_id)?;
    for cell in &mut reflected.cells {
        if !cell.enabled || cell.deleted {
            continue;
        }
        for assembly in &mut cell.assemblies {
            let key_id = crate::matrix::member_id(&source.id, cell.row, cell.column);
            let companion_id = crate::matrix::companion_id(&key_id, &assembly.id);
            let stabiliser = SYMMETRIC_DEFINITIONS.contains(&assembly.definition_id.as_str());
            let needs_source_parts =
                stabiliser || companion.mode == MirroredCompanionMode::DuplicatePosition;
            let (key, part) = if needs_source_parts {
                let find = |id: &str, what: &str| {
                    document
                        .parts
                        .iter()
                        .find(|part| part.id == id)
                        .ok_or_else(|| format!("The source {what} {id:?} is unavailable"))
                };
                (
                    Some(find(&key_id, "switch")?),
                    Some(find(&companion_id, "companion")?),
                )
            } else {
                (None, None)
            };
            let (offset, rotation) = derive::companion_assembly(derive::CompanionAssemblyInput {
                offset: assembly.offset,
                rotation: assembly.rotation,
                key,
                companion: part,
                mapping: facts.mapping,
                mode: companion.mode,
                tweak: companion.tweak,
                stabiliser,
                fallback: false,
            })?;
            assembly.offset = offset;
            assembly.rotation = rotation;
        }
    }
    Ok(reflected)
}

/// The members `set_matrix` placed are exactly the planned mirror members.
fn check_generated_members(
    draft: &ProjectDoc,
    addition: &MatrixAddition,
    source: &Matrix,
    plan: &Identities,
) -> Result<(), String> {
    let mirror = draft
        .matrices
        .iter()
        .find(|matrix| matrix.id == addition.mirror_id)
        .ok_or_else(|| format!("The mirror matrix {:?} was not placed", addition.mirror_id))?;
    let planned: BTreeSet<&str> = plan
        .pairs
        .iter()
        .map(|(_, mirror)| mirror.as_str())
        .collect();
    let placed: BTreeSet<&str> = mirror.part_ids.iter().map(String::as_str).collect();
    if planned != placed {
        let extra: Vec<_> = placed.difference(&planned).collect();
        let missing: Vec<_> = planned.difference(&placed).collect();
        return Err(format!(
            "The mirror of {:?} generated members the plan does not match (unplanned {extra:?}, \
             missing {missing:?})",
            source.id
        ));
    }
    Ok(())
}

/// A mirror key or companion keeps the source's authored attributes, as creation copies them.
fn copy_member_attributes(
    draft: &mut ProjectDoc,
    pairs: &[(String, String)],
) -> Result<(), String> {
    for (source_id, mirror_id) in pairs {
        let source = draft
            .parts
            .iter()
            .find(|part| &part.id == source_id)
            .cloned()
            .ok_or_else(|| format!("The source member {source_id:?} is unavailable"))?;
        let placed = draft
            .parts
            .iter_mut()
            .find(|part| &part.id == mirror_id)
            .ok_or_else(|| format!("The mirror of member {source_id:?} was not placed"))?;
        placed.keycap = source.keycap;
        placed.outline = source.outline.clone();
        placed.locked = source.locked;
        placed.properties = source.properties.clone();
        placed.generator_parameters = source.generator_parameters.clone();
    }
    Ok(())
}

/// What follows the matrix, resolved against the correspondences the pair holds plus the new
/// member pairs.
struct Follow<'a> {
    source_board: &'a str,
    mirror_board: &'a str,
    source_matrix: &'a str,
    mirror_matrix: &'a str,
    mapping: Pose2,
    halves: MirroredHalves,
    source_instance: Option<&'a str>,
    mirror_instance: Option<&'a str>,
    /// Source part to mirror part, for every part that is linked (no Local or Omitted overlay).
    parts: BTreeMap<String, String>,
    new_members: BTreeSet<String>,
    pairs: &'a [(String, String)],
}

impl<'a> Follow<'a> {
    fn new(facts: &'a PairFacts, addition: &'a MatrixAddition, plan: &'a Identities) -> Self {
        let mut parts = BTreeMap::new();
        for correspondence in facts
            .halves
            .correspondences
            .iter()
            .filter(|item| item.kind == MirroredItemKind::Part)
        {
            // A Local or Omitted part has no linked mirror to follow.
            if facts.halves.overlay_of(correspondence).is_none() {
                parts.insert(
                    correspondence.source_id.clone(),
                    correspondence.mirror_id.clone(),
                );
            }
        }
        parts.extend(plan.pairs.iter().cloned());
        Self {
            source_board: &addition.source_board_id,
            mirror_board: &addition.mirror_board_id,
            source_matrix: &addition.source_id,
            mirror_matrix: &addition.mirror_id,
            mapping: facts.mapping,
            halves: facts.halves.clone(),
            source_instance: facts.source_instance.as_deref(),
            mirror_instance: facts.mirror_instance.as_deref(),
            parts,
            new_members: plan
                .pairs
                .iter()
                .map(|(source, _)| source.clone())
                .collect(),
            pairs: &plan.pairs,
        }
    }

    fn has_correspondence(
        &self,
        added: &[MirroredCorrespondence],
        kind: MirroredItemKind,
        source_id: &str,
    ) -> bool {
        self.halves
            .correspondences
            .iter()
            .chain(added)
            .any(|item| item.kind == kind && item.source_id == source_id)
    }

    /// The mirror layout already following `source_id`, if any.
    fn layout_correspondence(
        &self,
        added: &[MirroredCorrespondence],
        source_id: &str,
    ) -> Option<String> {
        self.halves
            .correspondences
            .iter()
            .chain(added)
            .find(|item| item.kind == MirroredItemKind::Layout && item.source_id == source_id)
            .map(|item| item.mirror_id.clone())
    }

    /// A layout of the source matrix becomes a layout of the mirror matrix on the mirror board.
    fn layouts(
        &self,
        draft: &mut ProjectDoc,
        taken: &mut BTreeSet<String>,
        added: &mut Vec<MirroredCorrespondence>,
        changed: &mut BTreeSet<String>,
    ) -> Result<(), String> {
        let sources: Vec<Layout> = draft
            .layouts
            .iter()
            .filter(|item| {
                item.board_id == self.source_board
                    && item.matrix_ids.iter().any(|id| id == self.source_matrix)
            })
            .cloned()
            .collect();
        for source in sources {
            // An anchored add joins its target's layout, which already
            // mirrors: the new mirror matrix joins the existing mirror
            // layout instead of minting a second one.
            if let Some(existing) = self.layout_correspondence(added, &source.id) {
                let layout = draft
                    .layouts
                    .iter_mut()
                    .find(|layout| layout.id == existing)
                    .ok_or_else(|| format!("The mirror layout {existing:?} is unavailable"))?;
                if !layout.matrix_ids.iter().any(|id| id == self.mirror_matrix) {
                    layout.matrix_ids.push(self.mirror_matrix.to_owned());
                    changed.insert(existing.clone());
                }
                continue;
            }
            let id = mint(
                taken,
                mirror_reference(self.source_board, self.mirror_board, &source.id),
            )?;
            let part_ids = source
                .part_ids
                .iter()
                .filter(|part| !part.starts_with(GENERATED_CLOSURE_PREFIX))
                .map(|part| {
                    self.parts.get(part).cloned().ok_or_else(|| {
                        format!(
                            "The layout {:?} names {part:?}, which has no linked mirror to follow",
                            source.id
                        )
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            draft.layouts.push(Layout {
                id: id.clone(),
                name: source.name.clone(),
                board_id: self.mirror_board.to_owned(),
                matrix_ids: source
                    .matrix_ids
                    .iter()
                    .map(|id| {
                        if id == self.source_matrix {
                            Ok(self.mirror_matrix.to_owned())
                        } else {
                            draft
                                .hardware
                                .as_ref()
                                .into_iter()
                                .flat_map(|h| &h.pairs)
                                .filter_map(|p| p.halves.as_ref())
                                .flat_map(|h| &h.correspondences)
                                .find(|c| c.kind == MirroredItemKind::Matrix && c.source_id == *id)
                                .map(|c| c.mirror_id.clone())
                                .ok_or("Layout matrix correspondence is missing")
                        }
                    })
                    .collect::<Result<Vec<_>, _>>()?,
                part_ids,
                mirror_link: None,
            });
            added.push(corresponds(MirroredItemKind::Layout, &source.id, &id));
            changed.insert(id);
        }
        Ok(())
    }

    /// Each net a new member pin joins: an existing linked net gains the mapped pins, any other
    /// becomes a new mirror net with a correspondence.
    fn nets(
        &self,
        draft: &mut ProjectDoc,
        taken: &mut BTreeSet<String>,
        added: &mut Vec<MirroredCorrespondence>,
        changed: &mut BTreeSet<String>,
    ) -> Result<(), String> {
        let touched: Vec<Net> = draft
            .nets
            .iter()
            .filter(|net| {
                net.pins
                    .iter()
                    .any(|pin| self.new_members.contains(&pin.part_id))
            })
            .cloned()
            .collect();
        for net in touched {
            if net.id.starts_with("embedded/")
                || draft.embedded_circuits.iter().any(|circuit| {
                    circuit.host_board_id == self.source_board && circuit.net_ids.contains(&net.id)
                })
            {
                return Err(format!(
                    "The net {:?} belongs to an embedded circuit and a new matrix member joins it; \
                     ticket 112 translates hosted circuit edits",
                    net.id
                ));
            }
            let existing = self
                .halves
                .correspondences
                .iter()
                .find(|item| item.kind == MirroredItemKind::Net && item.source_id == net.id);
            if let Some(link) = existing {
                if self.halves.overlay_of(link).is_some() {
                    continue;
                }
                let mirror = draft
                    .nets
                    .iter_mut()
                    .find(|item| item.id == link.mirror_id)
                    .ok_or_else(|| format!("The mirror net {:?} is unavailable", link.mirror_id))?;
                for pin in net
                    .pins
                    .iter()
                    .filter(|pin| self.new_members.contains(&pin.part_id))
                {
                    let mapped = Pin {
                        part_id: self.parts[&pin.part_id].clone(),
                        pad_id: pin.pad_id.clone(),
                    };
                    if !mirror.pins.contains(&mapped) {
                        mirror.pins.push(mapped);
                    }
                }
                changed.insert(link.mirror_id.clone());
                continue;
            }
            let id = mint(
                taken,
                mirror_id(self.source_board, self.mirror_board, &net.id),
            )?;
            let pins = net
                .pins
                .iter()
                .filter_map(|pin| {
                    self.parts.get(&pin.part_id).map(|part_id| Pin {
                        part_id: part_id.clone(),
                        pad_id: pin.pad_id.clone(),
                    })
                })
                .collect();
            draft.nets.push(Net {
                id: id.clone(),
                name: mirror_net_name(self.source_board, self.mirror_board, &net.name),
                pins,
            });
            let on_source = draft
                .boards
                .iter()
                .any(|board| board.id == self.source_board && board.net_ids.contains(&net.id));
            if on_source
                && let Some(board) = draft
                    .boards
                    .iter_mut()
                    .find(|board| board.id == self.mirror_board)
            {
                board.net_ids.push(id.clone());
            }
            added.push(corresponds(MirroredItemKind::Net, &net.id, &id));
            changed.insert(id);
        }
        Ok(())
    }

    /// A mounted module plugged into a new member is mirrored as the creation path mirrors one.
    fn modules(
        &self,
        draft: &mut ProjectDoc,
        taken: &mut BTreeSet<String>,
        added: &mut Vec<MirroredCorrespondence>,
        changed: &mut BTreeSet<String>,
    ) -> Result<(), String> {
        let sources: Vec<_> = draft
            .modules
            .iter()
            .filter(|module| {
                module.host_board_id == self.source_board
                    && module.connection.as_ref().is_some_and(|connection| {
                        self.new_members
                            .contains(&connection.host_connector_part_id)
                    })
            })
            .cloned()
            .collect();
        if sources.is_empty() {
            return Ok(());
        }
        let mut modules: BTreeMap<String, String> = self
            .halves
            .correspondences
            .iter()
            .filter(|item| item.kind == MirroredItemKind::Module)
            .map(|item| (item.source_id.clone(), item.mirror_id.clone()))
            .collect();
        for module in &sources {
            if modules.contains_key(&module.id) {
                continue;
            }
            let id = mint(
                taken,
                mirror_id(self.source_board, self.mirror_board, &module.id),
            )?;
            modules.insert(module.id.clone(), id);
        }
        let source_instance = self.source_instance.unwrap_or_default();
        let mirror_instance = self.mirror_instance.unwrap_or_default();
        let context = ModuleMirror {
            from: self.source_board,
            to: self.mirror_board,
            source_instance,
            mirror_instance,
            mapping: self.mapping,
            parts: &self.parts,
            // A new member is never Omitted, so a module plugged into one keeps its connection.
            omitted_parts: &BTreeSet::new(),
            modules: &modules,
        };
        for module in &sources {
            if self.has_correspondence(added, MirroredItemKind::Module, &module.id) {
                continue;
            }
            let mirrored = mirror_module(module, &context)?;
            added.push(corresponds(
                MirroredItemKind::Module,
                &module.id,
                &mirrored.id,
            ));
            changed.insert(mirrored.id.clone());
            draft.modules.push(mirrored);
        }
        Ok(())
    }

    /// Keycap settings are copied to the mirror as they are; a setting already on a mirror id is
    /// stale data and refuses rather than being overwritten.
    fn keycaps(
        &self,
        draft: &mut ProjectDoc,
        changed: &mut BTreeSet<String>,
    ) -> Result<(), String> {
        let Some(keycaps) = draft.keycaps.as_mut() else {
            return Ok(());
        };
        let stale = |key: &str| format!("A keycap setting already names {key:?}");
        if let Some(settings) = keycaps.matrices.get(self.source_matrix).cloned() {
            if keycaps.matrices.contains_key(self.mirror_matrix) {
                return Err(stale(self.mirror_matrix));
            }
            keycaps
                .matrices
                .insert(self.mirror_matrix.to_owned(), settings);
        }
        for (source, mirror) in self.pairs {
            if let Some(settings) = keycaps.keys.get(source).cloned() {
                if keycaps.keys.contains_key(mirror) {
                    return Err(stale(mirror));
                }
                keycaps.keys.insert(mirror.clone(), settings);
                changed.insert(mirror.clone());
            }
        }
        Ok(())
    }
}
