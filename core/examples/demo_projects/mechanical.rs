//! Mechanical configuration for the included split keyboards.
//!
//! Recipes still build each seed as raw JSON; this module then applies the
//! mechanical state through the public typed `apply-mechanical-edit` operation, so
//! the shipped projects carry exactly what the Case workspace would author.
use super::*;

/// The bundled mounting-hole definition closure projection needs, exactly as the
/// catalogue hands it to the Case workspace.
fn mounting_hole() -> Result<Value> {
    let d = generators::catalogue()?
        .into_iter()
        .find(|d| {
            d.generator
                .as_ref()
                .is_some_and(|g| g.source == "ceoloide/mounting_hole_npth")
        })
        .ok_or("Missing mounting hole generator")?;
    Ok(serde_json::to_value(d)?)
}

/// Submits one typed mechanical change for a physical instance.
fn instance_edit(
    engine: &mut CoreEngine,
    doc: Value,
    instance: &str,
    change: Value,
    transaction: &str,
) -> Result<Value> {
    edit(
        engine,
        doc,
        json!({"kind":"apply-mechanical-edit","edit":{"target":{"kind":"instance","instanceId":instance},"change":change,"mountingHole":mounting_hole()?}}),
        transaction,
    )
}

fn instance<'a>(doc: &'a Value, id: &str) -> Result<&'a Value> {
    a(&doc["hardware"]["instances"])
        .iter()
        .find(|i| i["id"] == id)
        .ok_or_else(|| format!("Missing instance {id}").into())
}

/// Every linked feature of every pair; absent pairs serialize as nothing.
fn pair_features(doc: &Value) -> impl Iterator<Item = &Value> {
    doc["hardware"]["pairs"]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|p| p["features"].as_array().into_iter().flatten())
}

/// A stored opening's vertices as `(id, x, y)`: the half's local copy, or the
/// pair's authored value when this half authored it.
fn opening(doc: &Value, half: &str, opening_id: &str) -> Result<Vec<(String, f64, f64)>> {
    let local = instance(doc, half)?["mechanical"]["openings"]
        .as_array()
        .and_then(|openings| openings.iter().find(|o| o["id"] == opening_id));
    let pair = pair_features(doc)
        .find(|f| f["id"] == opening_id && f["author"] == half)
        .map(|f| &f["value"]["opening"]);
    let value = local.or(pair).ok_or("Missing authored opening")?;
    Ok(a(&value["vertices"])
        .iter()
        .map(|v| {
            (
                s(&v["id"]).to_owned(),
                n(&v["point"]["x"]),
                n(&v["point"]["y"]),
            )
        })
        .collect())
}

/// Moves an axis-aligned four-vertex opening to the rectangle centred on
/// `center`, one vertex coordinate per edit. The leading side moves first so
/// every intermediate footprint stays a simple rectangle.
fn place_rectangle(
    engine: &mut CoreEngine,
    mut doc: Value,
    half: &str,
    opening_id: &str,
    center: (f64, f64),
    size: (f64, f64),
) -> Result<Value> {
    let vertices = opening(&doc, half, opening_id)?;
    let current = |axis: usize| {
        vertices
            .iter()
            .map(|v| if axis == 0 { v.1 } else { v.2 })
            .sum::<f64>()
            / vertices.len() as f64
    };
    for axis in 0..2 {
        let middle = current(axis);
        let target = if axis == 0 { center.0 } else { center.1 };
        let half_size = if axis == 0 { size.0 } else { size.1 } / 2.;
        let (first, second) = if target >= middle {
            (1., -1.)
        } else {
            (-1., 1.)
        };
        for side in [first, second] {
            for (vertex, x, y) in &vertices {
                let value = if axis == 0 { *x } else { *y };
                if (value - middle).signum() != side {
                    continue;
                }
                doc = instance_edit(
                    engine,
                    doc,
                    half,
                    json!({"set-opening-dimension":{"opening_id":opening_id,"vertex_id":vertex,"field":if axis==0{"opening-point-x"}else{"opening-point-y"},"value":target+side*half_size}}),
                    &format!("{opening_id}-{half}-{vertex}-{axis}"),
                )?;
            }
        }
    }
    Ok(doc)
}

/// The id of the opening an `add-opening` edit just created on `half`.
fn added_opening(before: &Value, after: &Value, half: &str) -> Result<String> {
    let ids = |doc: &Value| -> Vec<String> {
        let mut ids: Vec<String> = instance(doc, half)
            .map(|i| {
                i["mechanical"]["openings"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(|o| s(&o["id"]).to_owned())
                    .collect()
            })
            .unwrap_or_default();
        ids.extend(pair_features(doc).map(|f| s(&f["id"]).to_owned()));
        ids
    };
    let old = ids(before);
    ids(after)
        .into_iter()
        .find(|id| !old.contains(id))
        .ok_or_else(|| "The opening was not added".into())
}

/// How one named opening relates to the pair's second member.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Counterpart {
    /// The second member's part is the reflection of the first's, so the opening
    /// stays one linked value.
    Linked,
    /// The second member's part is elsewhere: that member's opening is local and
    /// placed around its own part.
    Local,
    /// The second member has no such part: the opening is omitted there.
    Omitted,
}

/// One named Case opening around a part of the first member.
pub(super) struct Bay {
    pub name: &'static str,
    /// The part's id below its board, such as `U1`.
    pub part: &'static str,
    pub size: (f64, f64),
    pub counterpart: Counterpart,
}

const fn bay(
    name: &'static str,
    part: &'static str,
    size: (f64, f64),
    counterpart: Counterpart,
) -> Bay {
    Bay {
        name,
        part,
        size,
        counterpart,
    }
}

/// The measured recipes' generated nice!nano bay and TRRS jack. The recipes put
/// both at each PCB's right edge, which is not the reflection of the left half's.
pub(super) const MEASURED_BAYS: [Bay; 2] = [
    bay("controller-bay", "U1", (20., 36.), Counterpart::Local),
    bay("trrs-jack", "TRRS", (12., 14.), Counterpart::Local),
];
/// Sofle's measured nice!nano and TRRS jack, reflected between its halves.
pub(super) const SOFLE_BAYS: [Bay; 2] = [
    bay("controller-bay", "U1", (20., 36.), Counterpart::Linked),
    bay("trrs-jack", "J2", (14., 12.), Counterpart::Linked),
];
/// The reused-PCB demonstration: the controller bay follows the board into both
/// copies; the reset access exists only on the unflipped copy.
pub(super) const REUSED_BAYS: [Bay; 2] = [
    bay("controller-bay", "U1", (20., 36.), Counterpart::Linked),
    bay("reset-access", "RST", (12., 12.), Counterpart::Omitted),
];

/// What a pair shares and how it is mapped.
pub(super) struct Pair<'a> {
    pub id: &'a str,
    /// The first member authors the shared construction and each linked opening.
    pub members: [&'a str; 2],
    /// The map `second = (x_offset - first.x, first.y)` between their Case frames.
    pub x_offset: f64,
    pub bays: &'a [Bay],
    /// Whether the shared construction is the settled gasket mount.
    pub gasket: bool,
}

/// One instance's part in that instance's Case frame, which a flipped copy mirrors.
fn frame_position(doc: &Value, instance_id: &str, part: &str) -> Result<(f64, f64)> {
    let instance = instance(doc, instance_id)?;
    let part_id = format!("{}/{part}", s(&instance["boardId"]));
    let part = a(&doc["parts"])
        .iter()
        .find(|p| p["id"] == part_id)
        .ok_or_else(|| format!("Missing part {part_id}"))?;
    let x = n(&part["pose"]["at"]["x"]);
    Ok((
        if instance["flipped"] == true { -x } else { x },
        n(&part["pose"]["at"]["y"]),
    ))
}

/// Gives two physical halves an explicit mechanical mirror pair.
///
/// Both halves are enabled with the Core defaults and linked with the explicit map
/// `second = (x_offset - first.x, first.y)`; the pair shares the first member's
/// construction. Each bay becomes one linked Case opening around the first
/// member's part, then follows its [`Counterpart`]: reflected and linked, local and
/// placed around the second member's own part, or omitted. Local and omitted
/// members are named, deliberate asymmetry rather than a silent second copy.
pub(super) fn link_pair(engine: &mut CoreEngine, mut doc: Value, pair: &Pair) -> Result<Value> {
    let [first, second] = pair.members;
    for half in pair.members {
        // Enabling proposes closure mounts from the outline; they are kept as Core
        // proposes them, so each shipped half carries its closure holes.
        doc = instance_edit(
            engine,
            doc,
            half,
            json!("enable"),
            &format!("enable-{half}"),
        )?;
    }
    if pair.gasket {
        doc = instance_edit(
            engine,
            doc,
            first,
            json!({"set-mount":"gasket"}),
            &format!("gasket-{first}"),
        )?;
    }
    doc = instance_edit(
        engine,
        doc,
        first,
        json!({"link-mirror-pair":{"pair_id":pair.id,"members":[first,second],"mapping":{"at":{"x":pair.x_offset,"y":0},"rotation":0},"construction_source":first,"correspondences":[]}}),
        &format!("link-{}", pair.id),
    )?;
    for bay in pair.bays {
        let first_at = frame_position(&doc, first, bay.part)?;
        let before = doc.clone();
        doc = instance_edit(
            engine,
            doc,
            first,
            json!("add-opening"),
            &format!("{}-add", bay.name),
        )?;
        let id = added_opening(&before, &doc, first)?;
        doc = place_rectangle(engine, doc, first, &id, first_at, bay.size)?;
        match bay.counterpart {
            Counterpart::Linked => {
                let second_at = frame_position(&doc, second, bay.part)?;
                if ((pair.x_offset - first_at.0) - second_at.0).abs() > 1e-6
                    || (first_at.1 - second_at.1).abs() > 1e-6
                {
                    return Err(format!("{} is not mirrored by the pair map", bay.name).into());
                }
            }
            Counterpart::Local => {
                doc = instance_edit(
                    engine,
                    doc,
                    second,
                    json!({"make-feature-local":{"feature_id":id,"instance_id":second}}),
                    &format!("{}-local", bay.name),
                )?;
                let second_at = frame_position(&doc, second, bay.part)?;
                doc = place_rectangle(engine, doc, second, &id, second_at, bay.size)?;
            }
            Counterpart::Omitted => {
                doc = instance_edit(
                    engine,
                    doc,
                    second,
                    json!({"omit-feature-member":{"feature_id":id,"instance_id":second}}),
                    &format!("{}-omit", bay.name),
                )?;
            }
        }
    }
    Ok(doc)
}
