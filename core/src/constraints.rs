use crate::matrix::layout::AssemblyPlacement;
use crate::matrix::splay::rotate;
#[cfg(test)]
use crate::model::MirrorAxis;
use crate::model::{
    Constraint, EditOperation, MatrixCompanionMode, OffsetAxes, Part, Pose2, Position, ProjectDoc,
    Vec2,
};
use endpoints::{EndpointFrame, Side};
use std::collections::{BTreeMap, BTreeSet};

mod endpoints;
#[cfg(test)]
mod framed_tests;
mod relation;
mod retune;

// The mirrored-half preparation seam carries only these private compiler records across the
// crate; no public document or operation API is widened.
pub(crate) use endpoints::ConstraintEndpoint;
pub(crate) use endpoints::{FramedConstraint, compile_legacy};
pub(crate) use relation::ConstraintRelation;

/// Conservative bound on concurrently active constraint dependencies.
///
/// Every dependency edge (source authority, framed source/target owner and implicit
/// assembly parent) recurses through [`pose`], so one named budget covers them all. The
/// value is chosen and pinned by below/at/above boundary tests on both native and packaged
/// WASM while leaving ordinary design chains ample room.
const MAX_DEPENDENCY_DEPTH: usize = 256;

/// Outcome for the mirrored edit preparer when a moved target has an owner-dependent view.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum PreparedTargetRetune {
    Retuned(FramedConstraint),
    Deferred(DeferredTargetRetune),
}

/// Target drag intent that must wait for live endpoint owners to resolve (usually a key).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DeferredTargetRetune {
    pub(crate) record: FramedConstraint,
    pub(crate) requested: Position,
    pub(crate) source_owner: Option<String>,
    pub(crate) target_owner: Option<String>,
}

/// Retunes an already compiled record through its authored endpoint views.
///
/// `requested` is in authoritative part coordinates. Position edits have no requested heading,
/// so the existing relation turn is preserved exactly. Owner poses are supplied by a caller
/// that has resolved the MoveParts batch; missing owner poses return an explicit deferred intent.
pub(crate) fn prepare_target_retune(
    document: &ProjectDoc,
    record: &FramedConstraint,
    source_authority: Pose2,
    target_authority: Pose2,
    requested: &Position,
    owner_poses: [Option<Pose2>; 2],
) -> Result<PreparedTargetRetune, String> {
    let FramedConstraint::Framed {
        id,
        source: source_endpoint,
        target: target_endpoint,
        relation,
    } = record;
    let source =
        endpoints::resolve_endpoint(document, id, endpoints::Side::Source, source_endpoint)?;
    let target =
        endpoints::resolve_endpoint(document, id, endpoints::Side::Target, target_endpoint)?;
    if matches!(relation, ConstraintRelation::MirrorLine { .. }) {
        return Err(mirror_drag_refusal(&target.visible_id));
    }

    let owners = [source.owner.clone(), target.owner.clone()];
    if (owners[0].is_some() && owner_poses[0].is_none())
        || (owners[1].is_some() && owner_poses[1].is_none())
    {
        return Ok(PreparedTargetRetune::Deferred(DeferredTargetRetune {
            record: record.clone(),
            requested: requested.clone(),
            source_owner: owners[0].clone(),
            target_owner: owners[1].clone(),
        }));
    }

    let source_frame = source.frame.with_key(owner_poses[0]);
    let target_frame = target.frame.with_key(owner_poses[1]);
    let requested_view = target_frame.project(Pose2 {
        at: requested.at,
        rotation: target_authority.rotation,
    })?;
    let retuned = retune::retune_target(
        relation,
        source_authority,
        source_frame,
        target_frame,
        target_authority,
        requested_view.at,
        None,
    )?;
    Ok(PreparedTargetRetune::Retuned(FramedConstraint::Framed {
        id: id.clone(),
        source: source_endpoint.clone(),
        target: target_endpoint.clone(),
        relation: retuned.relation,
    }))
}

/// The refusal for a second driving record on one authority, worded once.
fn multiple_constraints(authority: &str) -> String {
    format!("Part {authority} has multiple constraints")
}

/// The one record that drives `part_id`'s geometry, found through whichever half is asked:
/// a part and its live mirror have one authority, so one driving record. Pure: nothing is
/// solved. More than one record on an authority is a located error rather than a pick.
///
/// Stored records keep their visible endpoint IDs, while the ownership compiler resolves their
/// current views and the solver orders them by authoritative target. Multiple records on one
/// authority are a located error rather than a pick.
pub(crate) fn driving_constraint<'a>(
    doc: &'a ProjectDoc,
    part_id: &str,
) -> Result<Option<&'a Constraint>, String> {
    let authority = endpoints::authoritative_part(doc, part_id).to_owned();
    let mut driving = None;
    for constraint in &doc.constraints {
        let owns_authority = match constraint {
            Constraint::Framed {
                id, source, target, ..
            } => {
                // Stored framed endpoints are authored bindings. Validate them against the live
                // correspondence instead of silently inferring a replacement from visible IDs.
                endpoints::resolve_endpoint(doc, id, Side::Source, source)?;
                let resolved_target = endpoints::resolve_endpoint(doc, id, Side::Target, target)?;
                resolved_target.authoritative_id == authority
            }
            _ => endpoints::authoritative_part(doc, constraint.target()) == authority,
        };
        if owns_authority && driving.replace(constraint).is_some() {
            return Err(multiple_constraints(&authority));
        }
    }
    Ok(driving)
}

pub fn resolve(doc: &mut ProjectDoc) -> Result<Vec<String>, String> {
    resolve_with_positions(doc, &[])
}

pub(crate) fn resolve_edit(
    doc: &mut ProjectDoc,
    operation: &EditOperation,
) -> Result<Vec<String>, String> {
    let positions = match operation {
        EditOperation::MoveParts { positions } => positions.as_slice(),
        _ => &[],
    };
    resolve_with_positions(doc, positions)
}

fn resolve_with_positions(
    doc: &mut ProjectDoc,
    positions: &[Position],
) -> Result<Vec<String>, String> {
    resolve_prepared(doc, positions, &[])
}

/// A record the solver evaluates for one authoritative target part.
enum Driver<'a> {
    /// A stored or prepared relation bound to the live document.
    Framed(Framed<'a>),
}

struct Framed<'a> {
    id: &'a str,
    source_authority: String,
    source_frame: EndpointFrame,
    target_frame: EndpointFrame,
    source_owner: Option<String>,
    target_owner: Option<String>,
    relation: &'a ConstraintRelation,
}

impl Driver<'_> {
    fn source_authority(&self) -> &str {
        match self {
            Self::Framed(framed) => &framed.source_authority,
        }
    }

    /// The target authority's pose given the resolved source authority's pose. `current` is the
    /// target's stored pose, which chooses between headings a full turn apart.
    fn evaluate(
        &self,
        source: Pose2,
        owners: [Option<Pose2>; 2],
        current: Pose2,
    ) -> Result<Pose2, String> {
        match self {
            Self::Framed(framed) => endpoints::solve_target(
                source,
                framed.source_frame.with_key(owners[0]),
                framed.target_frame.with_key(owners[1]),
                framed.relation,
                current,
            )
            .map_err(|message| format!("Constraint {}: {message}", framed.id)),
        }
    }
}

fn claim_id<'a>(ids: &mut BTreeSet<&'a str>, id: &'a str) -> Result<(), String> {
    if id.trim().is_empty() || !ids.insert(id) {
        return Err(format!("Duplicate or empty constraint ID {id}"));
    }
    Ok(())
}

/// Gives `authority` its one driving record; a second one is refused.
fn claim_target<'a>(
    by_target: &mut BTreeMap<String, Driver<'a>>,
    authority: &str,
    driver: Driver<'a>,
) -> Result<(), String> {
    if by_target.insert(authority.to_owned(), driver).is_some() {
        return Err(multiple_constraints(authority));
    }
    Ok(())
}

/// Resolves stored and prepared relations in one source-first graph. Prepared records replace
/// stored records with the same ID for this solve only.
pub(crate) fn resolve_prepared(
    doc: &mut ProjectDoc,
    positions: &[Position],
    framed: &[FramedConstraint],
) -> Result<Vec<String>, String> {
    // Ticket 12: the solver works board-space; half-frame parts evaluate
    // through their half offset, and solved poses convert back on write.
    let parts: BTreeMap<_, _> = doc
        .parts
        .iter()
        .map(|part| {
            (
                part.id.clone(),
                crate::matrix::layout::evaluated_part_pose(doc, part),
            )
        })
        .collect();
    let mut by_target: BTreeMap<String, Driver> = BTreeMap::new();
    let mut ids = BTreeSet::new();
    let replaced_ids: BTreeSet<_> = framed
        .iter()
        .map(|record| match record {
            FramedConstraint::Framed { id, .. } => id.as_str(),
        })
        .collect();
    let stored: Vec<_> = doc
        .constraints
        .iter()
        .filter(|constraint| !replaced_ids.contains(constraint.id()))
        .map(|constraint| endpoints::compile_legacy(doc, constraint))
        .collect::<Result<_, _>>()?;
    for record in stored.iter().chain(framed) {
        let FramedConstraint::Framed {
            id,
            source,
            target,
            relation,
        } = record;
        claim_id(&mut ids, id)?;
        let source = endpoints::resolve_endpoint(doc, id, Side::Source, source)?;
        let target = endpoints::resolve_endpoint(doc, id, Side::Target, target)?;
        relation
            .validate()
            .map_err(|_| format!("Constraint {id} has a non-finite value"))?;
        // Legacy and framed records share the same ownership compiler and authority graph.
        let driver = Driver::Framed(Framed {
            id,
            source_authority: source.authoritative_id,
            source_frame: source.frame,
            target_frame: target.frame,
            source_owner: source.owner,
            target_owner: target.owner,
            relation,
        });
        claim_target(&mut by_target, &target.authoritative_id, driver)?;
    }

    // Matrix companion classification reads stored constraints, so give it the same
    // replacement view used to build the explicit graph.
    let persisted = std::mem::take(&mut doc.constraints);
    doc.constraints = persisted
        .iter()
        .filter(|constraint| !replaced_ids.contains(constraint.id()))
        .cloned()
        .collect();
    let assemblies = crate::matrix::layout::assembly_placements(doc, positions);
    doc.constraints = persisted;
    // Assemblies are implicit local placements; explicit constraints take priority.
    // Resolve the combined graph source-first instead of relying on repeated passes.
    let mut resolved = BTreeMap::new();
    let mut visiting = BTreeSet::new();
    for target in by_target.keys().chain(assemblies.keys()) {
        pose(
            target,
            &parts,
            &by_target,
            &assemblies,
            &mut resolved,
            &mut visiting,
        )?;
    }
    // Unrelated free parts keep their existing poses without acquiring placement
    // validation semantics from the constraint solver.
    for (id, current) in &parts {
        resolved.entry(id.clone()).or_insert(*current);
    }
    let mut changed = vec![];
    let mut key_moves = BTreeMap::new();
    for index in 0..doc.parts.len() {
        let id = doc.parts[index].id.clone();
        if let Some(next) = resolved.get(&id) {
            let stored = crate::matrix::layout::stored_part_pose(doc, &id, *next);
            if doc.parts[index].pose != stored {
                key_moves.insert(id.clone(), (doc.parts[index].pose, *next));
                doc.parts[index].pose = stored;
                changed.push(id);
            }
        }
    }
    changed.extend(crate::matrix::layout::reconcile_constraint_keys(
        doc, &key_moves, &resolved,
    )?);
    Ok(changed)
}

fn follow_pose(source: Pose2, local: Pose2) -> Pose2 {
    let (sin, cos) = source.rotation.to_radians().sin_cos();
    Pose2 {
        at: Vec2 {
            x: source.at.x + local.at.x * cos - local.at.y * sin,
            y: source.at.y + local.at.x * sin + local.at.y * cos,
        },
        rotation: source.rotation + local.rotation,
    }
}

/// The pre-Framed arithmetic retained as the exact oracle for stored legacy documents.
#[cfg(test)]
fn constraint_pose(constraint: &Constraint, source: Pose2) -> Pose2 {
    match constraint {
        Constraint::Offset {
            offset, rotation, ..
        } => Pose2 {
            at: Vec2 {
                x: source.at.x + offset.x,
                y: source.at.y + offset.y,
            },
            rotation: source.rotation + rotation,
        },
        Constraint::PartOffset {
            offset, rotation, ..
        } => follow_pose(
            source,
            Pose2 {
                at: *offset,
                rotation: *rotation,
            },
        ),
        Constraint::Mirror {
            axis, coordinate, ..
        } => Pose2 {
            at: match axis {
                MirrorAxis::Vertical => Vec2 {
                    x: 2.0 * coordinate - source.at.x,
                    y: source.at.y,
                },
                MirrorAxis::Horizontal => Vec2 {
                    x: source.at.x,
                    y: 2.0 * coordinate - source.at.y,
                },
            },
            rotation: match axis {
                MirrorAxis::Vertical => 180.0 - source.rotation,
                MirrorAxis::Horizontal => -source.rotation,
            },
        },
        Constraint::Framed { .. } => unreachable!("the oracle only accepts legacy records"),
    }
}

/// The pose of the part a constraint is measured from.
pub(crate) fn source_pose(parts: &[Part], constraint: &Constraint) -> Result<Pose2, String> {
    parts
        .iter()
        .find(|part| part.id == constraint.source())
        .map(|part| part.pose)
        .ok_or_else(|| format!("Constraint {} source is missing", constraint.id()))
}

/// What `retune_offset` did.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Retune {
    Applied,
    /// The constraint is a mirror, which reflects rather than offsets. Nothing changed.
    Mirror,
}

/// Why a part driven by a mirror constraint cannot be dragged.
pub(crate) fn mirror_drag_refusal(part_id: &str) -> String {
    format!("Part {part_id} is controlled by a mirror constraint and cannot be dragged")
}

/// Moves a constraint's stored numbers so its target lands `delta` away from where it was
/// and, when `turn` is given, turns that many degrees further. `None` leaves the stored
/// rotation untouched, bit for bit: adding `0.0` would rewrite a stored `-0.0`. A drag
/// passes `None`; `dragging_a_held_companion_keeps_the_exact_stored_rotation_of_either_kind`
/// in `core/tests/part_offset_constraint.rs` pins it. An `offset` stores board-axis numbers,
/// so the drag is added as it is. A `part-offset` stores source-axis numbers, so the drag is
/// first turned back into the source's frame, measured from the source's pose in `parts`.
pub(crate) fn retune_offset(
    parts: &[Part],
    constraint: &mut Constraint,
    delta: Vec2,
    turn: Option<f64>,
) -> Result<Retune, String> {
    if constraint.is_mirror() {
        return Ok(Retune::Mirror);
    }
    let source = source_pose(parts, constraint)?;
    let (offset, rotation, step) = match constraint {
        Constraint::Offset {
            offset, rotation, ..
        } => (offset, rotation, delta),
        Constraint::PartOffset {
            offset, rotation, ..
        } => (
            offset,
            rotation,
            rotate(delta, -source.rotation, Vec2::default()),
        ),
        Constraint::Mirror { .. } => return Ok(Retune::Mirror),
        Constraint::Framed { relation, .. } => match relation {
            ConstraintRelation::Offset { offset, rotation } => (offset, rotation, delta),
            ConstraintRelation::PartOffset { offset, rotation } => (
                offset,
                rotation,
                rotate(delta, -source.rotation, Vec2::default()),
            ),
            ConstraintRelation::MirrorLine { .. } => return Ok(Retune::Mirror),
        },
    };
    offset.x += step.x;
    offset.y += step.y;
    if let Some(turn) = turn {
        *rotation += turn;
    }
    Ok(Retune::Applied)
}

/// Re-expresses an offset constraint's numbers in `to`'s axes, measured from the source's
/// current pose, so the target does not move. The id and endpoints are kept. Every check
/// runs before the constraint is replaced, so a refusal changes nothing.
pub(crate) fn convert_offset(
    doc: &mut ProjectDoc,
    constraint_id: &str,
    to: OffsetAxes,
) -> Result<Vec<String>, String> {
    let index = doc
        .constraints
        .iter()
        .position(|constraint| constraint.id() == constraint_id)
        .ok_or_else(|| format!("Constraint {constraint_id} does not exist"))?;
    let source = source_pose(&doc.parts, &doc.constraints[index])?;
    let target = doc.constraints[index].target().to_string();
    if !doc.parts.iter().any(|part| part.id == target) {
        return Err(format!("Constraint {constraint_id} target is missing"));
    }
    let current = &doc.constraints[index];
    let (Some(from), Some((offset, rotation))) = (current.offset_axes(), current.offset_numbers())
    else {
        return Err(format!(
            "Constraint {constraint_id} is a mirror and has no offset axes"
        ));
    };
    if from != to {
        let id = current.id().to_owned();
        let into_axes = match to {
            OffsetAxes::Part => -source.rotation,
            OffsetAxes::Board => source.rotation,
        };
        let offset = rotate(offset, into_axes, Vec2::default());
        let replacement = match current {
            Constraint::Framed { source, target, .. } => Constraint::Framed {
                id,
                source: source.clone(),
                target: target.clone(),
                relation: match to {
                    OffsetAxes::Board => ConstraintRelation::Offset { offset, rotation },
                    OffsetAxes::Part => ConstraintRelation::PartOffset { offset, rotation },
                },
            },
            _ => Constraint::offset(
                to,
                id,
                current.source().to_owned(),
                current.target().to_owned(),
                offset,
                rotation,
            ),
        };
        doc.constraints[index] = replacement;
    }
    Ok(vec![constraint_id.into()])
}

/// Evaluates the record that drives `id`: its source first, then any key a companion endpoint
/// is projected against, so the projection reads live poses and cycles through it are refused.
fn drive(
    id: &str,
    driver: &Driver,
    parts: &BTreeMap<String, Pose2>,
    constraints: &BTreeMap<String, Driver>,
    assemblies: &BTreeMap<String, AssemblyPlacement>,
    resolved: &mut BTreeMap<String, Pose2>,
    visiting: &mut BTreeSet<String>,
) -> Result<Pose2, String> {
    let current = *parts
        .get(id)
        .ok_or_else(|| format!("Constraint target {id} is missing"))?;
    let source = pose(
        driver.source_authority(),
        parts,
        constraints,
        assemblies,
        resolved,
        visiting,
    )?;
    let mut owners = [None, None];
    let Driver::Framed(framed) = driver;
    for (slot, owner) in [&framed.source_owner, &framed.target_owner]
        .into_iter()
        .enumerate()
    {
        if let Some(owner) = owner {
            owners[slot] = Some(pose(
                owner,
                parts,
                constraints,
                assemblies,
                resolved,
                visiting,
            )?);
        }
    }
    driver.evaluate(source, owners, current)
}

fn pose(
    id: &str,
    parts: &BTreeMap<String, Pose2>,
    constraints: &BTreeMap<String, Driver>,
    assemblies: &BTreeMap<String, AssemblyPlacement>,
    resolved: &mut BTreeMap<String, Pose2>,
    visiting: &mut BTreeSet<String>,
) -> Result<Pose2, String> {
    if let Some(value) = resolved.get(id) {
        return Ok(*value);
    }
    // Report a repeated active node as a cycle even when the path is already at the
    // budget, so cycle detection keeps its precedence over depth exhaustion.
    if visiting.contains(id) {
        return Err(format!("Constraint cycle includes part {id}"));
    }
    if visiting.len() >= MAX_DEPENDENCY_DEPTH {
        return Err(format!(
            "Constraint dependency depth exceeds the limit of {MAX_DEPENDENCY_DEPTH} active parts"
        ));
    }
    visiting.insert(id.to_string());
    // An explicit framed relation outranks the implicit companion placement, as a stored one
    // does: the accepted design's "Compiler, solver and canonical edit flow" step 3
    // (docs/plans/typed-core-edits/design/139-generalized-mirrored-constraint-model.md) keeps
    // "explicit constraints ... current precedence over following-assembly placement", and
    // `companion_mode` in matrix/layout.rs documents the stored rule.
    let framed = matches!(constraints.get(id), Some(Driver::Framed(_)));
    let next = if let Some(assembly) = assemblies.get(id).filter(|_| !framed) {
        // The classification already resolved explicit-constraint precedence, so
        // a constrained companion settles from its constraint and the others
        // follow the key (an authored pose only while the key is unchanged).
        match &assembly.mode {
            MatrixCompanionMode::Constrained { .. } => {
                let driver = constraints
                    .get(id)
                    .ok_or_else(|| format!("Constraint for companion {id} is missing"))?;
                drive(
                    id,
                    driver,
                    parts,
                    constraints,
                    assemblies,
                    resolved,
                    visiting,
                )?
            }
            MatrixCompanionMode::Authored | MatrixCompanionMode::Follows => {
                let source = pose(
                    &assembly.parent_id,
                    parts,
                    constraints,
                    assemblies,
                    resolved,
                    visiting,
                )?;
                if matches!(assembly.mode, MatrixCompanionMode::Authored)
                    && let Some(authored) = assembly.authored_pose
                    && parts.get(&assembly.parent_id) == Some(&source)
                {
                    authored
                } else {
                    follow_pose(source, assembly.local_pose)
                }
            }
        }
    } else if let Some(driver) = constraints.get(id) {
        drive(
            id,
            driver,
            parts,
            constraints,
            assemblies,
            resolved,
            visiting,
        )?
    } else {
        *parts
            .get(id)
            .expect("all placement references were checked")
    };
    if !next.at.x.is_finite() || !next.at.y.is_finite() || !next.rotation.is_finite() {
        return Err(format!("Constraint target {id} has a non-finite pose"));
    }
    visiting.remove(id);
    resolved.insert(id.to_string(), next);
    Ok(next)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Part, Side};

    fn part(id: &str, x: f64, y: f64) -> Part {
        Part {
            id: id.into(),
            definition_id: "key".into(),
            reference: id.into(),
            pose: Pose2 {
                at: Vec2 { x, y },
                rotation: 15.0,
            },
            side: Side::Front,
            locked: None,
            keycap: None,
            outline: None,
            properties: None,
            generator_parameters: None,
        }
    }

    fn offset(id: &str, source: &str, target: &str) -> Constraint {
        Constraint::Offset {
            id: id.into(),
            source_part_id: source.into(),
            target_part_id: target.into(),
            offset: Vec2 { x: 10.0, y: 2.0 },
            rotation: 5.0,
        }
    }

    #[test]
    fn resolves_chain_and_mirror() {
        let mut doc = ProjectDoc::empty("p", "P");
        doc.parts = vec![
            part("a", 2.0, 3.0),
            part("b", 0.0, 0.0),
            part("c", 0.0, 0.0),
        ];
        doc.constraints = vec![
            Constraint::Mirror {
                id: "m".into(),
                source_part_id: "b".into(),
                target_part_id: "c".into(),
                axis: MirrorAxis::Vertical,
                coordinate: 0.0,
            },
            offset("o", "a", "b"),
        ];
        assert_eq!(resolve(&mut doc).unwrap(), vec!["b", "c"]);
        assert_eq!(doc.parts[1].pose.at, Vec2 { x: 12.0, y: 5.0 });
        assert_eq!(doc.parts[2].pose.at, Vec2 { x: -12.0, y: 5.0 });
        assert_eq!(doc.parts[2].pose.rotation, 160.0);
        doc.parts[0].pose.at.x = 4.0;
        assert_eq!(resolve(&mut doc).unwrap(), vec!["b", "c"]);
        assert_eq!(doc.parts[2].pose.at.x, -14.0);
    }

    #[test]
    fn rejects_invalid_graph_without_mutation() {
        let mut doc = ProjectDoc::empty("p", "P");
        doc.parts = vec![part("a", 1.0, 0.0), part("b", 2.0, 0.0)];
        doc.constraints = vec![offset("a", "a", "b"), offset("b", "b", "a")];
        let before = doc.parts.clone();
        assert!(resolve(&mut doc).unwrap_err().contains("cycle"));
        assert_eq!(doc.parts, before);
        doc.constraints[1] = offset("b", "a", "b");
        assert!(
            resolve(&mut doc)
                .unwrap_err()
                .contains("multiple constraints")
        );
        doc.constraints[1] = offset("b", "missing", "a");
        assert!(resolve(&mut doc).unwrap_err().contains("missing part"));
    }

    #[test]
    fn mirror_angles_follow_reflection_axis() {
        for (axis, source_angle, expected) in [
            (MirrorAxis::Vertical, 0.0, 180.0),
            (MirrorAxis::Vertical, 35.0, 145.0),
            (MirrorAxis::Horizontal, 0.0, 0.0),
            (MirrorAxis::Horizontal, 35.0, -35.0),
        ] {
            let mut doc = ProjectDoc::empty("p", "P");
            let mut source = part("a", 2.0, 3.0);
            source.pose.rotation = source_angle;
            doc.parts = vec![source, part("b", 0.0, 0.0)];
            doc.constraints.push(Constraint::Mirror {
                id: "m".into(),
                source_part_id: "a".into(),
                target_part_id: "b".into(),
                axis,
                coordinate: 0.0,
            });
            resolve(&mut doc).unwrap();
            assert_eq!(doc.parts[1].pose.rotation, expected);
        }
    }

    #[test]
    fn resolve_refuses_a_non_finite_number_of_any_kind_without_mutation() {
        let kinds = |x: f64, y: f64, turn: f64| {
            let offset = Vec2 { x, y };
            [
                Constraint::Offset {
                    id: "o".into(),
                    source_part_id: "a".into(),
                    target_part_id: "b".into(),
                    offset,
                    rotation: turn,
                },
                Constraint::PartOffset {
                    id: "o".into(),
                    source_part_id: "a".into(),
                    target_part_id: "b".into(),
                    offset,
                    rotation: turn,
                },
                Constraint::Mirror {
                    id: "o".into(),
                    source_part_id: "a".into(),
                    target_part_id: "b".into(),
                    axis: MirrorAxis::Vertical,
                    coordinate: x + y + turn,
                },
            ]
        };
        for bad in [f64::NAN, f64::INFINITY] {
            for (x, y, turn) in [(bad, 0.0, 0.0), (0.0, bad, 0.0), (0.0, 0.0, bad)] {
                for constraint in kinds(x, y, turn) {
                    let mut doc = ProjectDoc::empty("p", "P");
                    doc.parts = vec![part("a", 1.0, 0.0), part("b", 2.0, 0.0)];
                    doc.constraints = vec![constraint.clone()];
                    let before = doc.parts.clone();
                    let message = resolve(&mut doc).unwrap_err();
                    assert!(
                        message.contains("non-finite"),
                        "{constraint:?} gave {message}"
                    );
                    // The constraint holds a NaN, so compare the parts it would have moved.
                    assert_eq!(doc.parts, before);
                }
            }
        }
    }

    /// An accepted document never holds a constraint with a missing endpoint, so the edit API
    /// cannot reach these guards; a conversion must still refuse and change nothing if one is
    /// ever handed such a document.
    #[test]
    fn convert_offset_refuses_a_missing_endpoint_and_changes_nothing() {
        for (missing, source, target) in [("source", "ghost", "b"), ("target", "a", "ghost")] {
            for to in [OffsetAxes::Board, OffsetAxes::Part] {
                let mut doc = ProjectDoc::empty("p", "P");
                doc.parts = vec![part("a", 2.0, 3.0), part("b", 12.0, 5.0)];
                doc.constraints = vec![offset("o", source, target)];
                let before = doc.clone();
                let message = convert_offset(&mut doc, "o", to).unwrap_err();
                assert!(
                    message.contains(missing) && message.contains("missing"),
                    "{message}"
                );
                assert_eq!(doc, before, "a missing {missing} changed the document");
            }
        }
    }

    fn chain_id(index: usize) -> String {
        format!("p{index:05}")
    }

    fn chain_parts(count: usize) -> BTreeMap<String, Pose2> {
        (0..count)
            .map(|index| {
                (
                    chain_id(index),
                    Pose2 {
                        at: Vec2 {
                            x: index as f64 * 10.0,
                            y: 0.0,
                        },
                        rotation: 0.0,
                    },
                )
            })
            .collect()
    }

    fn linear_drivers<'a>(
        count: usize,
        relation: &'a ConstraintRelation,
    ) -> BTreeMap<String, Driver<'a>> {
        (0..count - 1)
            .map(|index| {
                (
                    chain_id(index),
                    Driver::Framed(Framed {
                        id: "linear",
                        source_authority: chain_id(index + 1),
                        source_frame: EndpointFrame::Identity,
                        target_frame: EndpointFrame::Identity,
                        source_owner: None,
                        target_owner: None,
                        relation,
                    }),
                )
            })
            .collect()
    }

    fn resolve_active_chain(count: usize) -> Result<Pose2, String> {
        let parts = chain_parts(count);
        let relation = ConstraintRelation::Offset {
            offset: Vec2 { x: 10.0, y: 0.0 },
            rotation: 0.0,
        };
        let constraints = linear_drivers(count, &relation);
        let assemblies = BTreeMap::new();
        let mut resolved = BTreeMap::new();
        let mut visiting = BTreeSet::new();
        pose(
            &chain_id(0),
            &parts,
            &constraints,
            &assemblies,
            &mut resolved,
            &mut visiting,
        )
    }

    #[test]
    fn active_dependency_depth_accepts_below_and_at_budget_and_refuses_above() {
        assert!(resolve_active_chain(MAX_DEPENDENCY_DEPTH - 1).is_ok());
        assert!(resolve_active_chain(MAX_DEPENDENCY_DEPTH).is_ok());
        let refused = resolve_active_chain(MAX_DEPENDENCY_DEPTH + 1).unwrap_err();
        assert!(
            refused.contains("depth exceeds")
                && refused.contains(&MAX_DEPENDENCY_DEPTH.to_string()),
            "{refused}"
        );
    }

    #[test]
    fn a_twelve_thousand_part_acyclic_chain_is_refused_without_a_stack_overflow() {
        let refused = resolve_active_chain(12_000).unwrap_err();
        assert!(refused.contains("depth exceeds"), "{refused}");
    }

    #[test]
    fn source_owner_and_target_owner_edges_share_the_active_depth_budget() {
        let relation = ConstraintRelation::Offset {
            offset: Vec2 { x: 1.0, y: 0.0 },
            rotation: 0.0,
        };
        for source_side in [true, false] {
            let count = MAX_DEPENDENCY_DEPTH;
            let parts = chain_parts(count);
            let mut constraints = BTreeMap::new();
            // The owner chain is itself linear, so every recursion step has one
            // active dependency and the budget counts it.
            for index in 0..count - 1 {
                constraints.insert(
                    chain_id(index),
                    Driver::Framed(Framed {
                        id: "owner-chain",
                        source_authority: chain_id(index + 1),
                        source_frame: EndpointFrame::Identity,
                        target_frame: EndpointFrame::Identity,
                        source_owner: None,
                        target_owner: None,
                        relation: &relation,
                    }),
                );
            }
            constraints.insert(
                "target".into(),
                Driver::Framed(Framed {
                    id: "owner-edge",
                    source_authority: "source".into(),
                    source_frame: EndpointFrame::Identity,
                    target_frame: EndpointFrame::Identity,
                    source_owner: source_side.then(|| chain_id(0)),
                    target_owner: (!source_side).then(|| chain_id(0)),
                    relation: &relation,
                }),
            );
            let mut owner_parts = parts;
            owner_parts.insert(
                "source".into(),
                Pose2 {
                    at: Vec2 { x: 0.0, y: 0.0 },
                    rotation: 0.0,
                },
            );
            owner_parts.insert(
                "target".into(),
                Pose2 {
                    at: Vec2 { x: 1.0, y: 0.0 },
                    rotation: 0.0,
                },
            );
            let mut resolved = BTreeMap::new();
            let mut visiting = BTreeSet::new();
            let refused = pose(
                "target",
                &owner_parts,
                &constraints,
                &BTreeMap::new(),
                &mut resolved,
                &mut visiting,
            )
            .unwrap_err();
            assert!(
                refused.contains("depth exceeds"),
                "{} owner edge: {refused}",
                if source_side { "source" } else { "target" }
            );
        }
    }

    #[test]
    fn implicit_assembly_parents_share_the_active_depth_budget() {
        let count = MAX_DEPENDENCY_DEPTH + 1;
        let parts = chain_parts(count);
        let assemblies: BTreeMap<_, _> = (0..count - 1)
            .map(|index| {
                (
                    chain_id(index),
                    AssemblyPlacement {
                        parent_id: chain_id(index + 1),
                        local_pose: Pose2 {
                            at: Vec2 { x: 1.0, y: 0.0 },
                            rotation: 0.0,
                        },
                        authored_pose: None,
                        mode: MatrixCompanionMode::Follows,
                    },
                )
            })
            .collect();
        let mut resolved = BTreeMap::new();
        let mut visiting = BTreeSet::new();
        let refused = pose(
            &chain_id(0),
            &parts,
            &BTreeMap::new(),
            &assemblies,
            &mut resolved,
            &mut visiting,
        )
        .unwrap_err();
        assert!(refused.contains("depth exceeds"), "{refused}");
    }

    #[test]
    fn a_cycle_still_reports_as_a_cycle_at_the_depth_budget() {
        let count = MAX_DEPENDENCY_DEPTH;
        let parts = chain_parts(count);
        let relation = ConstraintRelation::Offset {
            offset: Vec2 { x: 1.0, y: 0.0 },
            rotation: 0.0,
        };
        let mut constraints = linear_drivers(count, &relation);
        constraints.insert(
            chain_id(count - 1),
            Driver::Framed(Framed {
                id: "cycle",
                source_authority: chain_id(0),
                source_frame: EndpointFrame::Identity,
                target_frame: EndpointFrame::Identity,
                source_owner: None,
                target_owner: None,
                relation: &relation,
            }),
        );
        let mut resolved = BTreeMap::new();
        let mut visiting = BTreeSet::new();
        let error = pose(
            &chain_id(0),
            &parts,
            &constraints,
            &BTreeMap::new(),
            &mut resolved,
            &mut visiting,
        )
        .unwrap_err();
        assert!(error.contains("cycle"), "{error}");
        assert!(!error.contains("depth"), "{error}");
    }
}
