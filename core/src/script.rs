use crate::model::{Operation, OutlineFeature, Part, Pose2, ProjectDoc, Side, Vec2};
use rhai::{Dynamic, Engine, EvalAltResult, Position};
use std::cell::RefCell;
use std::rc::Rc;

const MAX_OPERATIONS: u64 = 100_000;
const MAX_SOURCE_BYTES: usize = 256 * 1024;
const MAX_EMISSIONS: usize = 10_000;
const MAX_STRING_BYTES: usize = 1024 * 1024;
/// Aggregate UTF-8 bytes copied from every string field emitted by one script.
/// Rhai's per-string limit does not bound the callback-owned output, so this is
/// the finite ceiling that keeps a hostile script from growing Core storage.
const MAX_EMISSION_STRING_BYTES: usize = 1024 * 1024;
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

/// Shared emission accounting for one evaluation. Exhaustion is sticky: once a
/// budget is exceeded, every later call refuses so a script cannot resume.
#[derive(Default)]
struct EmissionBudget {
    emissions: usize,
    string_bytes: usize,
    exhausted: bool,
}

/// Non-catchable termination: `try/catch` cannot swallow budget exhaustion.
fn budget_exhausted() -> Box<EvalAltResult> {
    Box::new(EvalAltResult::ErrorTerminated(
        Dynamic::UNIT,
        Position::NONE,
    ))
}

impl EmissionBudget {
    /// Charge one emission and all of its copied string fields before any copy.
    fn charge(&mut self, field_bytes: &[usize]) -> Result<(), Box<EvalAltResult>> {
        if self.exhausted {
            return Err(budget_exhausted());
        }
        let mut bytes = self.string_bytes;
        let mut fits = self.emissions < MAX_EMISSIONS;
        for len in field_bytes {
            match bytes.checked_add(*len) {
                Some(total) if total <= MAX_EMISSION_STRING_BYTES => bytes = total,
                _ => {
                    fits = false;
                    break;
                }
            }
        }
        if !fits {
            self.exhausted = true;
            return Err(budget_exhausted());
        }
        self.emissions += 1;
        self.string_bytes = bytes;
        Ok(())
    }
}

fn collect(source: &str) -> Result<Vec<Emission>, String> {
    if source.len() > MAX_SOURCE_BYTES {
        return Err("Script source is too large".into());
    }
    let output = Rc::new(RefCell::new(Vec::new()));
    let budget = Rc::new(RefCell::new(EmissionBudget::default()));
    let mut engine = Engine::new_raw();
    engine.set_max_operations(MAX_OPERATIONS);
    engine.set_max_string_size(MAX_STRING_BYTES);
    engine.set_max_array_size(MAX_ARRAY_SIZE);
    engine.set_max_map_size(MAX_MAP_SIZE);
    engine.set_max_call_levels(MAX_CALL_LEVELS);
    engine.set_max_expr_depths(MAX_EXPR_DEPTH, MAX_EXPR_DEPTH);
    let parts = Rc::clone(&output);
    let parts_budget = Rc::clone(&budget);
    engine.register_fn(
        "part",
        move |group: &str,
              local: &str,
              definition: &str,
              reference: &str,
              x: f64,
              y: f64,
              rotation: f64|
              -> Result<(), Box<EvalAltResult>> {
            parts_budget.borrow_mut().charge(&[
                group.len(),
                local.len(),
                definition.len(),
                reference.len(),
            ])?;
            parts.borrow_mut().push(Emission::Part {
                group: group.into(),
                local: local.into(),
                definition: definition.into(),
                reference: reference.into(),
                x,
                y,
                rotation,
            });
            Ok(())
        },
    );
    let rects = Rc::clone(&output);
    let rects_budget = Rc::clone(&budget);
    engine.register_fn(
        "rect",
        move |group: &str,
              local: &str,
              x: f64,
              y: f64,
              width: f64,
              height: f64,
              radius: f64,
              operation: &str|
              -> Result<(), Box<EvalAltResult>> {
            rects_budget
                .borrow_mut()
                .charge(&[group.len(), local.len(), operation.len()])?;
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
            Ok(())
        },
    );
    let outcome = engine.eval::<()>(source);
    drop(engine);
    // Budget exhaustion fails the script even if it swallowed the termination.
    if budget.borrow().exhausted {
        return Err("Script emission budget exceeded".into());
    }
    outcome.map_err(|error| error.to_string())?;
    Ok(Rc::try_unwrap(output)
        .map_err(|_| "Script output is still shared".to_string())?
        .into_inner())
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
                        anchor: None,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Script;

    /// Build a Rhai string variable by appending `chunk` `times` times, so the
    /// test controls the exact byte length without relying on loop counts alone.
    fn fill(name: &str, chunk: &str, times: usize) -> String {
        format!(
            "let {name} = \"\"; let n = 0; while n < {times} {{ {name} += \"{chunk}\"; n += 1; }}"
        )
    }

    #[test]
    fn collect_accepts_an_emission_exactly_on_the_aggregate_string_byte_budget() {
        let chunk = "a".repeat(1024);
        let field = 1024 * 1024; // chunk bytes * times
        let source = format!(
            "{}part(\"\", \"\", s, \"\", 0.0, 0.0, 0.0);",
            fill("s", &chunk, field / 1024)
        );
        let output = collect(&source).expect("an emission exactly on the byte budget is allowed");
        assert_eq!(output.len(), 1);
    }

    #[test]
    fn collect_refuses_the_first_emission_beyond_the_aggregate_string_byte_budget() {
        let chunk = "a".repeat(1000);
        let source = format!(
            "{}{}part(\"\", \"\", s1, s2, 0.0, 0.0, 0.0);",
            fill("s1", &chunk, 600),
            fill("s2", &chunk, 600)
        );
        assert!(
            collect(&source).is_err(),
            "an emission of {} bytes must be refused before copying it",
            1_200_000
        );
    }

    #[test]
    fn collect_counts_emitted_string_fields_as_utf8_bytes() {
        // Each field is 600_000 bytes (307_200 two-byte characters); both
        // together are 1_200_000 bytes but only 614_400 characters.
        let chunk = "\u{e9}".repeat(512); // 1024 bytes, 512 characters
        let two_fields = format!(
            "{}{}part(\"\", \"\", s1, s2, 0.0, 0.0, 0.0);",
            fill("s1", &chunk, 600),
            fill("s2", &chunk, 600)
        );
        assert!(
            collect(&two_fields).is_err(),
            "byte accounting must refuse 1_200_000 bytes even though it is under 1_048_576 characters"
        );
        let one_field = format!(
            "{}part(\"\", \"\", s, \"\", 0.0, 0.0, 0.0);",
            fill("s", &chunk, 600)
        );
        assert!(
            collect(&one_field).is_ok(),
            "a single 600_000-byte field is allowed"
        );
    }

    #[test]
    fn collect_shares_one_budget_across_part_and_rect_emissions() {
        let chunk = "a".repeat(1024);
        // A part, then a rect, then another rect: 3 * 400_000 bytes on one budget.
        let mixed_over = format!(
            "{}{}part(\"\", \"\", s, \"\", 0.0, 0.0, 0.0); rect(\"\", \"\", 0.0, 0.0, 20.0, 10.0, 0.0, t); rect(\"\", \"\", 0.0, 0.0, 20.0, 10.0, 0.0, s);",
            fill("s", &"a".repeat(1000), 400),
            fill("t", &"a".repeat(1000), 400)
        );
        assert!(
            collect(&mixed_over).is_err(),
            "part and rect emissions must draw on the same byte budget"
        );
        // Two 524_288-byte fields total exactly on the budget.
        let boundary = format!(
            "{}{}part(\"\", \"\", s, \"\", 0.0, 0.0, 0.0); rect(\"\", \"\", 0.0, 0.0, 20.0, 10.0, 0.0, t);",
            fill("s", &chunk, 512),
            fill("t", &chunk, 512)
        );
        assert_eq!(
            collect(&boundary)
                .expect("a mixed emission exactly on the budget is allowed")
                .len(),
            2
        );
    }

    #[test]
    fn collect_stays_failed_after_a_script_tries_to_catch_the_budget_error() {
        let chunk = "a".repeat(1000);
        let source = format!(
            "{}{}try {{ part(\"\", \"\", s1, s2, 0.0, 0.0, 0.0); }} catch (err) {{ part(\"g\", \"k\", \"d\", \"r\", 0.0, 0.0, 0.0); }}",
            fill("s1", &chunk, 600),
            fill("s2", &chunk, 600)
        );
        assert!(
            collect(&source).is_err(),
            "a script must not catch budget exhaustion and publish a truncated result"
        );
    }

    #[test]
    fn collect_refuses_emissions_past_the_object_count_budget() {
        // Unrolled emissions stay within the source and operation limits while
        // reaching the count budget; a loop's per-iteration cost does not.
        let source = |count: usize| {
            format!(
                "let s = \"\"; let a = 0.0; {}",
                "part(s,s,s,s,a,a,a);".repeat(count)
            )
        };
        assert_eq!(
            collect(&source(10_000))
                .expect("exactly the count budget is allowed")
                .len(),
            10_000
        );
        assert!(
            collect(&source(10_001)).is_err(),
            "the first emission past the count budget is refused"
        );
        let caught = format!(
            "{}try {{ part(s,s,s,s,a,a,a); }} catch (err) {{ part(s,s,s,s,a,a,a); }}",
            source(10_000)
        );
        assert!(
            collect(&caught).is_err(),
            "a script must not catch the count budget and emit again"
        );
    }

    #[test]
    fn apply_scripts_keeps_the_document_when_the_emission_budget_is_exceeded() {
        let chunk = "a".repeat(1024);
        let source = format!(
            "{}rect(\"main\", \"board\", 0.0, 0.0, 20.0, 10.0, 0.0, \"add\"); rect(\"main\", s, 0.0, 0.0, 20.0, 10.0, 0.0, \"add\");",
            fill("s", &chunk, 1024)
        );
        let mut doc = ProjectDoc::empty("p", "Project");
        doc.scripts.push(Script {
            id: "bad".into(),
            name: "Bad".into(),
            enabled: true,
            source,
        });
        let before = doc.clone();
        assert!(
            apply_scripts(&mut doc).is_err(),
            "the oversized script must fail the apply"
        );
        assert_eq!(
            doc, before,
            "a failed apply must leave the accepted document unchanged"
        );
        assert!(
            doc.outline.is_empty(),
            "no partial script output may be accepted"
        );
    }
}
