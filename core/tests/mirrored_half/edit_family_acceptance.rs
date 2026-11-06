//! Public Core commit/history acceptance for parent 19's edit families.
//!
//! These cases close the activated-route gap recorded in
//! `.evidence/ticket19/fresh-acceptance-inventory.md`: representative independent Part,
//! embedded-circuit member, outline-feature, and plain-net edits are committed from either
//! side, with exact counterpart projection and one-step document history. The inventory's
//! remaining matrix/key breadth, fixed outline-version operations, and base/thumb context
//! remain separate acceptance work.
use super::*;

const ANGLED_MAP: f64 = 90.0;

fn accepted_pair(pair_id: &str) -> (CoreEngine, Value) {
    accepted_pair_from(pair_id, &runtime_supported_document())
}

fn accepted_pair_from(pair_id: &str, source: &Value) -> (CoreEngine, Value) {
    let (mut engine, seed) = seed(source, "left");
    let created = create(
        &mut engine,
        &seed,
        pair_id,
        "left",
        new_board("right", "right"),
        json!({"at":{"x":200.0,"y":30.0},"rotation":ANGLED_MAP}),
    )
    .unwrap();
    (engine, created["document"].clone())
}

fn module_definition() -> Value {
    json!({
        "id":"parent19-mounted-module-definition",
        "name":"Parent 19 mounted module",
        "family":"fixture",
        "variant":"v1",
        "source":{"repository":"https://example.invalid/parent19-module",
            "revision":"r1","path":"module.kicad_pcb","license":"MIT"},
        "board":{"contours":[]},
        "mounts":[],"volumes":[],"openings":[],"models":[],"gates":[],"interfaces":[],
        "electrical":{"protocol":"pass-through","requiredSignals":[],"logicVoltage":3.3},
        "constituents":[]
    })
}

fn with_mounted_module(mut document: Value) -> Value {
    document["moduleDefinitions"] = json!([module_definition()]);
    document["modules"] = json!([{
        "id":"left/parent19-module",
        "definitionId":"parent19-mounted-module-definition",
        "hostBoardId":"left",
        "hostFace":"front",
        "facingFace":"back",
        "at":{"x":12.0,"y":18.0},
        "rotation":25.0,
        "gap":3.0,
        "attachment":"board",
        "detached":false,
        "serviceClearance":0.0
    }]);
    document
}

fn with_fixed_outline_versions(mut document: Value) -> Value {
    let features = document["outline"].clone();
    let settings = features[0]["settings"].clone();
    let version = |id: &str, name: &str| {
        json!({
            "id":id,
            "name":name,
            "source":{"revision":0},
            "geometry":{"features":features,"settings":settings,"expectedRegions":1}
        })
    };
    document["boardOutlines"] = json!([{
        "boardId":"left",
        "activeVersionId":"left/parent19-version-a",
        "versions":[
            version("left/parent19-version-a","Version A"),
            version("left/parent19-version-b","Version B")
        ]
    }]);
    document
}

fn outline_version<'a>(document: &'a Value, board_id: &str, id: &str) -> &'a Value {
    document["boardOutlines"]
        .as_array()
        .unwrap()
        .iter()
        .find(|state| state["boardId"] == board_id)
        .unwrap_or_else(|| panic!("outline state for {board_id} is missing"))["versions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|version| version["id"] == id)
        .unwrap_or_else(|| panic!("outline version {id} on {board_id} is missing"))
}

fn mounted_module<'a>(document: &'a Value, id: &str) -> &'a Value {
    document["modules"]
        .as_array()
        .unwrap()
        .iter()
        .find(|module| module["id"] == id)
        .unwrap_or_else(|| panic!("mounted module {id} is missing"))
}

fn commit(
    engine: &mut CoreEngine,
    document: &Value,
    id: &str,
    target_ids: &[&str],
    operation: Value,
) -> Value {
    request(
        engine,
        json!({"id":id,"kind":"edit","command":{
            "baseRevision":document["revision"],
            "transactionId":id,
            "phase":"commit",
            "targetIds":target_ids,
            "operation":operation
        }}),
    )
}

fn assert_history_round_trip(
    engine: &mut CoreEngine,
    before: &Value,
    committed: &Value,
    label: &str,
) {
    let undone = request(engine, json!({"id":format!("{label}-undo"),"kind":"undo"}));
    assert_eq!(undone["kind"], "scene", "{label} Undo: {undone}");
    let mut expected_before = before.clone();
    expected_before["revision"] = undone["document"]["revision"].clone();
    assert_eq!(
        undone["document"], expected_before,
        "{label} Undo restores the full document"
    );

    let redone = request(engine, json!({"id":format!("{label}-redo"),"kind":"redo"}));
    assert_eq!(redone["kind"], "scene", "{label} Redo: {redone}");
    let mut expected_after = committed.clone();
    expected_after["revision"] = redone["document"]["revision"].clone();
    assert_eq!(
        redone["document"], expected_after,
        "{label} Redo restores the committed document"
    );
}

fn assert_repeated_edit_is_noop(
    engine: &mut CoreEngine,
    committed: &Value,
    id: &str,
    target_ids: &[&str],
    operation: Value,
) {
    let repeated = commit(engine, committed, id, target_ids, operation);
    assert_eq!(repeated["kind"], "scene", "{repeated}");
    assert_eq!(
        &repeated["document"],
        committed,
        "repeating an accepted edit with the same value must preserve revision and document; \
         scene changed IDs: {}; revision: {} -> {}; document field deltas (excluding revision): {:?}",
        repeated["scene"]["changedIds"],
        committed["revision"],
        repeated["document"]["revision"],
        document_field_deltas(committed, &repeated["document"])
    );
}

fn document_field_deltas(before: &Value, after: &Value) -> Vec<String> {
    fn collect(before: &Value, after: &Value, path: &str, deltas: &mut Vec<String>) {
        if before == after {
            return;
        }
        match (before, after) {
            (Value::Object(before), Value::Object(after)) => {
                let mut keys: Vec<_> = before.keys().chain(after.keys()).collect();
                keys.sort_unstable();
                keys.dedup();
                for key in keys {
                    if path == "$" && key == "revision" {
                        continue;
                    }
                    let child = format!("{path}.{key}");
                    match (before.get(key), after.get(key)) {
                        (Some(before), Some(after)) => collect(before, after, &child, deltas),
                        (before, after) => {
                            deltas.push(format!("{child}: {before:?} -> {after:?}"));
                        }
                    }
                }
            }
            (Value::Array(before), Value::Array(after)) => {
                for index in 0..before.len().max(after.len()) {
                    let child = format!("{path}[{index}]");
                    match (before.get(index), after.get(index)) {
                        (Some(before), Some(after)) => collect(before, after, &child, deltas),
                        (before, after) => {
                            deltas.push(format!("{child}: {before:?} -> {after:?}"));
                        }
                    }
                }
            }
            _ => deltas.push(format!("{path}: {before} -> {after}")),
        }
    }

    let mut deltas = Vec::new();
    collect(before, after, "$", &mut deltas);
    deltas
}

fn corresponding_id(document: &Value, pair_id: &str, kind: &str, id: &str) -> String {
    let record = pair(document, pair_id)["halves"]["correspondences"]
        .as_array()
        .unwrap()
        .iter()
        .find(|record| {
            record["kind"] == kind && (record["sourceId"] == id || record["mirrorId"] == id)
        })
        .unwrap_or_else(|| panic!("missing {kind} correspondence for {id}"));
    if record["sourceId"] == id {
        record["mirrorId"].as_str().unwrap().to_owned()
    } else {
        record["sourceId"].as_str().unwrap().to_owned()
    }
}

fn assert_prior_correspondences_preserved(before: &Value, after: &Value, pair_id: &str) {
    let previous = pair(before, pair_id)["halves"]["correspondences"]
        .as_array()
        .unwrap();
    let current = pair(after, pair_id)["halves"]["correspondences"]
        .as_array()
        .unwrap();
    for record in previous {
        assert!(
            current.contains(record),
            "unrelated correspondence changed or disappeared: {record}"
        );
    }
}

fn outline_feature<'a>(document: &'a Value, id: &str) -> &'a Value {
    document["outline"]
        .as_array()
        .unwrap()
        .iter()
        .find(|feature| feature["id"] == id)
        .unwrap_or_else(|| panic!("outline feature {id} is missing"))
}

/// An independent Part payload and pose authored on either member keep their explicit fields,
/// project the pose through the rotated pair map, and publish one undoable edit.
#[test]
fn parent19_independent_part_add_projects_payload_and_pose_from_either_half() {
    for author in ["left", "right"] {
        let pair_id = format!("parent19-part-add-{author}");
        let (mut engine, before) = accepted_pair(&pair_id);
        let target_id = format!("{author}/independent-added");
        let counterpart_id = if author == "left" {
            "right/independent-added"
        } else {
            "left/independent-added"
        };
        let authored_pose = json!({"at":{"x":12.0,"y":17.0},"rotation":23.0});
        let operation = json!({"kind":"add-part","boardId":author,"part":{
            "id":target_id,
            "definitionId":"synthetic/reset",
            "reference":"SW-PARENT19",
            "pose":authored_pose,
            "side":"front",
            "properties":{"assembly":"hand-authored","acceptanceTag":"parent19"}
        }});
        let committed = commit(
            &mut engine,
            &before,
            &format!("parent19-add-part-{author}"),
            &[],
            operation,
        );
        assert_eq!(committed["kind"], "scene", "{committed}");
        let after = &committed["document"];
        assert_eq!(after["revision"], before["revision"].as_u64().unwrap() + 1);
        assert_eq!(part(after, &target_id)["pose"], authored_pose);
        assert_eq!(part(after, &target_id)["definitionId"], "synthetic/reset");
        assert_eq!(part(after, &target_id)["reference"], "SW-PARENT19");
        assert_eq!(
            part(after, &target_id)["properties"]["acceptanceTag"],
            "parent19"
        );
        assert_eq!(
            part(after, counterpart_id)["definitionId"],
            "synthetic/reset"
        );
        let counterpart_reference = part(after, counterpart_id)["reference"]
            .as_str()
            .expect("generated mirror reference is a string");
        assert!(!counterpart_reference.is_empty());
        assert_eq!(
            after["parts"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|part| part["reference"].as_str() == Some(counterpart_reference))
                .count(),
            1,
            "generated mirror reference remains collision-free"
        );
        assert_eq!(
            part(after, counterpart_id)["properties"]["acceptanceTag"],
            "parent19"
        );
        let (source_id, mirror_id) = if author == "left" {
            (target_id.as_str(), counterpart_id)
        } else {
            (counterpart_id, target_id.as_str())
        };
        let source_pose = &part(after, source_id)["pose"]["at"];
        let mirror_pose = &part(after, mirror_id)["pose"]["at"];
        assert_point(
            (
                mirror_pose["x"].as_f64().unwrap(),
                mirror_pose["y"].as_f64().unwrap(),
            ),
            reflected_point(
                &json!({"at":{"x":200.0,"y":30.0},"rotation":ANGLED_MAP}),
                (
                    source_pose["x"].as_f64().unwrap(),
                    source_pose["y"].as_f64().unwrap(),
                ),
            ),
            "independent Part forward projection from actual source pose",
        );
        assert_prior_correspondences_preserved(&before, after, &pair_id);
        assert_eq!(
            corresponding_id(after, &pair_id, "part", source_id),
            mirror_id,
            "the new independent Part receives its one intended correspondence"
        );
        // AddPart is intentionally not a repeatable no-op: the same identity is a duplicate.
        assert_history_round_trip(&mut engine, &before, after, &format!("part-add-{author}"));
    }
}

/// A hosted circuit member frame edit authored from each host follows the same pair map while
/// preserving its embedded-circuit owner and every other correspondence.
#[test]
fn parent19_hosted_module_member_frame_commits_from_either_half() {
    for author in ["left", "right"] {
        let pair_id = format!("parent19-module-frame-{author}");
        let (mut engine, paired) = accepted_pair(&pair_id);
        let embedded_id = format!("{author}/runtime-circuit");
        let embedded = request(
            &mut engine,
            json!({"id":format!("parent19-embed-{author}"),"kind":"edit","command":{
                "baseRevision":paired["revision"],
                "transactionId":format!("parent19-embed-{author}"),
                "phase":"commit",
                "targetIds":[embedded_id],
                "operation":{"kind":"embed-module-circuit","id":embedded_id,
                    "definition":imported_haptic(),"hostBoardId":author,
                    "pose":{"at":{"x":120.0,"y":40.0},"rotation":370.0},
                    "side":"front","joins":{}}
            }}),
        );
        assert_eq!(embedded["kind"], "scene", "{embedded}");
        let with_circuit = embedded["document"].clone();
        let member_pair = pairs_of_kind(&with_circuit, &pair_id, "part")
            .into_iter()
            .find(|(source_id, mirror_id)| {
                source_id.starts_with("embedded/left/runtime-circuit/")
                    && mirror_id.starts_with("embedded/right/runtime-circuit/")
            })
            .unwrap_or_else(|| panic!("embedded member pair is missing: {with_circuit}"));
        let authored_id = if author == "left" {
            member_pair.0.as_str()
        } else {
            member_pair.1.as_str()
        };
        let original_authored = part(&with_circuit, authored_id).clone();
        let requested = if author == "left" {
            json!({"x":71.0,"y":29.0})
        } else {
            json!({"x":181.0,"y":53.0})
        };
        let committed = commit(
            &mut engine,
            &with_circuit,
            &format!("parent19-member-move-{author}"),
            &[authored_id],
            json!({"kind":"move-parts","positions":[{"id":authored_id,"at":requested}]}),
        );
        assert_eq!(committed["kind"], "scene", "{committed}");
        let after = &committed["document"];
        assert_eq!(part(after, authored_id)["pose"]["at"], requested);
        let source_position = &part(after, &member_pair.0)["pose"]["at"];
        let mirror_position = &part(after, &member_pair.1)["pose"]["at"];
        assert_point(
            (
                mirror_position["x"].as_f64().unwrap(),
                mirror_position["y"].as_f64().unwrap(),
            ),
            reflected_point(
                &json!({"at":{"x":200.0,"y":30.0},"rotation":ANGLED_MAP}),
                (
                    source_position["x"].as_f64().unwrap(),
                    source_position["y"].as_f64().unwrap(),
                ),
            ),
            "hosted member forward projection from actual source pose",
        );
        let mut unchanged_authored = original_authored;
        unchanged_authored["pose"]["at"] = requested.clone();
        assert_eq!(part(after, authored_id), &unchanged_authored);
        assert_eq!(
            pair(after, &pair_id)["halves"]["correspondences"],
            pair(&with_circuit, &pair_id)["halves"]["correspondences"]
        );
        assert_repeated_edit_is_noop(
            &mut engine,
            after,
            &format!("parent19-member-repeat-{author}"),
            &[authored_id],
            json!({"kind":"move-parts","positions":[{"id":authored_id,"at":requested}]}),
        );
        assert_history_round_trip(
            &mut engine,
            &with_circuit,
            after,
            &format!("module-frame-{author}"),
        );
    }
}

/// A source or mirror outline-feature edit projects only that feature and remains one
/// document-history unit under the rotated map.
#[test]
fn parent19_outline_feature_edit_commits_from_either_half() {
    for author in ["left", "right"] {
        let pair_id = format!("parent19-outline-{author}");
        let (mut engine, before) = accepted_pair(&pair_id);
        let authored_id = if author == "left" {
            "synthetic-boundary"
        } else {
            "right/synthetic-boundary"
        };
        let counterpart_id = corresponding_id(&before, &pair_id, "outline", authored_id);
        let mut feature = outline_feature(&before, authored_id).clone();
        feature["center"] = json!({"x":14.0,"y":9.0});
        let board = author;
        let committed = commit(
            &mut engine,
            &before,
            &format!("parent19-set-outline-{author}"),
            &[authored_id],
            // Ticket 37: raw SetOutline is deleted; SetFeature is the live typed write.
            json!({"kind":"apply-outline-edit","edit":{
                "target":{"kind":"board","boardId":board},
                "change":{"set-feature":{"feature":feature}},
                "seed":41, "reviewed": null}}),
        );
        assert_eq!(committed["kind"], "scene", "{committed}");
        let after = &committed["document"];
        assert_eq!(
            outline_feature(after, authored_id)["center"],
            json!({"x":14.0,"y":9.0})
        );
        let (source_id, mirror_id) = if author == "left" {
            (authored_id, counterpart_id.as_str())
        } else {
            (counterpart_id.as_str(), authored_id)
        };
        let source_center = &outline_feature(after, source_id)["center"];
        let mirror_center = &outline_feature(after, mirror_id)["center"];
        assert_point(
            (
                mirror_center["x"].as_f64().unwrap(),
                mirror_center["y"].as_f64().unwrap(),
            ),
            reflected_point(
                &json!({"at":{"x":200.0,"y":30.0},"rotation":ANGLED_MAP}),
                (
                    source_center["x"].as_f64().unwrap(),
                    source_center["y"].as_f64().unwrap(),
                ),
            ),
            "outline feature forward projection from actual source center",
        );
        assert_eq!(
            pair(after, &pair_id)["halves"]["correspondences"],
            pair(&before, &pair_id)["halves"]["correspondences"],
            "outline edit preserves unrelated pair records"
        );
        let mut repeated_feature = outline_feature(after, authored_id).clone();
        repeated_feature["center"] = json!({"x":14.0,"y":9.0});
        assert_repeated_edit_is_noop(
            &mut engine,
            after,
            &format!("parent19-outline-repeat-{author}"),
            &[authored_id],
            json!({"kind":"apply-outline-edit","edit":{
                "target":{"kind":"board","boardId":board},
                "change":{"set-feature":{"feature":repeated_feature}},
                "seed":42, "reviewed": null}}),
        );
        assert_history_round_trip(&mut engine, &before, after, &format!("outline-{author}"));
    }
}

/// A plain net membership edit from either half maps the pin onto its corresponding Part and
/// leaves every other net, membership, and pair correspondence untouched.
#[test]
fn parent19_mounted_module_placement_commits_from_either_half() {
    for author in ["left", "right"] {
        let pair_id = format!("parent19-mounted-module-{author}");
        let source = with_mounted_module(runtime_supported_document());
        let (mut engine, seed) = open(&source);
        let created = create(
            &mut engine,
            &seed,
            &pair_id,
            "left",
            new_board("right", "right"),
            json!({"at":{"x":200.0,"y":30.0},"rotation":ANGLED_MAP}),
        )
        .unwrap();
        let before = created["document"].clone();
        let authored_id = if author == "left" {
            "left/parent19-module"
        } else {
            "right/parent19-module"
        };
        let counterpart_id = corresponding_id(&before, &pair_id, "module", authored_id);
        let mut authored_module = mounted_module(&before, authored_id).clone();
        authored_module["at"] = if author == "left" {
            json!({"x":43.0,"y":26.0})
        } else {
            json!({"x":177.0,"y":77.0})
        };
        authored_module["rotation"] = json!(41.0);
        let committed = commit(
            &mut engine,
            &before,
            &format!("parent19-mounted-module-edit-{author}"),
            &[authored_id],
            json!({"kind":"set-mounted-module","instance":authored_module}),
        );
        assert_eq!(committed["kind"], "scene", "{committed}");
        let after = &committed["document"];
        assert_eq!(mounted_module(after, authored_id), &authored_module);
        let (source_id, mirror_id) = if author == "left" {
            (authored_id, counterpart_id.as_str())
        } else {
            (counterpart_id.as_str(), authored_id)
        };
        let source_module = mounted_module(after, source_id);
        let mirror_module = mounted_module(after, mirror_id);
        let source_at = &source_module["at"];
        let mirror_at = &mirror_module["at"];
        assert_point(
            (
                mirror_at["x"].as_f64().unwrap(),
                mirror_at["y"].as_f64().unwrap(),
            ),
            reflected_point(
                &json!({"at":{"x":200.0,"y":30.0},"rotation":ANGLED_MAP}),
                (
                    source_at["x"].as_f64().unwrap(),
                    source_at["y"].as_f64().unwrap(),
                ),
            ),
            "mounted module forward projection from actual source position",
        );
        assert_eq!(
            mirror_module["rotation"].as_f64().unwrap(),
            (ANGLED_MAP - source_module["rotation"].as_f64().unwrap() + 180.0).rem_euclid(360.0),
            "mounted module rotation follows the source-to-mirror map"
        );
        let mut normalized_counterpart = mounted_module(after, &counterpart_id).clone();
        let original_counterpart = mounted_module(&before, &counterpart_id);
        normalized_counterpart["at"] = original_counterpart["at"].clone();
        normalized_counterpart["rotation"] = original_counterpart["rotation"].clone();
        assert_eq!(normalized_counterpart, original_counterpart.clone());
        assert_eq!(
            pair(after, &pair_id)["halves"]["correspondences"],
            pair(&before, &pair_id)["halves"]["correspondences"],
            "mounted-module edit must preserve every correspondence"
        );
        assert_eq!(
            after["moduleDefinitions"], before["moduleDefinitions"],
            "placement edits retain the shared module definition"
        );
        assert_repeated_edit_is_noop(
            &mut engine,
            after,
            &format!("parent19-mounted-module-repeat-{author}"),
            &[authored_id],
            json!({"kind":"set-mounted-module","instance":authored_module}),
        );
        assert_history_round_trip(
            &mut engine,
            &before,
            after,
            &format!("mounted-module-{author}"),
        );
    }
}

/// A fixed outline version rename authored from either board changes only the corresponded
/// version name; the second version, outline geometry, and pair records remain byte-equivalent.
#[test]
fn parent19_outline_version_rename_commits_from_either_half() {
    for author in ["left", "right"] {
        let pair_id = format!("parent19-outline-version-{author}");
        let source = with_fixed_outline_versions(runtime_supported_document());
        let (mut engine, seed) = open(&source);
        let created = create(
            &mut engine,
            &seed,
            &pair_id,
            "left",
            new_board("right", "right"),
            json!({"at":{"x":200.0,"y":30.0},"rotation":ANGLED_MAP}),
        )
        .unwrap();
        let before = created["document"].clone();
        let source_version_id = "left/parent19-version-a";
        let authored_id = if author == "left" {
            source_version_id.to_owned()
        } else {
            corresponding_id(&before, &pair_id, "outline-version", source_version_id)
        };
        let authored_board = author;
        let counterpart_id = corresponding_id(&before, &pair_id, "outline-version", &authored_id);
        let counterpart_board = if author == "left" { "right" } else { "left" };
        let unrelated_authored_id = if author == "left" {
            "left/parent19-version-b".to_owned()
        } else {
            corresponding_id(
                &before,
                &pair_id,
                "outline-version",
                "left/parent19-version-b",
            )
        };
        let unrelated_before =
            outline_version(&before, authored_board, &unrelated_authored_id).clone();
        let counterpart_unrelated_id =
            corresponding_id(&before, &pair_id, "outline-version", &unrelated_authored_id);
        let counterpart_unrelated_before =
            outline_version(&before, counterpart_board, &counterpart_unrelated_id).clone();
        let committed = commit(
            &mut engine,
            &before,
            &format!("parent19-rename-outline-version-{author}"),
            &[&authored_id],
            // Ticket 37: raw RenameOutline is deleted; Rename is the live typed rename.
            json!({"kind":"apply-outline-edit","edit":{
                "target":{"kind":"version","boardId":author,"versionId":authored_id},
                "change":{"rename":{"name":format!("Renamed by {author}")}},
                "seed":43, "reviewed": null}}),
        );
        assert_eq!(committed["kind"], "scene", "{committed}");
        let after = &committed["document"];
        assert_eq!(
            outline_version(after, authored_board, &authored_id)["name"],
            format!("Renamed by {author}")
        );
        assert_eq!(
            outline_version(after, counterpart_board, &counterpart_id)["name"],
            format!("Renamed by {author}")
        );
        assert_eq!(
            outline_version(after, authored_board, &unrelated_authored_id),
            &unrelated_before
        );
        assert_eq!(
            outline_version(after, counterpart_board, &counterpart_unrelated_id),
            &counterpart_unrelated_before
        );
        assert_eq!(
            pair(after, &pair_id)["halves"]["correspondences"],
            pair(&before, &pair_id)["halves"]["correspondences"]
        );
        assert_repeated_edit_is_noop(
            &mut engine,
            after,
            &format!("parent19-rename-outline-version-repeat-{author}"),
            &[&authored_id],
            json!({"kind":"apply-outline-edit","edit":{
                "target":{"kind":"version","boardId":author,"versionId":authored_id},
                "change":{"rename":{"name":format!("Renamed by {author}")}},
                "seed":44, "reviewed": null}}),
        );
        assert_history_round_trip(
            &mut engine,
            &before,
            after,
            &format!("outline-version-{author}"),
        );
    }
}
