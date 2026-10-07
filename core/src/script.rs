use crate::model::{Operation, OutlineFeature, Part, Pose2, ProjectDoc, Side, Vec2};
use rhai::Engine;
use std::cell::RefCell;
use std::rc::Rc;

const MAX_OPERATIONS: u64 = 100_000;
const MAX_SOURCE_BYTES: usize = 256 * 1024;
const MAX_EMISSIONS: usize = 10_000;
const MAX_STRING_BYTES: usize = 1024 * 1024;
const MAX_ARRAY_SIZE: usize = 10_000;
const MAX_MAP_SIZE: usize = 1_000;
const MAX_CALL_LEVELS: usize = 32;
const MAX_EXPR_DEPTH: usize = 64;
const PREFIX: &str = "script/";

enum Emission {
    Part {
        group: String,
        local: String,
        definition: String,
        reference: String,
        x: f64,
        y: f64,
        rotation: f64,
    },
    Rect {
        group: String,
        local: String,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        radius: f64,
        operation: String,
    },
}

fn segment(value: &str) -> Result<(), String> {
    if !value.is_empty()
        && value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
    {
        return Ok(());
    }
    Err(format!("Invalid script ID segment: {value}"))
}

fn stable_id(script: &str, group: &str, local: &str) -> Result<String, String> {
    segment(script)?;
    segment(group)?;
    segment(local)?;
    Ok(format!("{PREFIX}{script}/{group}/{local}"))
}

fn collect(source: &str) -> Result<Vec<Emission>, String> {
    if source.len() > MAX_SOURCE_BYTES {
        return Err("Script source is too large".into());
    }
    let output = Rc::new(RefCell::new(Vec::new()));
    let mut engine = Engine::new_raw();
    engine.set_max_operations(MAX_OPERATIONS);
    engine.set_max_string_size(MAX_STRING_BYTES);
    engine.set_max_array_size(MAX_ARRAY_SIZE);
    engine.set_max_map_size(MAX_MAP_SIZE);
    engine.set_max_call_levels(MAX_CALL_LEVELS);
    engine.set_max_expr_depths(MAX_EXPR_DEPTH, MAX_EXPR_DEPTH);
    let parts = Rc::clone(&output);
    engine.register_fn(
        "part",
        move |group: &str,
              local: &str,
              definition: &str,
              reference: &str,
              x: f64,
              y: f64,
              rotation: f64| {
            parts.borrow_mut().push(Emission::Part {
                group: group.into(),
                local: local.into(),
                definition: definition.into(),
                reference: reference.into(),
                x,
                y,
                rotation,
            });
        },
    );
    let rects = Rc::clone(&output);
    engine.register_fn(
        "rect",
        move |group: &str,
              local: &str,
              x: f64,
              y: f64,
              width: f64,
              height: f64,
              radius: f64,
              operation: &str| {
            rects.borrow_mut().push(Emission::Rect {
                group: group.into(),
                local: local.into(),
                x,
                y,
                width,
                height,
                radius,
                operation: operation.into(),
            });
        },
    );
    engine
        .eval::<()>(source)
        .map_err(|error| error.to_string())?;
    drop(engine);
    let result = Rc::try_unwrap(output)
        .map_err(|_| "Script output is still shared".to_string())?
        .into_inner();
    if result.len() > MAX_EMISSIONS {
        return Err("Script emitted too many objects".into());
    }
    Ok(result)
}

/// Rebuild reserved script IDs, preserving a part's visual pose when the ID survives.
pub fn apply_scripts(doc: &mut ProjectDoc) -> Result<(), String> {
    let old_poses: std::collections::BTreeMap<_, _> = doc
        .parts
        .iter()
        .filter(|part| part.id.starts_with(PREFIX))
        .map(|part| (part.id.clone(), part.pose))
        .collect();
    let mut next = doc.clone();
    next.parts.retain(|part| !part.id.starts_with(PREFIX));
    next.outline
        .retain(|feature| !feature.id().starts_with(PREFIX));
    let mut emitted = std::collections::BTreeSet::new();
    let mut part_ids = std::collections::BTreeSet::new();
    let mut outline_ids = std::collections::BTreeSet::new();
    for script in doc.scripts.iter().filter(|script| script.enabled) {
        segment(&script.id)?;
        let output = collect(&script.source)
            .map_err(|message| format!("Script {}: {message}", script.id))?;
        for item in output {
            match item {
                Emission::Part {
                    group,
                    local,
                    definition,
                    reference,
                    x,
                    y,
                    rotation,
                } => {
                    let id = stable_id(&script.id, &group, &local)?;
                    if !emitted.insert(id.clone()) {
                        return Err(format!("Duplicate script object {id}"));
                    }
                    if !next.definitions.iter().any(|entry| entry.id == definition) {
                        return Err(format!("Unknown part definition {definition}"));
                    }
                    if !x.is_finite() || !y.is_finite() || !rotation.is_finite() {
                        return Err(format!("Non-finite part pose {id}"));
                    }
                    let pose = old_poses.get(&id).copied().unwrap_or(Pose2 {
                        at: Vec2 { x, y },
                        rotation,
                    });
                    part_ids.insert(id.clone());
                    next.parts.push(Part {
                        id,
                        definition_id: definition,
                        reference,
                        pose,
                        side: Side::Front,
                        locked: None,
                        keycap: None,
                        outline: None,
                        properties: None,
                        generator_parameters: None,
                    });
                }
                Emission::Rect {
                    group,
                    local,
                    x,
                    y,
                    width,
                    height,
                    radius,
                    operation,
                } => {
                    let id = stable_id(&script.id, &group, &local)?;
                    if !emitted.insert(id.clone()) {
                        return Err(format!("Duplicate script object {id}"));
                    }
                    if ![x, y, width, height, radius]
                        .iter()
                        .all(|value| value.is_finite())
                        || width <= 0.0
                        || height <= 0.0
                        || radius < 0.0
                    {
                        return Err(format!("Invalid rectangle {id}"));
                    }
                    let operation = match operation.as_str() {
                        "add" => Operation::Add,
                        "subtract" => Operation::Subtract,
                        _ => return Err(format!("Invalid outline operation {operation}")),
                    };
                    outline_ids.insert(id.clone());
                    next.outline.push(OutlineFeature::Rect {
                        rotation: None,
                        anchor_part_id: None,
                        id,
                        center: Vec2 { x, y },
                        size: Vec2 {
                            x: width,
                            y: height,
                        },
                        radius,
                        operation,
                    });
                }
            }
        }
    }
    for board in &mut next.boards {
        board
            .part_ids
            .retain(|id| !id.starts_with(PREFIX) || part_ids.contains(id));
        board
            .outline_ids
            .retain(|id| !id.starts_with(PREFIX) || outline_ids.contains(id));
    }
    if let Some(board) = next.boards.first_mut() {
        for id in &part_ids {
            if !board.part_ids.contains(id) {
                board.part_ids.push(id.clone());
            }
        }
        for id in &outline_ids {
            if !board.outline_ids.contains(id) {
                board.outline_ids.push(id.clone());
            }
        }
    }
    for net in &mut next.nets {
        net.pins
            .retain(|pin| !pin.part_id.starts_with(PREFIX) || part_ids.contains(&pin.part_id));
    }
    for matrix in &mut next.matrices {
        matrix
            .part_ids
            .retain(|id| !id.starts_with(PREFIX) || part_ids.contains(id));
    }
    for feature in &mut next.outline {
        if let OutlineFeature::PartEnvelope { part_ids: ids, .. } = feature {
            ids.retain(|id| !id.starts_with(PREFIX) || part_ids.contains(id));
        }
    }
    *doc = next;
    Ok(())
}
