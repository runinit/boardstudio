//! Inverse-map translation of requests that target a linked mirror (ticket 110).
//!
//! An edit made on a linked mirror is an edit of the authored source value (ADR-0010). This
//! private layer rewrites such a request into the equivalent source request, using the pair map
//! and the stored correspondences, before the existing operation executes. Prepared paired edits
//! call it before staging the canonical source and mirror changes.
//!
//! Where things live (search the names; line numbers drift):
//! - Seam: [`canonicalize`] is the one exhaustive dispatch over every `EditOperation` variant. It
//!   returns a [`Canonical`] request, or a [`Refusal`] that carries the ledger entry
//!   ([`Coverage`]) of a family another ticket owns. There is no wildcard arm: a new variant does
//!   not compile until it is given a ledger entry here.
//! - Targets: [`Links`] resolves an id to Not-a-mirror, Local, Omitted or Linked.
//! - Parts: `move_parts`, `add_part`, `remove_parts`; companion placement is `source_position`.
//! - Hosted, electrical and keycap families (ticket 112) live in the child module `hosted`.
//! - Matrices: `matrix_frame`, `selected_cells`, `set_matrix`; a matrix new to a paired board is
//!   `matrix_addition`.
//!
//! Rules this layer keeps:
//! - A target that is not a linked mirror (unpaired, a source, or a Local exception) executes
//!   as written. An Omitted target is refused: its geometry is absent and is not recreated.
//! - Positions and headings use the pair's exact inverse (`pair::inverse`, `inverse_angle`).
//!   Switches take no half turn; independent parts take the full-circle rule; companions follow
//!   their stored placement mode and stabilisers keep their world heading.
//! - Two entries that resolve to one canonical target are one action when they agree and a
//!   refusal when they do not; they are never executed twice.
//! - A part or matrix added on a paired board is planned as a source/mirror identity pair
//!   (`PartAddition`, `MatrixAddition`) for the forward Add transitions to consume.

#[cfg(test)]
#[path = "inverse/tests.rs"]
mod tests;

mod constraints;
mod hosted;
mod layout;
mod outline;

pub(in crate::mirrored_half) use constraints::PreparedConstraintEdit;

use crate::matrix::splay::rotate as rotate_about;
use crate::mechanical::pair;
use crate::model::{
    EditOperation, Matrix, MatrixAnchor, MatrixSplayChange, Mirror, MirroredCompanionMode,
    MirroredCompanionTweak, MirroredCorrespondence, MirroredExceptionState, MirroredHalves,
    MirroredItemKind, Part, PartKind, Pose2, Position, ProjectDoc, Vec2,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

/// Cell coordinates chosen per matrix id.
type CellSelection = BTreeMap<String, Vec<(u32, u32)>>;

/// Two requested values of one canonical target agree when they differ by less than this.
const AGREEMENT_TOLERANCE_MM: f64 = 1e-6;

const ORIGIN: Vec2 = Vec2 { x: 0.0, y: 0.0 };

/// What becomes of an operation under the inverse map: one entry of the ledger.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Coverage {
    /// Ticket 110 (this module) rewrites linked mirror targets to their sources.
    Translated,
    /// An existing lifecycle already carries both halves; the inverse must not run twice.
    Delegated(&'static str),
    /// The request keeps its meaning on the half it names.
    DeliberatelyUnchanged(&'static str),
}

/// Why a request was not canonicalised.
#[derive(Debug)]
pub(super) enum Refusal {
    /// This request cannot be translated, with the reason.
    Request(String),
}

impl fmt::Display for Refusal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Refusal::Request(message) => formatter.write_str(message),
        }
    }
}

impl From<String> for Refusal {
    fn from(message: String) -> Self {
        Refusal::Request(message)
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
    pub(super) mirror_layout_id: Option<String>,
}

/// A matrix new to a paired board, planned as a linked pair. This layer supplies the identities
/// and the canonical source request; the forward transition that creates the mirror matrix, its
/// members, modules and wiring, and records the correspondence, is ticket 145's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct MatrixAddition {
    pub(super) source_id: String,
    pub(super) mirror_id: String,
    pub(super) source_board_id: String,
    pub(super) mirror_board_id: String,
    /// Each generated member (key or companion) as `(source id, mirror id)`.
    pub(super) members: Vec<(String, String)>,
}

/// The source matrix a mirror-side addition becomes, with its planned pair.
struct NewMatrix {
    source_matrix: Matrix,
    plan: MatrixAddition,
}

/// The source-equivalent request.
#[derive(Debug)]
pub(super) struct Canonical {
    /// The request to execute, apply and resolve; downstream resolution must see this value too.
    pub(super) operation: EditOperation,
    /// How the operation was handled; `Translated` unless the ledger says it runs as written.
    pub(super) coverage: Coverage,
    /// Mirror-side part additions, one per translated `AddPart`.
    pub(super) additions: Vec<PartAddition>,
    /// Matrices new to a paired board, one per `SetMatrix` that creates one.
    pub(super) matrix_additions: Vec<MatrixAddition>,
    /// Modules and circuits new to a mirror board (ticket 112).
    pub(super) hosted: hosted::Plan,
    /// Constraint edits compiled to their live endpoint views; ticket 115 threads this through
    /// apply and both resolutions.
    pub(super) prepared_constraints: Vec<constraints::PreparedConstraintEdit>,
}

const BINDINGS_UNCHANGED: &str =
    "bindings are never reflected; the edit keeps its targeted-half semantics (ADR-0010)";
const CASE_BODIES_UNCHANGED: &str = "case bodies are independently authored per board";
const MECHANICAL_DELEGATED: &str = "the typed mechanical lifecycle already edits both halves of a pair; inverting it again would \
     apply it twice";
const PAIR_CREATION_DELEGATED: &str =
    "CreateMirroredPair creates both linked layouts in one Core operation";
const HALF_CREATION_DELEGATED: &str =
    "CreateMirroredHalf derives both halves through the Core mirror lifecycle";
const REPLACE_DOCUMENT_DELEGATED: &str =
    "ticket 116 owns source-authoritative Open and ReplaceDocument repair";

/// Rewrites `operation` so that every linked mirror target names its authored source value.
pub(super) fn canonicalize(
    document: &ProjectDoc,
    operation: &EditOperation,
) -> Result<Canonical, Refusal> {
    use EditOperation as Op;
    let links = Links::new(document);
    let mut canonical = Canonical {
        operation: operation.clone(),
        coverage: Coverage::Translated,
        additions: Vec::new(),
        matrix_additions: Vec::new(),
        hosted: hosted::Plan::default(),
        prepared_constraints: Vec::new(),
    };
    match operation {
        Op::MoveParts { positions } => {
            canonical.operation = Op::MoveParts {
                positions: links.move_parts(positions)?,
            };
            let Op::MoveParts { positions } = &canonical.operation else {
                unreachable!("the MoveParts arm builds a MoveParts operation");
            };
            canonical.prepared_constraints =
                constraints::prepare_move_parts(document, positions).map_err(Refusal::Request)?;
        }
        Op::AddPart {
            part,
            board_id,
            definition,
            layout_id,
        } => {
            if let Some((source, addition)) = links.add_part(
                part,
                board_id.as_deref(),
                definition.as_ref(),
                layout_id.as_deref(),
            )? {
                canonical.operation = source;
                canonical.additions.push(addition);
            }
        }
        Op::AddMirrorOnlyPart { .. } => {
            canonical.coverage = Coverage::DeliberatelyUnchanged(
                "explicit mirror-only placement is target-owned and prepared atomically",
            );
        }
        Op::RemoveParts { ids } => {
            canonical.operation = Op::RemoveParts {
                ids: links.remove_parts(ids)?,
            };
        }
        Op::RemoveMatrix { id } => {
            canonical.operation = Op::RemoveMatrix {
                id: links.matrix_id(id)?,
            };
        }
        Op::RemoveMatrixCells { matrix_id, cells } => {
            canonical.operation = Op::RemoveMatrixCells {
                matrix_id: links.matrix_id(matrix_id)?,
                cells: cells.clone(),
            };
        }
        Op::RemoveMatrixRow { matrix_id, row } => {
            canonical.operation = Op::RemoveMatrixRow {
                matrix_id: links.matrix_id(matrix_id)?,
                row: *row,
            };
        }
        Op::RemoveMatrixColumn { matrix_id, column } => {
            canonical.operation = Op::RemoveMatrixColumn {
                matrix_id: links.matrix_id(matrix_id)?,
                column: *column,
            };
        }
        Op::SetMatrixCellsEnabled { cells, enabled } => {
            canonical.operation = Op::SetMatrixCellsEnabled {
                cells: links.selected_cells(cells)?,
                enabled: *enabled,
            };
        }
        Op::RemoveSelectedMatrixCells { cells } => {
            canonical.operation = Op::RemoveSelectedMatrixCells {
                cells: links.selected_cells(cells)?,
            };
        }
        Op::SetMatrixSplay {
            matrix_id,
            column,
            change,
        } => {
            let (id, mapping) = links.matrix_frame(matrix_id)?;
            // An origin is a world point, so it takes the pair's inverse. An angle is a value
            // of the matrix's own frame, which the mirror shares with the source.
            let change = match (change, mapping) {
                (MatrixSplayChange::Origin { world: Some(world) }, Some(mapping)) => {
                    MatrixSplayChange::Origin {
                        world: Some(pair::inverse(mapping, *world)),
                    }
                }
                (MatrixSplayChange::OriginSwing { world: Some(world) }, Some(mapping)) => {
                    MatrixSplayChange::OriginSwing {
                        world: Some(pair::inverse(mapping, *world)),
                    }
                }
                (change, _) => change.clone(),
            };
            canonical.operation = Op::SetMatrixSplay {
                matrix_id: id,
                column: *column,
                change,
            };
        }
        Op::SetMatrixPlacement { matrix_id, change } => {
            let (id, mapping) = links.matrix_frame(matrix_id)?;
            let mut change = change.clone();
            if let Some(mapping) = mapping {
                use crate::model::MatrixPlacementChange;
                if let MatrixPlacementChange::MatrixTranslation { delta }
                | MatrixPlacementChange::ColumnTranslation { delta, .. } = &mut change
                {
                    let zero = pair::inverse(mapping, Vec2::default());
                    let point = pair::inverse(mapping, *delta);
                    *delta = Vec2 {
                        x: point.x - zero.x,
                        y: point.y - zero.y,
                    };
                }
            }
            canonical.operation = Op::SetMatrixPlacement {
                matrix_id: id,
                change,
            };
        }
        // Ticket 20: anchor intents map their matrix identities to the source.
        // Row/column stay the physical key and Follow keeps its subset with
        // source-world transport in the kernel; anchored relative Origin X
        // and Rotation negate while Y stays, via the stored relative values
        // in `reflected`/`sync`, not by negating cell turns twice.
        Op::AddAnchoredMatrix { id, anchor } => {
            let (source_id, _) = links.matrix_frame(&anchor.matrix_id)?;
            canonical.operation = Op::AddAnchoredMatrix {
                id: id.clone(),
                anchor: MatrixAnchor {
                    matrix_id: source_id.clone(),
                    ..anchor.clone()
                },
            };
            // Ticket 29: an anchored add on a paired board is reflected to
            // the mirror half (spec §4), like a SetMatrix addition. Without
            // a relating pair the add stays source-only, as before.
            if let Some(plan) = links.anchored_addition(id, &source_id)? {
                canonical.matrix_additions.push(plan);
            }
        }
        Op::SetMatrixAnchor { matrix_id, anchor } => {
            let (id, _) = links.matrix_frame(matrix_id)?;
            let (source_id, _) = links.matrix_frame(&anchor.matrix_id)?;
            canonical.operation = Op::SetMatrixAnchor {
                matrix_id: id,
                anchor: MatrixAnchor {
                    matrix_id: source_id,
                    ..anchor.clone()
                },
            };
        }
        Op::DetachMatrixAnchor { matrix_id } => {
            // Detach keeps the live world frame, so translating the identity
            // is enough: the pair sync mirrors the kept pose in the same edit.
            canonical.operation = Op::DetachMatrixAnchor {
                matrix_id: links.matrix_id(matrix_id)?,
            };
        }
        Op::DetachMatrixAnchorConfirmed {
            matrix_id,
            local_matrix_id,
        } => {
            // The detach executes on the source, but the Local exception
            // belongs on the mirror half: a currently linked mirror keeps
            // its authored identity so the kernel records the exception
            // against it (ticket 23's confirmation names the mirror).
            // Anything else maps as before for the stale downstream checks.
            let keep_local = links.pairs.iter().any(|pair| {
                pair.halves.correspondences.iter().any(|record| {
                    record.kind == MirroredItemKind::Matrix && record.mirror_id == *local_matrix_id
                }) && !pair.halves.exceptions.iter().any(|exception| {
                    exception.kind == MirroredItemKind::Matrix
                        && exception.mirror_id == *local_matrix_id
                })
            });
            canonical.operation = Op::DetachMatrixAnchorConfirmed {
                matrix_id: links.matrix_id(matrix_id)?,
                local_matrix_id: if keep_local {
                    local_matrix_id.clone()
                } else {
                    links.matrix_id(local_matrix_id)?
                },
            };
        }
        Op::SetMatrix {
            matrix,
            definitions,
        } => {
            if let Some(addition) = links.matrix_addition(matrix)? {
                canonical.operation = Op::SetMatrix {
                    matrix: addition.source_matrix,
                    definitions: definitions.clone(),
                };
                canonical.matrix_additions.push(addition.plan);
            } else if let Some(source) = links.set_matrix(matrix)? {
                canonical.operation = Op::SetMatrix {
                    matrix: source,
                    definitions: definitions.clone(),
                };
            }
        }
        // Bindings are excluded by ADR-0010: the edit keeps its targeted-half semantics.
        Op::EditKeymap { .. } | Op::SetKeyBinding { .. } => {
            canonical.coverage = Coverage::DeliberatelyUnchanged(BINDINGS_UNCHANGED);
        }
        // The typed mechanical lifecycle is already bidirectional.
        Op::ApplyMechanicalEdit { .. } => {
            canonical.coverage = Coverage::Delegated(MECHANICAL_DELEGATED);
        }
        // Case bodies are authored independently on each board; preserve the targeted board.
        Op::SetCase { .. } => {
            canonical.coverage = Coverage::DeliberatelyUnchanged(CASE_BODIES_UNCHANGED);
        }
        // These operations create or replace both sides as one Core lifecycle. Do not inverse-map
        // their already paired effects; Open/Replace repair is owned by ticket 116.
        Op::CreateMirroredPair { .. } => {
            canonical.coverage = Coverage::Delegated(PAIR_CREATION_DELEGATED);
        }
        Op::CreateMirroredHalf { .. } => {
            canonical.coverage = Coverage::Delegated(HALF_CREATION_DELEGATED);
        }
        Op::ReplaceDocument { .. } => {
            canonical.coverage = Coverage::Delegated(REPLACE_DOCUMENT_DELEGATED);
        }
        // Ticket 33: the key-size planner pre-computes both halves of a linked
        // pair, so the typed resize travels exactly as written.
        Op::SetKeycapSize { .. } => {
            canonical.coverage = Coverage::Delegated(REPLACE_DOCUMENT_DELEGATED);
        }
        // Ticket 112: hosted, electrical, keycap and shared-definition families.
        Op::SetModuleDefinition { .. }
        | Op::SetMountedModule { .. }
        | Op::RemoveMountedModule { .. }
        | Op::EmbedModuleCircuit { .. }
        | Op::RemoveEmbeddedCircuit { .. }
        | Op::SetInputScanMode { .. }
        | Op::SetWiringMode { .. }
        | Op::SetKeycapBoard { .. }
        | Op::SetMatrixKeycaps { .. }
        | Op::SetKeycapKey { .. } => {
            hosted::translate(&links, operation, &mut canonical)?;
        }
        // Ticket 113: outline and reviewed geometry. Ticket 37 deleted the raw
        // outline family; ApplyOutlineEdit is the live translated outline edit.
        Op::ApplyOutlineEdit { .. } => outline::translate(&links, operation, &mut canonical)?,
        // Ticket 114: native SetLayout compatibility and raw mechanical configuration.
        Op::SetSplitGap {
            layout_id,
            split_gap,
        } => {
            // A same-board Split gap edit runs on the authored half; a native
            // mirror layout resolves to its source layout first.
            let source = match links.target(MirroredItemKind::Layout, layout_id)? {
                Target::Linked(link) => link.correspondence.source_id.clone(),
                Target::NotMirror | Target::Local | Target::Omitted => layout_id.clone(),
            };
            canonical.operation = Op::SetSplitGap {
                layout_id: source,
                split_gap: *split_gap,
            };
        }
        Op::SetMirroredPart { part_id, mirrored } => {
            // The toggle names either copy; the canonical edit runs on the
            // source copy, and unlinked parts pass through unchanged.
            let source = match links.target(MirroredItemKind::Part, part_id)? {
                Target::Linked(link) => link.correspondence.source_id.clone(),
                Target::NotMirror | Target::Local | Target::Omitted => part_id.clone(),
            };
            canonical.operation = Op::SetMirroredPart {
                part_id: source,
                mirrored: *mirrored,
            };
        }
        Op::SetLayout { layout } => match layout::translate(&links, layout)? {
            layout::Outcome::Translated(source) => {
                canonical.operation = Op::SetLayout { layout: source };
            }
            layout::Outcome::PassThrough => {}
            layout::Outcome::DeliberatelyUnchanged(reason) => {
                canonical.coverage = Coverage::DeliberatelyUnchanged(reason);
            }
        },
        Op::SetHalfException { .. }
        | Op::SetCompanionPlacement { .. }
        | Op::UnlinkHalves { .. } => {
            canonical.coverage = Coverage::Delegated(
                "the prepared lifecycle owns exceptions, companion placement and Unlink",
            );
        }
        Op::SetConstraint { .. }
        | Op::RemoveConstraint { .. }
        | Op::ConvertOffsetConstraint { .. } => {
            canonical.operation = constraints::normalize_command(document, &links, operation)
                .map_err(Refusal::Request)?;
            if let Some(prepared) = constraints::prepare(document, &canonical.operation)? {
                canonical.prepared_constraints.push(prepared);
            }
        }
    }
    Ok(canonical)
}

/// One pair with a `halves` record.
struct PairView<'a> {
    pair_id: &'a str,
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
                    pair_id: &pair.id,
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
                // A circuit member follows its circuit's namespace and member map (ticket 112).
                Ok(hosted::member_is_linked(self, id, link)?.then_some(link))
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
        if native && crate::matrix::layout::has_link(self.document, id) {
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

    fn matrix_id(&self, id: &str) -> Result<String, String> {
        Ok(self.matrix_frame(id)?.0)
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

        Ok(Some(invert_matrix(
            payload,
            current_source,
            mapping,
            link.pair.halves,
            self.document,
        )?))
    }

    /// A `SetMatrix` that creates a matrix on a board a pair relates is the addition of a linked
    /// pair: the matrix on the other board must be mirrored and linked too. Returns the source
    /// matrix to create and the planned identities, or `None` when the matrix already exists, is
    /// already named by a correspondence, or sits on a board no pair relates.
    fn matrix_addition(&self, payload: &Matrix) -> Result<Option<NewMatrix>, String> {
        if self
            .document
            .matrices
            .iter()
            .any(|matrix| matrix.id == payload.id)
            || self.pairs.iter().any(|pair| {
                pair.halves.correspondences.iter().any(|correspondence| {
                    correspondence.kind == MirroredItemKind::Matrix
                        && (correspondence.source_id == payload.id
                            || correspondence.mirror_id == payload.id)
                })
            })
        {
            return Ok(None);
        }
        let board = payload
            .board_id
            .clone()
            .or_else(|| self.document.boards.first().map(|board| board.id.clone()));
        let Some(board) = board else {
            return Ok(None);
        };
        let Some(pair) = self.pairs.iter().find(|pair| {
            pair.halves.source_board_id == board || pair.halves.mirror_board_id == board
        }) else {
            return Ok(None);
        };
        let source_board = pair.halves.source_board_id.clone();
        let mirror_board = pair.halves.mirror_board_id.clone();
        let mirror_side = board == mirror_board;

        let mut taken: BTreeSet<String> = self
            .document
            .matrices
            .iter()
            .map(|matrix| matrix.id.clone())
            .collect();
        for pair in &self.pairs {
            for correspondence in &pair.halves.correspondences {
                if correspondence.kind == MirroredItemKind::Matrix {
                    taken.insert(correspondence.source_id.clone());
                    taken.insert(correspondence.mirror_id.clone());
                }
            }
        }
        taken.insert(payload.id.clone());
        let free = |candidate: String| {
            let mut id = candidate.clone();
            let mut index = 2;
            while taken.contains(&id) {
                id = format!("{candidate}-{index}");
                index += 1;
            }
            id
        };
        let (source_id, mirror_id, source_matrix) = if mirror_side {
            let source_id = free(super::mirror_id(&mirror_board, &source_board, &payload.id));
            let mut seed = payload.clone();
            seed.id = source_id.clone();
            seed.board_id = Some(source_board.clone());
            seed.part_ids.clear();
            seed.cells.clear();
            seed.mirror = None;
            if let Some(name) = &payload.name {
                seed.name = Some(
                    super::substitute_token(name, &mirror_board, &source_board)
                        .unwrap_or_else(|| name.clone()),
                );
            }
            let source_matrix =
                invert_matrix(payload, &seed, pair.mapping, pair.halves, self.document)?;
            (source_id, payload.id.clone(), source_matrix)
        } else {
            let mirror_id = free(super::mirror_id(&source_board, &mirror_board, &payload.id));
            (payload.id.clone(), mirror_id, payload.clone())
        };

        let mut members = Vec::new();
        for row in 0..source_matrix.rows {
            for column in 0..source_matrix.columns {
                let cell = source_matrix
                    .cells
                    .iter()
                    .find(|cell| (cell.row, cell.column) == (row, column));
                if cell.is_some_and(|cell| !cell.enabled || cell.deleted) {
                    continue;
                }
                let key = |matrix: &str| format!("matrix/{matrix}/r{row}c{column}");
                members.push((key(&source_id), key(&mirror_id)));
                for assembly in cell.iter().flat_map(|cell| &cell.assemblies) {
                    members.push((
                        format!("{}/{}", key(&source_id), assembly.id),
                        format!("{}/{}", key(&mirror_id), assembly.id),
                    ));
                }
            }
        }
        Ok(Some(NewMatrix {
            plan: MatrixAddition {
                source_id,
                mirror_id,
                source_board_id: source_board,
                mirror_board_id: mirror_board,
                members,
            },
            source_matrix,
        }))
    }

    /// A matrix an anchored add creates on a paired board, planned as a linked
    /// pair. The added matrix is always a bare 1x3 (its cells are generated
    /// without assemblies), anchored to `target_id`. Only a target that is
    /// itself a paired source mirrors; anything else stays source-only.
    fn anchored_addition(
        &self,
        id: &str,
        target_id: &str,
    ) -> Result<Option<MatrixAddition>, String> {
        let Some(link) = self.pairs.iter().find_map(|pair| {
            pair.halves
                .correspondences
                .iter()
                .find(|correspondence| {
                    correspondence.kind == MirroredItemKind::Matrix
                        && correspondence.source_id == target_id
                })
                .map(|_| pair)
        }) else {
            return Ok(None);
        };
        let source_board = link.halves.source_board_id.clone();
        let mirror_board = link.halves.mirror_board_id.clone();
        let mut taken: BTreeSet<String> = self
            .document
            .matrices
            .iter()
            .map(|matrix| matrix.id.clone())
            .collect();
        for pair in &self.pairs {
            for correspondence in &pair.halves.correspondences {
                if correspondence.kind == MirroredItemKind::Matrix {
                    taken.insert(correspondence.source_id.clone());
                    taken.insert(correspondence.mirror_id.clone());
                }
            }
        }
        taken.insert(id.to_owned());
        let mut mirror_id = super::mirror_id(&source_board, &mirror_board, id);
        let mut index = 2;
        while taken.contains(&mirror_id) {
            mirror_id = format!("{mirror_id}-{index}");
            index += 1;
        }
        // The anchored add always materializes a bare 1x3: `set_matrix`
        // generates key members for every slot, with companions only from
        // explicit cells (of which the add carries none).
        let key = |matrix: &str, row: u32, column: u32| format!("matrix/{matrix}/r{row}c{column}");
        let mut members = Vec::new();
        for row in 0..1 {
            for column in 0..3 {
                members.push((key(id, row, column), key(&mirror_id, row, column)));
            }
        }
        Ok(Some(MatrixAddition {
            source_id: id.to_owned(),
            mirror_id,
            source_board_id: source_board,
            mirror_board_id: mirror_board,
            members,
        }))
    }

    /// Per-matrix cell selections. Entries for both halves of one link are one set of
    /// coordinates on the source matrix, so a grouped selection cannot run the action twice.
    fn selected_cells(&self, cells: &CellSelection) -> Result<CellSelection, String> {
        let mut groups: BTreeMap<String, Vec<&Vec<(u32, u32)>>> = BTreeMap::new();
        for (id, selection) in cells {
            groups
                .entry(self.matrix_id(id)?)
                .or_default()
                .push(selection);
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
        Ok(merged)
    }

    // ---- MoveParts -------------------------------------------------------------------------

    fn move_parts(&self, positions: &[Position]) -> Result<Vec<Position>, String> {
        let requested: BTreeMap<&str, Vec2> = positions
            .iter()
            .map(|position| (position.id.as_str(), position.at))
            .collect();
        // (canonical position, the id as written when it was rewritten)
        let mut out: Vec<(Position, Option<&str>)> = Vec::with_capacity(positions.len());
        for position in positions {
            match self.linked_part(&position.id)? {
                None => out.push((position.clone(), None)),
                Some(link) => {
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
        Ok(result.into_iter().map(|(position, _)| position).collect())
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
                let shift = rotate_about(
                    Vec2 {
                        x: tweak.x,
                        y: tweak.y,
                    },
                    mirror_rotation,
                    ORIGIN,
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
                let offset = rotate_about(
                    Vec2 {
                        x: at.x - mirror_key_at.x,
                        y: at.y - mirror_key_at.y,
                    },
                    -mirror_rotation,
                    ORIGIN,
                );
                let source_key_at = pair::inverse(mapping, mirror_key_at);
                let carried = rotate_about(offset, source_key.pose.rotation, ORIGIN);
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
        definition: Option<&crate::model::PartDefinition>,
        layout_id: Option<&str>,
    ) -> Result<Option<(EditOperation, PartAddition)>, String> {
        let board = board_id
            .map(str::to_owned)
            .or_else(|| self.document.boards.first().map(|board| board.id.clone()));
        let Some(board) = board else {
            return Ok(None);
        };
        let Some(pair) = self.pairs.iter().find(|pair| {
            pair.halves.source_board_id == board || pair.halves.mirror_board_id == board
        }) else {
            return Ok(None);
        };
        if self
            .pairs
            .iter()
            .filter(|candidate| {
                candidate.halves.source_board_id == board
                    || candidate.halves.mirror_board_id == board
            })
            .count()
            != 1
        {
            return Err(format!(
                "Target board {board:?} belongs to more than one native mirrored pair"
            ));
        }
        if board == pair.halves.source_board_id {
            let mirror_id = unique_generated_part_id(
                self.document,
                &pair.halves.source_board_id,
                &pair.halves.mirror_board_id,
                &part.id,
            );
            let mirror_reference = unique_generated_reference(self.document, &part.reference);
            let selected_layout = layout_id.map(|id| unique_layout(self, id)).transpose()?;
            if selected_layout.is_some_and(|layout| layout.board_id != board) {
                return Err(format!(
                    "Selected layout does not belong to source board {board:?}"
                ));
            }
            return Ok(Some((
                EditOperation::AddPart {
                    part: part.clone(),
                    board_id: Some(board),
                    definition: definition.cloned(),
                    layout_id: layout_id.map(str::to_owned),
                },
                PartAddition {
                    source_id: part.id.clone(),
                    mirror_id,
                    mirror_definition_id: definition.map_or_else(
                        || part.definition_id.clone(),
                        |definition| definition.id.clone(),
                    ),
                    mirror_reference,
                    mirror_layout_id: None,
                },
            )));
        }
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

        let (source_layout_id, mirror_layout_id) =
            translate_layout_id(self, pair, layout_id, &board)?;
        let mut source = part.clone();
        source.id = source_id.clone();
        source.reference = reference;
        // The position takes the exact inverse; each heading rule is its own inverse over the
        // full circle. Controllers keep the reflected heading without the half turn, like the
        // forward rule, so a mirrored MCU lands as authored.
        let is_controller = definition
            .map(|definition| definition.kind == PartKind::Controller)
            .unwrap_or(false)
            || self.document.definitions.iter().any(|candidate| {
                candidate.id == part.definition_id && candidate.kind == PartKind::Controller
            });
        let reflected =
            pair::mirrored_part_heading(is_controller, pair.mapping, part.pose.rotation);
        source.pose = Pose2 {
            at: pair::inverse(pair.mapping, part.pose.at),
            rotation: reflected,
        };
        Ok(Some((
            EditOperation::AddPart {
                part: source,
                board_id: Some(source_board),
                definition: definition.cloned(),
                layout_id: source_layout_id,
            },
            PartAddition {
                source_id,
                mirror_id: part.id.clone(),
                mirror_definition_id: part.definition_id.clone(),
                mirror_reference: part.reference.clone(),
                mirror_layout_id,
            },
        )))
    }

    // ---- RemoveParts -----------------------------------------------------------------------

    fn remove_parts(&self, ids: &[String]) -> Result<Vec<String>, String> {
        let mut out: Vec<String> = Vec::with_capacity(ids.len());
        for id in ids {
            match self.linked_part(id)? {
                None => out.push(id.clone()),
                Some(link) => {
                    let source = link.correspondence.source_id.clone();
                    if !out.contains(&source) {
                        out.push(source);
                    }
                }
            }
        }
        // A source named before or after its mirror is one removal.
        let mut seen = BTreeSet::new();
        out.retain(|id| seen.insert(id.clone()));
        Ok(out)
    }
}

fn unique_generated_part_id(
    document: &ProjectDoc,
    source_board: &str,
    mirror_board: &str,
    requested_id: &str,
) -> String {
    let base = super::mirror_id(source_board, mirror_board, requested_id);
    let taken: BTreeSet<_> = document.parts.iter().map(|part| part.id.as_str()).collect();
    let mut candidate = base.clone();
    let mut suffix = 2;
    while taken.contains(candidate.as_str()) || candidate == requested_id {
        candidate = format!("{base}-{suffix}");
        suffix += 1;
    }
    candidate
}

fn unique_generated_reference(document: &ProjectDoc, requested: &str) -> String {
    let mut taken: BTreeSet<_> = document
        .parts
        .iter()
        .map(|part| part.reference.as_str())
        .collect();
    taken.insert(requested);
    let mut candidate = requested.to_owned();
    let mut suffix = 2;
    while taken.contains(candidate.as_str()) {
        candidate = format!("{requested}-{suffix}");
        suffix += 1;
    }
    candidate
}

fn unique_layout<'a>(links: &'a Links<'_>, id: &str) -> Result<&'a crate::model::Layout, String> {
    let mut layouts = links
        .document
        .layouts
        .iter()
        .filter(|layout| layout.id == id);
    let layout = layouts
        .next()
        .ok_or_else(|| format!("Layout {id:?} is unavailable"))?;
    if layouts.next().is_some() {
        return Err(format!("Layout {id:?} is ambiguous"));
    }
    Ok(layout)
}

fn translate_layout_id(
    links: &Links<'_>,
    pair: &PairView<'_>,
    requested_id: Option<&str>,
    target_board_id: &str,
) -> Result<(Option<String>, Option<String>), String> {
    let Some(requested_id) = requested_id else {
        return Ok((None, None));
    };
    let target = unique_layout(links, requested_id)?;
    if target.board_id != target_board_id {
        return Err(format!(
            "Layout {requested_id:?} does not belong to target board {target_board_id:?}"
        ));
    }
    if target.matrix_ids.is_empty()
        || target.matrix_ids.iter().any(|id| {
            !links.document.matrices.iter().any(|matrix| {
                &matrix.id == id && matrix.board_id.as_deref() == Some(target_board_id)
            })
        })
    {
        return Err(format!(
            "Layout {requested_id:?} has inconsistent matrix ownership"
        ));
    }
    let mut matches = pair
        .halves
        .correspondences
        .iter()
        .filter(|item| item.kind == MirroredItemKind::Layout && item.mirror_id == requested_id);
    let Some(correspondence) = matches.next() else {
        return Ok((None, Some(requested_id.to_owned())));
    };
    if matches.next().is_some() {
        return Err(format!(
            "Layout {requested_id:?} has ambiguous native correspondence"
        ));
    }
    match pair
        .halves
        .overlay_of(correspondence)
        .map(|exception| exception.state)
    {
        Some(MirroredExceptionState::Local) => Ok((None, Some(requested_id.to_owned()))),
        Some(MirroredExceptionState::Omitted) => Err(format!(
            "Layout {requested_id:?} is omitted on the mirror half"
        )),
        None => {
            let source = unique_layout(links, &correspondence.source_id)?;
            let matrix_links = pair.halves.correspondences.iter().filter(|item| {
                item.kind == MirroredItemKind::Matrix
                    && source.matrix_ids.contains(&item.source_id)
                    && target.matrix_ids.contains(&item.mirror_id)
            });
            if source.board_id != pair.halves.source_board_id
                || matrix_links.count() != source.matrix_ids.len()
            {
                return Err(format!(
                    "Layout {requested_id:?} has inconsistent source ownership"
                ));
            }
            Ok((Some(source.id.clone()), None))
        }
    }
}

/// The source matrix that derives `payload` (a mirror matrix) under `mapping`, with identity,
/// name, definition, board and members taken from `receiver`. Companion placement follows each
/// companion's stored mode in `halves`, or the pair default for a companion with no record yet.
fn invert_matrix(
    payload: &Matrix,
    receiver: &Matrix,
    mapping: Pose2,
    halves: &MirroredHalves,
    document: &ProjectDoc,
) -> Result<Matrix, String> {
    let mut source = crate::matrix::layout::reflected(payload, receiver, mapping, false, false)?;
    // `reflected` maps the origin with the forward map, which is not its own inverse when the
    // map has a translation along the mirror line; the heading rule is an involution.
    source.origin = if payload.anchor.is_some() {
        Vec2 {
            x: -payload.origin.x,
            y: payload.origin.y,
        }
    } else {
        pair::inverse(mapping, payload.origin)
    };
    // An absent flag and `none` are one frame; keep the stored spelling so an unchanged
    // matrix translates to exactly what the source holds.
    if source.mirror == Some(Mirror::None) && receiver.mirror.is_none() {
        source.mirror = None;
    }
    // Companion placement depends on each companion's stored mode, so it is recomputed from
    // the payload rather than taken from the literal reflection.
    let layout = crate::matrix::evaluated(&source, document)?;
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
        let heading =
            crate::matrix::evaluation::cell_pose(&layout, cell.row, cell.column)?.rotation;
        let mirror_key_id = format!("matrix/{}/r{}c{}", payload.id, cell.row, cell.column);
        let mut assemblies = Vec::with_capacity(payload_cell.assemblies.len());
        for assembly in &payload_cell.assemblies {
            let part_id = format!("{mirror_key_id}/{}", assembly.id);
            let correspondence = halves.correspondences.iter().find(|candidate| {
                candidate.kind == MirroredItemKind::Part && candidate.mirror_id == part_id
            });
            let mode = correspondence
                .and_then(|candidate| candidate.companion_mode)
                .unwrap_or(halves.companion_mode);
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
    Ok(source)
}
