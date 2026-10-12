//! Inverse-map translation of requests that target a linked mirror (ticket 110).
//!
//! An edit made on a linked mirror is an edit of the authored source value (ADR-0010). This
//! private layer rewrites such a request into the equivalent source request, using the pair map
//! and the stored correspondences, before the existing operation executes. Nothing calls it
//! yet: ticket 115 composes the result through apply and resolution, and 111 activates it.
//!
//! Where things live (search the names; line numbers drift):
//! - Seam: [`canonicalize`] returns a [`Canonical`] request; [`coverage`] is the exhaustive
//!   ledger of every `EditOperation` variant and who translates it.
//! - Targets: [`Links`] resolves an id to Not-a-mirror, Local, Omitted or Linked.
//! - Parts: `move_parts`, `add_part`, `remove_parts`; companion placement is `source_position`.
//!
//! Rules this layer keeps:
//! - A target that is not a linked mirror (unpaired, a source, or a Local exception) executes
//!   as written. An Omitted target is refused: its geometry is absent and is not recreated.
//! - Positions and headings use the pair's exact inverse (`pair::inverse`, `inverse_angle`).
//!   Switches take no half turn; independent parts take the full-circle rule; companions follow
//!   their stored placement mode and stabilisers keep their world heading.
//! - Two entries that resolve to one canonical target are one action when they agree and a
//!   refusal when they do not; they are never executed twice.

#[cfg(test)]
#[path = "inverse/tests.rs"]
mod tests;

use crate::mechanical::pair;
use crate::model::{
    EditOperation, Matrix, MatrixSplayChange, Mirror, MirroredCompanionMode,
    MirroredCompanionTweak, MirroredCorrespondence, MirroredExceptionState, MirroredHalves,
    MirroredItemKind, Part, Pose2, Position, ProjectDoc, Vec2,
};
use std::collections::{BTreeMap, BTreeSet};

/// Cell coordinates chosen per matrix id.
type CellSelection = BTreeMap<String, Vec<(u32, u32)>>;

/// Two requested values of one canonical target agree when they differ by less than this.
const AGREEMENT_TOLERANCE_MM: f64 = 1e-6;

/// Who translates an operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Coverage {
    /// Ticket 110 (this module).
    Translated,
    /// Another slice of ticket 19 owns the family; this module refuses it rather than let it
    /// pass through untranslated.
    OwnedBy(&'static str),
}

/// The exhaustive ledger. There is no wildcard arm: a new `EditOperation` variant does not
/// compile until it is given an owner.
pub(super) fn coverage(operation: &EditOperation) -> Coverage {
    use EditOperation as Op;
    match operation {
        Op::SetMatrix { .. }
        | Op::SetMatrixSplay { .. }
        | Op::SetMatrixPlacement { .. }
        | Op::RemoveMatrix { .. }
        | Op::SetMatrixCellsEnabled { .. }
        | Op::RemoveSelectedMatrixCells { .. }
        | Op::RemoveMatrixCells { .. }
        | Op::RemoveMatrixRow { .. }
        | Op::RemoveMatrixColumn { .. }
        | Op::MoveParts { .. }
        | Op::AddPart { .. }
        | Op::RemoveParts { .. } => Coverage::Translated,
        // Ticket 112: hosted, electrical and keycap families.
        Op::SetModuleDefinition { .. }
        | Op::SetMountedModule { .. }
        | Op::RemoveMountedModule { .. }
        | Op::EmbedModuleCircuit { .. }
        | Op::RemoveEmbeddedCircuit { .. }
        | Op::SetNet { .. }
        | Op::SetInputScanMode { .. }
        | Op::SetWiringMode { .. }
        | Op::SetKeycapBoard { .. }
        | Op::SetMatrixKeycaps { .. }
        | Op::SetKeycapKey { .. } => Coverage::OwnedBy("112"),
        // Ticket 113: outline and reviewed geometry.
        Op::SetOutline { .. }
        | Op::CopyOutline { .. }
        | Op::SelectOutline { .. }
        | Op::RenameOutline { .. }
        | Op::RemoveOutline { .. }
        | Op::ApplyOutlineEdit { .. } => Coverage::OwnedBy("113"),
        // Ticket 114: compatibility, constraints and the final exhaustive ledger.
        Op::EditKeymap { .. }
        | Op::SetKeyBinding { .. }
        | Op::SetLayout { .. }
        | Op::CreateMirroredPair { .. }
        | Op::SetConstraint { .. }
        | Op::RemoveConstraint { .. }
        | Op::ConvertOffsetConstraint { .. }
        | Op::ApplyMechanicalEdit { .. }
        | Op::SetMechanical { .. }
        | Op::SetCase { .. }
        | Op::CreateMirroredHalf { .. }
        | Op::ReplaceDocument { .. } => Coverage::OwnedBy("114"),
    }
}

/// A part the request adds on a mirror board, as the forward kernel's independent Add
/// transition wants it: the source identity this layer allocated beside the mirror identity
/// the caller chose.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct PartAddition {
    pub(super) source_id: String,
    pub(super) mirror_id: String,
    pub(super) mirror_definition_id: String,
    pub(super) mirror_reference: String,
}

/// The source-equivalent request.
#[derive(Debug)]
pub(super) struct Canonical {
    /// The request to execute, apply and resolve; downstream resolution must see this value too.
    pub(super) operation: EditOperation,
    /// Whether any target was rewritten.
    pub(super) translated: bool,
    /// Mirror-side part additions, one per translated `AddPart`.
    pub(super) additions: Vec<PartAddition>,
}

/// Rewrites `operation` so that every linked mirror target names its authored source value.
pub(super) fn canonicalize(
    document: &ProjectDoc,
    operation: &EditOperation,
) -> Result<Canonical, String> {
    if let Coverage::OwnedBy(ticket) = coverage(operation) {
        return Err(format!(
            "This request is translated by ticket {ticket}, not by the matrix and part inverse"
        ));
    }
    let links = Links::new(document);
    let mut canonical = Canonical {
        operation: operation.clone(),
        translated: false,
        additions: Vec::new(),
    };
    match operation {
        EditOperation::MoveParts { positions } => {
            let positions = links.move_parts(positions)?;
            canonical.translated = positions.1;
            canonical.operation = EditOperation::MoveParts {
                positions: positions.0,
            };
        }
        EditOperation::AddPart { part, board_id } => {
            if let Some((source, addition)) = links.add_part(part, board_id.as_deref())? {
                canonical.operation = source;
                canonical.additions.push(addition);
                canonical.translated = true;
            }
        }
        EditOperation::RemoveParts { ids } => {
            let (ids, translated) = links.remove_parts(ids)?;
            canonical.translated = translated;
            canonical.operation = EditOperation::RemoveParts { ids };
        }
        EditOperation::RemoveMatrix { id } => {
            let (id, translated) = links.matrix_id(id)?;
            canonical.translated = translated;
            canonical.operation = EditOperation::RemoveMatrix { id };
        }
        EditOperation::RemoveMatrixCells { matrix_id, cells } => {
            let (matrix_id, translated) = links.matrix_id(matrix_id)?;
            canonical.translated = translated;
            canonical.operation = EditOperation::RemoveMatrixCells {
                matrix_id,
                cells: cells.clone(),
            };
        }
        EditOperation::RemoveMatrixRow { matrix_id, row } => {
            let (matrix_id, translated) = links.matrix_id(matrix_id)?;
            canonical.translated = translated;
            canonical.operation = EditOperation::RemoveMatrixRow {
                matrix_id,
                row: *row,
            };
        }
        EditOperation::RemoveMatrixColumn { matrix_id, column } => {
            let (matrix_id, translated) = links.matrix_id(matrix_id)?;
            canonical.translated = translated;
            canonical.operation = EditOperation::RemoveMatrixColumn {
                matrix_id,
                column: *column,
            };
        }
        EditOperation::SetMatrixCellsEnabled { cells, enabled } => {
            let (cells, translated) = links.selected_cells(cells)?;
            canonical.translated = translated;
            canonical.operation = EditOperation::SetMatrixCellsEnabled {
                cells,
                enabled: *enabled,
            };
        }
        EditOperation::RemoveSelectedMatrixCells { cells } => {
            let (cells, translated) = links.selected_cells(cells)?;
            canonical.translated = translated;
            canonical.operation = EditOperation::RemoveSelectedMatrixCells { cells };
        }
        EditOperation::SetMatrixSplay {
            matrix_id,
            column,
            change,
        } => {
            let (id, mapping) = links.matrix_frame(matrix_id)?;
            canonical.translated = mapping.is_some();
            // An origin is a world point, so it takes the pair's inverse. An angle is a value
            // of the matrix's own frame, which the mirror shares with the source.
            let change = match (change, mapping) {
                (MatrixSplayChange::Origin { world: Some(world) }, Some(mapping)) => {
                    MatrixSplayChange::Origin {
                        world: Some(pair::inverse(mapping, *world)),
                    }
                }
                (change, _) => change.clone(),
            };
            canonical.operation = EditOperation::SetMatrixSplay {
                matrix_id: id,
                column: *column,
                change,
            };
        }
        EditOperation::SetMatrixPlacement { matrix_id, change } => {
            // Every placement value is in the matrix's own frame, shared with the source.
            let (id, translated) = links.matrix_id(matrix_id)?;
            canonical.translated = translated;
            canonical.operation = EditOperation::SetMatrixPlacement {
                matrix_id: id,
                change: change.clone(),
            };
        }
        EditOperation::SetMatrix {
            matrix,
            definitions,
        } => {
            if let Some(source) = links.set_matrix(matrix)? {
                canonical.translated = true;
                canonical.operation = EditOperation::SetMatrix {
                    matrix: source,
                    definitions: definitions.clone(),
                };
            }
        }
        _ => return Err("This request is not an inverse-translated family".into()),
    }
    Ok(canonical)
}

/// One pair with a `halves` record.
struct PairView<'a> {
    mapping: Pose2,
    halves: &'a MirroredHalves,
}

/// What an id is, relative to the pairs of a document.
enum Target<'a> {
    /// Not the mirror side of any correspondence: it executes as written.
    NotMirror,
    /// A mirror kept as an independent local item: it executes as written.
    Local,
    /// A mirror that is absent by exception.
    Omitted,
    Linked(Link<'a>),
}

#[derive(Clone, Copy)]
struct Link<'a> {
    pair: &'a PairView<'a>,
    correspondence: &'a MirroredCorrespondence,
}

struct Links<'a> {
    document: &'a ProjectDoc,
    pairs: Vec<PairView<'a>>,
}

impl<'a> Links<'a> {
    fn new(document: &'a ProjectDoc) -> Self {
        let pairs = document
            .hardware
            .iter()
            .flat_map(|hardware| &hardware.pairs)
            .filter_map(|pair| {
                pair.halves.as_ref().map(|halves| PairView {
                    mapping: pair.mapping,
                    halves,
                })
            })
            .collect();
        Self { document, pairs }
    }

    /// The correspondence whose mirror side is `id`, with its state overlay applied.
    fn target(&self, kind: MirroredItemKind, id: &str) -> Result<Target<'_>, String> {
        let mut found = None;
        for pair in &self.pairs {
            for correspondence in &pair.halves.correspondences {
                if correspondence.kind == kind && correspondence.mirror_id == id {
                    if found.is_some() {
                        return Err(format!("{id:?} is the mirror of more than one pair"));
                    }
                    found = Some(Link {
                        pair,
                        correspondence,
                    });
                }
            }
        }
        let Some(link) = found else {
            return Ok(Target::NotMirror);
        };
        if let Some(overlay) = link.pair.halves.overlay_of(link.correspondence) {
            return Ok(match overlay.state {
                MirroredExceptionState::Local => Target::Local,
                MirroredExceptionState::Omitted => Target::Omitted,
            });
        }
        // Children of a Local or Omitted matrix follow their owner's exception.
        if kind == MirroredItemKind::Part {
            for owner in &link.pair.halves.correspondences {
                if owner.kind != MirroredItemKind::Matrix
                    || !id.starts_with(&format!("matrix/{}/", owner.mirror_id))
                {
                    continue;
                }
                if let Some(overlay) = link.pair.halves.overlay_of(owner) {
                    return Ok(match overlay.state {
                        MirroredExceptionState::Local => Target::Local,
                        MirroredExceptionState::Omitted => Target::Omitted,
                    });
                }
            }
        }
        Ok(Target::Linked(link))
    }

    fn omitted(id: &str) -> String {
        format!(
            "{id:?} is omitted on its mirror half and has no geometry; \
             rejoin it before editing it"
        )
    }

    /// The part a linked mirror part id names on the source half, or `None` when the id is not
    /// a linked mirror (it then executes as written).
    fn linked_part(&self, id: &str) -> Result<Option<Link<'_>>, String> {
        match self.target(MirroredItemKind::Part, id)? {
            Target::NotMirror | Target::Local => Ok(None),
            Target::Omitted => Err(Self::omitted(id)),
            Target::Linked(link) => {
                let owned = self.document.embedded_circuits.iter().any(|circuit| {
                    circuit.part_ids.iter().any(|part| part == id)
                        && self.pairs.iter().any(|pair| {
                            pair.halves.correspondences.iter().any(|correspondence| {
                                correspondence.kind == MirroredItemKind::EmbeddedCircuit
                                    && correspondence.mirror_id == circuit.id
                            })
                        })
                });
                if owned {
                    return Err(format!(
                        "{id:?} belongs to a linked embedded circuit; ticket 112 translates \
                         circuit edits"
                    ));
                }
                Ok(Some(link))
            }
        }
    }

    // ---- matrices --------------------------------------------------------------------------

    /// The matrix a request names, as the request must execute it, with the pair map when the
    /// name was a linked mirror's. A matrix reached by a native pair and a legacy layout link
    /// at once is refused; so is an Omitted mirror matrix.
    fn matrix_frame(&self, id: &str) -> Result<(String, Option<Pose2>), String> {
        let native = self.pairs.iter().any(|pair| {
            pair.halves.correspondences.iter().any(|correspondence| {
                correspondence.kind == MirroredItemKind::Matrix
                    && (correspondence.source_id == id || correspondence.mirror_id == id)
            })
        });
        if native && crate::matrix::layout::partner(self.document, id).is_some() {
            return Err(format!(
                "The matrix {id:?} is linked by a native mirrored pair and by a legacy layout \
                 mirror link; unlink one of them before editing it"
            ));
        }
        match self.target(MirroredItemKind::Matrix, id)? {
            Target::NotMirror | Target::Local => Ok((id.to_owned(), None)),
            Target::Omitted => Err(Self::omitted(id)),
            Target::Linked(link) => Ok((
                link.correspondence.source_id.clone(),
                Some(link.pair.mapping),
            )),
        }
    }

    fn matrix_id(&self, id: &str) -> Result<(String, bool), String> {
        let (id, mapping) = self.matrix_frame(id)?;
        Ok((id, mapping.is_some()))
    }

    /// A full matrix payload for a linked mirror, as the source matrix that derives it.
    ///
    /// Apart from its origin, the matrix reflection is its own inverse (the heading rule, the
    /// local frame's X flip toggled, cell turns and companion handedness negated), so the
    /// forward `reflected` is reused with the payload as input and the current source matrix as
    /// the receiver of identity, name, definition, board and members; the origin takes the
    /// exact inverse map. Companion placement is
    /// then re-derived under each companion's own placement mode. A value the forward pass never
    /// carries to the mirror (name, definition, board) cannot travel back through a source
    /// edit, so a payload that changed one is refused instead of silently dropped.
    fn set_matrix(&self, payload: &Matrix) -> Result<Option<Matrix>, String> {
        let (source_id, Some(mapping)) = self.matrix_frame(&payload.id)? else {
            return Ok(None);
        };
        let link = match self.target(MirroredItemKind::Matrix, &payload.id)? {
            Target::Linked(link) => link,
            Target::NotMirror | Target::Local | Target::Omitted => return Ok(None),
        };
        let find = |id: &str| {
            self.document
                .matrices
                .iter()
                .find(|matrix| matrix.id == id)
                .ok_or_else(|| format!("The matrix {id:?} is unavailable"))
        };
        let current_mirror = find(&payload.id)?;
        let current_source = find(&source_id)?;
        let mirror_only = |field: &str| {
            format!(
                "The {field} of the linked mirror matrix {:?} is not reflected from its source; \
                 change it on a local matrix instead",
                payload.id
            )
        };
        if payload.name != current_mirror.name {
            return Err(mirror_only("name"));
        }
        if payload.definition_id != current_mirror.definition_id {
            return Err(mirror_only("definition"));
        }
        if payload.board_id.is_some() && payload.board_id != current_mirror.board_id {
            return Err(mirror_only("board"));
        }
        if payload
            .cells
            .iter()
            .any(|cell| cell.assemblies_local.is_some())
        {
            return Err(format!(
                "The linked mirror matrix {:?} cannot carry local companions; a native pair \
                 reflects every companion from its source",
                payload.id
            ));
        }

        let mut source = crate::matrix::layout::reflected(payload, current_source, mapping, false)?;
        // `reflected` maps the origin with the forward map, which is not its own inverse when the
        // map has a translation along the mirror line; the heading rule is an involution.
        source.origin = pair::inverse(mapping, payload.origin);
        // An absent flag and `none` are one frame; keep the stored spelling so an unchanged
        // matrix translates to exactly what the source holds.
        if source.mirror == Some(Mirror::None) && current_source.mirror.is_none() {
            source.mirror = None;
        }
        // Companion placement depends on each companion's stored mode, so it is recomputed from
        // the payload rather than taken from the literal reflection.
        for cell in &mut source.cells {
            if cell.assemblies_local == Some(true) {
                continue;
            }
            let Some(payload_cell) = payload
                .cells
                .iter()
                .find(|candidate| (candidate.row, candidate.column) == (cell.row, cell.column))
            else {
                continue;
            };
            let heading = source.rotation.unwrap_or(0.0)
                + column_rotation(&source.column_splays, source.mirror, cell.column)
                + cell.rotation.unwrap_or(0.0);
            let mirror_key_id = format!("matrix/{}/r{}c{}", payload.id, cell.row, cell.column);
            let mut assemblies = Vec::with_capacity(payload_cell.assemblies.len());
            for assembly in &payload_cell.assemblies {
                let part_id = format!("{mirror_key_id}/{}", assembly.id);
                let correspondence = link.pair.halves.correspondences.iter().find(|candidate| {
                    candidate.kind == MirroredItemKind::Part && candidate.mirror_id == part_id
                });
                let mode = correspondence
                    .and_then(|candidate| candidate.companion_mode)
                    .unwrap_or(link.pair.halves.companion_mode);
                let tweak = correspondence.and_then(|candidate| candidate.companion_tweak);
                let stabiliser =
                    super::SYMMETRIC_DEFINITIONS.contains(&assembly.definition_id.as_str());
                let mut placed = assembly.clone();
                (placed.offset, placed.rotation) = if stabiliser {
                    // The forward rule keeps the stabiliser's world heading:
                    // `mirror = 2 * key + source - map`, so `source = mirror + map - 2 * key`.
                    (
                        Vec2 {
                            x: -assembly.offset.x,
                            y: assembly.offset.y,
                        },
                        Some(assembly.rotation.unwrap_or(0.0) + mapping.rotation - 2.0 * heading),
                    )
                } else {
                    match mode {
                        MirroredCompanionMode::LiteralMirror => (
                            Vec2 {
                                x: -assembly.offset.x,
                                y: assembly.offset.y,
                            },
                            assembly.rotation.map(|rotation| -rotation),
                        ),
                        MirroredCompanionMode::LiteralMirrorWithTweak => {
                            let tweak = tweak.ok_or_else(|| {
                                format!("The companion {part_id:?} has no stored tweak")
                            })?;
                            (
                                Vec2 {
                                    x: -(assembly.offset.x - tweak.x),
                                    y: assembly.offset.y - tweak.y,
                                },
                                Some(-(assembly.rotation.unwrap_or(0.0) - tweak.rotation)),
                            )
                        }
                        MirroredCompanionMode::DuplicatePosition => {
                            (assembly.offset, assembly.rotation)
                        }
                    }
                };
                assemblies.push(placed);
            }
            cell.assemblies = assemblies;
        }
        Ok(Some(source))
    }

    /// Per-matrix cell selections. Entries for both halves of one link are one set of
    /// coordinates on the source matrix, so a grouped selection cannot run the action twice.
    fn selected_cells(&self, cells: &CellSelection) -> Result<(CellSelection, bool), String> {
        let mut groups: BTreeMap<String, Vec<&Vec<(u32, u32)>>> = BTreeMap::new();
        let mut translated = false;
        for (id, selection) in cells {
            let (canonical, was_mirror) = self.matrix_id(id)?;
            translated |= was_mirror;
            groups.entry(canonical).or_default().push(selection);
        }
        let merged = groups
            .into_iter()
            .map(|(id, selections)| {
                let cells = match selections.as_slice() {
                    [only] => (*only).clone(),
                    many => many
                        .iter()
                        .flat_map(|selection| selection.iter().copied())
                        .collect::<BTreeSet<_>>()
                        .into_iter()
                        .collect(),
                };
                (id, cells)
            })
            .collect();
        Ok((merged, translated))
    }

    // ---- MoveParts -------------------------------------------------------------------------

    fn move_parts(&self, positions: &[Position]) -> Result<(Vec<Position>, bool), String> {
        let requested: BTreeMap<&str, Vec2> = positions
            .iter()
            .map(|position| (position.id.as_str(), position.at))
            .collect();
        // (canonical position, the id as written when it was rewritten)
        let mut out: Vec<(Position, Option<&str>)> = Vec::with_capacity(positions.len());
        let mut translated = false;
        for position in positions {
            match self.linked_part(&position.id)? {
                None => out.push((position.clone(), None)),
                Some(link) => {
                    translated = true;
                    let at = self.source_position(link, &position.id, position.at, &requested)?;
                    out.push((
                        Position {
                            id: link.correspondence.source_id.clone(),
                            at,
                        },
                        Some(position.id.as_str()),
                    ));
                }
            }
        }
        // One canonical target is one action. Only collisions that involve a rewritten entry
        // are folded; an untouched duplicate keeps the existing operation's own behaviour.
        let mut result: Vec<(Position, Option<&str>)> = Vec::with_capacity(out.len());
        for (position, written) in out {
            let earlier = result.iter().find(|(kept, kept_written)| {
                kept.id == position.id && (written.is_some() || kept_written.is_some())
            });
            match earlier {
                None => result.push((position, written)),
                Some((kept, kept_written)) => {
                    let agree = (kept.at.x - position.at.x).abs() < AGREEMENT_TOLERANCE_MM
                        && (kept.at.y - position.at.y).abs() < AGREEMENT_TOLERANCE_MM;
                    if !agree {
                        return Err(format!(
                            "{:?} and {:?} are both halves of one link but request different \
                             positions; move one linked half at a time",
                            kept_written.unwrap_or(&kept.id),
                            written.unwrap_or(&position.id)
                        ));
                    }
                }
            }
        }
        Ok((
            result.into_iter().map(|(position, _)| position).collect(),
            translated,
        ))
    }

    /// The source position that makes the linked mirror `id` land at `at`.
    ///
    /// A switch and an independent part are plain pair-map images of their sources. A companion
    /// follows its stored placement mode (ADR-0010, companion modes):
    /// literal mirror (also every stabiliser): the pair-map image;
    /// literal mirror with a tweak: the image plus the tweak turned by the mirror key's heading;
    /// duplicate position: the source key's position plus the mirror's offset in its own key's
    /// frame, turned by the source key's heading.
    fn source_position(
        &self,
        link: Link<'_>,
        id: &str,
        at: Vec2,
        requested: &BTreeMap<&str, Vec2>,
    ) -> Result<Vec2, String> {
        let mapping = link.pair.mapping;
        let Some((mirror_key_id, _assembly)) = self.companion_of(id) else {
            return Ok(pair::inverse(mapping, at));
        };
        let part = self.part(id)?;
        if super::SYMMETRIC_DEFINITIONS.contains(&part.definition_id.as_str()) {
            return Ok(pair::inverse(mapping, at));
        }
        let mode = link
            .correspondence
            .companion_mode
            .unwrap_or(link.pair.halves.companion_mode);
        let mirror_key = self.part(&mirror_key_id)?;
        let mirror_rotation = mirror_key.pose.rotation;
        match mode {
            MirroredCompanionMode::LiteralMirror => Ok(pair::inverse(mapping, at)),
            MirroredCompanionMode::LiteralMirrorWithTweak => {
                let tweak = link
                    .correspondence
                    .companion_tweak
                    .unwrap_or(MirroredCompanionTweak {
                        x: 0.0,
                        y: 0.0,
                        rotation: 0.0,
                    });
                let shift = rotate(
                    Vec2 {
                        x: tweak.x,
                        y: tweak.y,
                    },
                    mirror_rotation,
                );
                Ok(pair::inverse(
                    mapping,
                    Vec2 {
                        x: at.x - shift.x,
                        y: at.y - shift.y,
                    },
                ))
            }
            MirroredCompanionMode::DuplicatePosition => {
                let source_key_id = link
                    .pair
                    .halves
                    .correspondences
                    .iter()
                    .find(|correspondence| {
                        correspondence.kind == MirroredItemKind::Part
                            && correspondence.mirror_id == mirror_key_id
                    })
                    .map(|correspondence| correspondence.source_id.as_str())
                    .ok_or_else(|| format!("The switch {mirror_key_id:?} has no source"))?;
                let source_key = self.part(source_key_id)?;
                let mirror_key_at = requested
                    .get(mirror_key_id.as_str())
                    .copied()
                    .unwrap_or(mirror_key.pose.at);
                let offset = rotate(
                    Vec2 {
                        x: at.x - mirror_key_at.x,
                        y: at.y - mirror_key_at.y,
                    },
                    -mirror_rotation,
                );
                let source_key_at = pair::inverse(mapping, mirror_key_at);
                let carried = rotate(offset, source_key.pose.rotation);
                Ok(Vec2 {
                    x: source_key_at.x + carried.x,
                    y: source_key_at.y + carried.y,
                })
            }
        }
    }

    fn part(&self, id: &str) -> Result<&Part, String> {
        self.document
            .parts
            .iter()
            .find(|part| part.id == id)
            .ok_or_else(|| format!("The part {id:?} is unavailable"))
    }

    /// The key id and assembly id when `id` is a matrix companion of some matrix.
    fn companion_of(&self, id: &str) -> Option<(String, String)> {
        for matrix in &self.document.matrices {
            let Some(rest) = id.strip_prefix(&format!("matrix/{}/r", matrix.id)) else {
                continue;
            };
            let (slot, assembly) = rest.split_once('/')?;
            let (row, column) = slot.split_once('c')?;
            row.parse::<u32>().ok()?;
            column.parse::<u32>().ok()?;
            return Some((format!("matrix/{}/r{slot}", matrix.id), assembly.to_owned()));
        }
        None
    }

    // ---- AddPart ---------------------------------------------------------------------------

    /// A part added on the mirror board of a pair becomes an addition on the source board; the
    /// forward Add transition then creates the mirror under the identity the caller chose.
    fn add_part(
        &self,
        part: &Part,
        board_id: Option<&str>,
    ) -> Result<Option<(EditOperation, PartAddition)>, String> {
        let board = board_id
            .map(str::to_owned)
            .or_else(|| self.document.boards.first().map(|board| board.id.clone()));
        let Some(board) = board else {
            return Ok(None);
        };
        let Some(pair) = self
            .pairs
            .iter()
            .find(|pair| pair.halves.mirror_board_id == board)
        else {
            return Ok(None);
        };
        let source_board = pair.halves.source_board_id.clone();

        let mut taken: BTreeSet<&str> = self
            .document
            .parts
            .iter()
            .map(|existing| existing.id.as_str())
            .collect();
        for pair in &self.pairs {
            for correspondence in &pair.halves.correspondences {
                taken.insert(&correspondence.source_id);
                taken.insert(&correspondence.mirror_id);
            }
        }
        let candidate = super::mirror_id(&board, &source_board, &part.id);
        let mut source_id = candidate.clone();
        let mut index = 2;
        while taken.contains(source_id.as_str()) || source_id == part.id {
            source_id = format!("{candidate}-{index}");
            index += 1;
        }
        let references: BTreeSet<&str> = self
            .document
            .parts
            .iter()
            .map(|existing| existing.reference.as_str())
            .collect();
        let base = super::mirror_reference(&board, &source_board, &part.reference);
        let mut reference = base.clone();
        let mut index = 2;
        while references.contains(reference.as_str()) {
            reference = format!("{base}-{index}");
            index += 1;
        }

        let mut source = part.clone();
        source.id = source_id.clone();
        source.reference = reference;
        // The position takes the exact inverse; the heading rule `map - r + 180` is its own
        // inverse over the full circle.
        source.pose = Pose2 {
            at: pair::inverse(pair.mapping, part.pose.at),
            rotation: (pair::inverse_angle(pair.mapping, part.pose.rotation) + 180.0)
                .rem_euclid(360.0),
        };
        Ok(Some((
            EditOperation::AddPart {
                part: source,
                board_id: Some(source_board),
            },
            PartAddition {
                source_id,
                mirror_id: part.id.clone(),
                mirror_definition_id: part.definition_id.clone(),
                mirror_reference: part.reference.clone(),
            },
        )))
    }

    // ---- RemoveParts -----------------------------------------------------------------------

    fn remove_parts(&self, ids: &[String]) -> Result<(Vec<String>, bool), String> {
        let mut out: Vec<String> = Vec::with_capacity(ids.len());
        let mut translated = false;
        for id in ids {
            match self.linked_part(id)? {
                None => out.push(id.clone()),
                Some(link) => {
                    translated = true;
                    let source = link.correspondence.source_id.clone();
                    if !out.contains(&source) {
                        out.push(source);
                    }
                }
            }
        }
        // A source named before or after its mirror is one removal.
        let mut seen = BTreeSet::new();
        let ids = if translated {
            out.into_iter()
                .filter(|id| seen.insert(id.clone()))
                .collect()
        } else {
            out
        };
        Ok((ids, translated))
    }
}

/// `vector` turned by `degrees` about the origin.
fn rotate(vector: Vec2, degrees: f64) -> Vec2 {
    let (sin, cos) = degrees.to_radians().sin_cos();
    Vec2 {
        x: vector.x * cos - vector.y * sin,
        y: vector.x * sin + vector.y * cos,
    }
}

/// The turn a matrix's column splays give a key, as the generator applies it: the splays up to
/// and including the column, negated when the local frame is flipped.
fn column_rotation(splays: &[f64], mirror: Option<Mirror>, column: u32) -> f64 {
    let angle: f64 = splays.iter().take(column as usize + 1).sum();
    if matches!(mirror, Some(Mirror::X | Mirror::Y)) {
        -angle
    } else {
        angle
    }
}
