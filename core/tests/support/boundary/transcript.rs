use super::*;

/// Numbers match to a relative 1e-9; everything else exactly.
pub fn equivalent(actual: &Value, expected: &Value, label: &str) {
    match expected {
        Value::Number(want) => {
            let (got, want) = (
                actual
                    .as_f64()
                    .unwrap_or_else(|| panic!("{label}: not a number: {actual}")),
                want.as_f64().unwrap(),
            );
            assert!(
                (got - want).abs() <= 1e-9 * want.abs().max(1.0),
                "{label}: {got} versus {want}"
            );
        }
        Value::Object(want) => {
            let got = actual
                .as_object()
                .unwrap_or_else(|| panic!("{label}: not an object"));
            let (mut left, mut right): (Vec<_>, Vec<_>) =
                (got.keys().collect(), want.keys().collect());
            left.sort();
            right.sort();
            assert_eq!(left, right, "{label} keys");
            for (key, value) in want {
                equivalent(&got[key], value, &format!("{label}.{key}"));
            }
        }
        Value::Array(want) => {
            let got = actual
                .as_array()
                .unwrap_or_else(|| panic!("{label}: not an array"));
            assert_eq!(got.len(), want.len(), "{label} length");
            for (index, value) in want.iter().enumerate() {
                // An entry with an `id` is named by it, so a failure at `core[11]` is findable.
                let entry = match value.get("id").and_then(Value::as_str) {
                    Some(id) => format!("{label}[{index}:{id}]"),
                    None => format!("{label}[{index}]"),
                };
                equivalent(&got[index], value, &entry);
            }
        }
        _ => assert_eq!(actual, expected, "{label}"),
    }
}

fn check_core(replies: &[Value]) {
    // These scenario outcomes must hold before transcript.json compaction or blessing.
    let requests = core_requests();
    assert_eq!(requests.len(), 67, "retain every existing primary scenario");
    assert_eq!(replies.len(), requests.len());
    let mut revision = 0;
    for (request, reply) in requests.iter().zip(replies) {
        let id = request["id"].as_str().unwrap();
        assert_eq!(reply["id"], id);
        let refusal = match id {
            "invalid-draft" => Some("invalid matrix dimensions"),
            "stale-draft" => Some("Stale base revision"),
            "convert-refused" => {
                Some("Converting requires zero Splay, Column rotation and carried Bend")
            }
            "mirrored-half-new-board" | "mirrored-half-reuse-board" => {
                Some("Unknown source physical instance")
            }
            _ => None,
        };
        let kind = if let Some(reason) = refusal {
            assert_eq!(reply["message"], reason, "{id}");
            "error"
        } else {
            match request["kind"].as_str().unwrap() {
                "edit" if request["command"]["phase"] == "preview" => "preview",
                "edit" => {
                    assert_eq!(request["command"]["phase"], "commit");
                    if !matches!(
                        id,
                        "mechanical-closure-repeat"
                            | "outline-corners-repeat"
                            | "outline-rename-repeat"
                    ) {
                        revision += 1;
                    }
                    "scene"
                }
                "undo" | "redo" => {
                    revision += 1;
                    "scene"
                }
                "open" | "snapshot" => "scene",
                "project-matrices" => "matrix-projections",
                other => panic!("declare the intended outcome of {other}: {id}"),
            }
        };
        assert_eq!(reply["kind"], kind, "{id}: {reply}");
        let actual_revision = reply
            .get("scene")
            .map_or(&reply["revision"], |scene| &scene["revision"]);
        assert_eq!(*actual_revision, revision, "{id}: revision/history intent");
    }

    let initial = &replies[0]["document"];
    let committed = &replies[6]["document"];
    for index in [4, 6] {
        assert_eq!(
            replies[index]["scene"]["transforms"]
                .as_array()
                .unwrap()
                .iter()
                .map(|part| &part["id"])
                .collect::<Vec<_>>(),
            initial["parts"]
                .as_array()
                .unwrap()
                .iter()
                .map(|part| &part["id"])
                .collect::<Vec<_>>(),
            "preview/commit retain every visible member identity",
        );
        assert_eq!(
            replies[index]["scene"]["changedIds"],
            json!(["matrix", "matrix/matrix/r0c0"])
        );
        for transform in replies[index]["scene"]["transforms"].as_array().unwrap() {
            let original = initial["parts"]
                .as_array()
                .unwrap()
                .iter()
                .find(|part| part["id"] == transform["id"])
                .unwrap();
            if transform["id"] == "matrix/matrix/r0c0" {
                assert_eq!(transform["pose"]["at"], json!({"x":10.0,"y":12.0}));
                assert_eq!(transform["pose"]["rotation"], original["pose"]["rotation"]);
            } else {
                assert_eq!(
                    transform["pose"], original["pose"],
                    "unselected key stays still"
                );
            }
        }
    }
    assert_eq!(replies[5]["document"], *initial, "preview must not publish");
    assert_eq!(
        committed["matrices"][0]["partIds"],
        initial["matrices"][0]["partIds"]
    );
    let moved = committed["matrices"][0]["cells"]
        .as_array()
        .unwrap()
        .iter()
        .find(|cell| cell["row"] == 0 && cell["column"] == 0)
        .unwrap();
    assert_eq!(moved["offset"], json!({"x":3.0,"y":15.0}));
    for (index, expected) in [(8, initial), (9, committed)] {
        let mut actual = replies[index]["document"].clone();
        actual["revision"] = expected["revision"].clone();
        assert_eq!(actual, *expected, "history restores all authored state");
    }
    let before_origin = &replies[9]["document"];
    let held = &replies[12]["document"];
    for index in [11, 12] {
        let matrix_scene = &replies[index]["scene"]["matrixScenes"][0];
        let original_cells = replies[9]["scene"]["matrixScenes"][0]["cells"]
            .as_array()
            .unwrap();
        let cells = matrix_scene["cells"].as_array().unwrap();
        assert_eq!(cells.len(), original_cells.len());
        for (cell, original) in cells.iter().zip(original_cells) {
            assert_eq!(cell["memberId"], original["memberId"]);
            assert_eq!(cell["enabled"], original["enabled"]);
            for axis in ["x", "y"] {
                assert!(
                    (cell["pose"]["at"][axis].as_f64().unwrap()
                        - original["pose"]["at"][axis].as_f64().unwrap())
                    .abs()
                        < 1e-9,
                    "hold includes logical disabled keys"
                );
            }
            assert_eq!(cell["pose"]["rotation"], original["pose"]["rotation"]);
        }
        assert_eq!(
            matrix_scene["columns"][1]["splayOrigin"],
            json!({"x":5.0,"y":-12.0})
        );
        for transform in replies[index]["scene"]["transforms"].as_array().unwrap() {
            let original = before_origin["parts"]
                .as_array()
                .unwrap()
                .iter()
                .find(|part| part["id"] == transform["id"])
                .unwrap();
            for axis in ["x", "y"] {
                assert!(
                    (transform["pose"]["at"][axis].as_f64().unwrap()
                        - original["pose"]["at"][axis].as_f64().unwrap())
                    .abs()
                        < 1e-9,
                    "origin hold: {} {axis}",
                    transform["id"]
                );
            }
            assert_eq!(transform["pose"]["rotation"], original["pose"]["rotation"]);
        }
    }
    assert_eq!(
        held["matrices"][0]["cells"],
        before_origin["matrices"][0]["cells"]
    );
    let mut allowed = before_origin["matrices"][0].clone();
    for field in ["spread", "stagger", "splayOrigin"] {
        allowed["columnLayout"][1][field] = held["matrices"][0]["columnLayout"][1][field].clone();
    }
    assert_eq!(
        held["matrices"][0], allowed,
        "hold writes only this step and relative origin when Column rotation is zero"
    );
    assert_eq!(
        replies[0]["scene"]["matrixScenes"][0]["cells"]
            .as_array()
            .unwrap()
            .len(),
        9
    );
    assert_eq!(replies[2]["kind"], "matrix-projections");
    assert_eq!(
        replies[4]["scene"]["matrixScenes"],
        replies[6]["scene"]["matrixScenes"]
    );
    assert_eq!(
        replies[8]["scene"]["matrixScenes"],
        replies[0]["scene"]["matrixScenes"]
    );
    assert_eq!(
        replies[9]["scene"]["matrixScenes"],
        replies[6]["scene"]["matrixScenes"]
    );
    let drafts = replies[2]["matrixScenes"].as_array().unwrap();
    assert_eq!(drafts.len(), 6);
    for pair in (0..drafts.len()).step_by(2) {
        for (index, cell) in drafts[pair]["cells"].as_array().unwrap().iter().enumerate() {
            let translated = &drafts[pair + 1]["cells"][index];
            let at = &cell["pose"]["at"];
            equivalent(
                &translated["pose"],
                &json!({ "at": { "x": at["x"].as_f64().unwrap() + 42.0, "y": at["y"].as_f64().unwrap() - 17.0 }, "rotation": cell["pose"]["rotation"] }),
                "draft translation",
            );
            assert!(cell.get("memberId").is_none());
        }
    }
    assert_eq!(
        replies[10]["matrixScenes"][0]["cells"]
            .as_array()
            .unwrap()
            .len(),
        500
    );
    assert_eq!(replies[11]["kind"], "preview");
    assert_eq!(replies[12]["kind"], "scene");
    assert_eq!(
        replies[11]["scene"]["matrixScenes"],
        replies[12]["scene"]["matrixScenes"]
    );
    assert_eq!(replies[3]["kind"], "error");
    assert_eq!(replies[4]["kind"], "preview");
    assert_eq!(
        replies[1]["scene"]["matrixScenes"],
        replies[5]["scene"]["matrixScenes"]
    );
    assert_eq!(replies[7]["kind"], "error");
    assert_eq!(replies[7]["message"], "Stale base revision");
    assert_eq!(
        replies[13]["document"]["hardware"]["boards"][0]["keyBindings"]["matrix/matrix/r0c0"],
        "&kp A"
    );
    assert_eq!(
        replies[14]["document"]["keycaps"]["matrices"]["matrix"]["wallThickness"].as_f64(),
        Some(1.2)
    );
    assert_eq!(
        replies[15]["document"]["keycaps"]["boards"]["board"]["color"],
        "#123456"
    );
    assert_eq!(
        replies[16]["document"]["keycaps"]["keys"]["matrix/matrix/r0c0"]["legend"],
        ""
    );
    let companions = replies[26]["scene"]["matrixScenes"][0]["companions"]
        .as_array()
        .unwrap();
    assert_eq!(companions.len(), 3);
    assert!(companions.iter().any(|item| item["mode"] == "follows"));
    assert!(companions.iter().any(|item| item["mode"] == "authored"));
    assert!(companions.contains(&json!({
        "keyId": "matrix/matrix/r0c0",
        "mode": { "constrained": { "constraintId": "sensor-key", "sourcePartId": "matrix/matrix/r0c0" } },
        "partId": "matrix/matrix/r0c0/sensor"
    })));

    // Guard scenario intent before compaction or blessing can hide a refusal cascade.
    for (id, kind, revision) in [
        ("convert-offset-1", "scene", 43),
        ("convert-preview", "preview", 43),
        ("convert-commit", "scene", 44),
        ("convert-offset-2", "scene", 45),
    ] {
        let reply = &replies[position(replies, id)];
        assert_eq!(reply["kind"], kind, "{id}: {reply}");
        assert_eq!(reply["scene"]["revision"], revision, "{id}: {reply}");
    }
    let converted = &replies[position(replies, "convert-commit")]["document"]["matrices"][0];
    assert_eq!(converted["columnLayout"][0]["stagger"], 3.0);
    assert!(converted["columnLayout"][0].get("offset").is_none());
    assert_eq!(converted["columnLayout"][1]["stagger"], -1.0);
    for (id, message) in [
        (
            "convert-refused",
            "Converting requires zero Splay, Column rotation and carried Bend",
        ),
        (
            "mirrored-half-new-board",
            "Unknown source physical instance",
        ),
        (
            "mirrored-half-reuse-board",
            "Unknown source physical instance",
        ),
    ] {
        let reply = &replies[position(replies, id)];
        assert_eq!(reply["kind"], "error", "{id}: {reply}");
        assert_eq!(reply["message"], message, "{id}: {reply}");
        assert_eq!(reply["revision"], 45, "{id}: {reply}");
    }

    // The mechanical edit reports its configuration owner, the added closure
    // mounts and the generated clearance it projects, then a no-op keeps the
    // revision and one Undo removes the complete result.
    let mechanical = position(replies, "mechanical-enable");
    assert_eq!(replies[mechanical]["kind"], "scene");
    assert_eq!(
        replies[mechanical]["scene"]["changedIds"],
        json!(["board"]),
        "enabling reports the configuration owner"
    );
    assert_eq!(replies[mechanical + 1]["kind"], "preview");
    let changed = replies[mechanical + 2]["scene"]["changedIds"]
        .as_array()
        .unwrap();
    assert!(changed.iter().any(|id| id.as_str() == Some("board")));
    for mount in ["auto-closure/left-front", "auto-closure/left-back"] {
        assert!(
            changed.iter().any(|id| id.as_str() == Some(mount)),
            "changed ids name {mount}"
        );
    }
    let count_generated = |reply: &Value, collection: &str, prefix: &str| -> usize {
        reply["document"][collection]
            .as_array()
            .unwrap()
            .iter()
            .filter(|entry| {
                entry["id"]
                    .as_str()
                    .is_some_and(|id| id.starts_with(prefix))
            })
            .count()
    };
    assert_eq!(
        count_generated(&replies[mechanical + 2], "parts", "case-closure/"),
        2
    );
    assert_eq!(
        count_generated(
            &replies[mechanical + 2],
            "definitions",
            "assembly-closure/definition/"
        ),
        2
    );
    for entry in replies[mechanical + 2]["document"]["parts"]
        .as_array()
        .unwrap()
    {
        let id = entry["id"].as_str().unwrap();
        if id.starts_with("case-closure/") {
            assert!(
                changed
                    .iter()
                    .any(|candidate| candidate.as_str() == Some(id)),
                "generated {id} is reported"
            );
        }
    }
    assert_eq!(
        replies[mechanical + 3]["scene"]["changedIds"],
        json!([]),
        "an identical commit changes nothing"
    );
    assert_eq!(
        replies[mechanical + 3]["scene"]["revision"],
        replies[mechanical + 2]["scene"]["revision"],
        "a no-op edit adds no revision step"
    );
    assert_eq!(
        replies[mechanical + 4]["document"]["mechanical"]["closureMounts"],
        json!([]),
        "one Undo restores the closure mounts the edit added"
    );
    assert_eq!(
        count_generated(&replies[mechanical + 4], "parts", "case-closure/"),
        0
    );
}

/// The typed outline block's semantic assertions: generated/changed identities,
/// generated contours and the persisted document shape, verified before the golden
/// transcript is blessed.
fn check_outline(replies: &[Value]) {
    let by_id = |id: &str| {
        replies
            .iter()
            .find(|reply| reply["id"] == id)
            .unwrap_or_else(|| panic!("reply {id}"))
    };
    let changed = |id: &str| by_id(id)["scene"]["changedIds"].clone();
    let revision = |id: &str| by_id(id)["scene"]["revision"].clone();
    let gaps = |contours: &Value| contours.as_array().unwrap().len();
    let settings = |id: &str, feature_id: &str| -> Value {
        by_id(id)["document"]["outline"]
            .as_array()
            .unwrap()
            .iter()
            .find(|feature| feature["id"] == feature_id)
            .unwrap_or_else(|| panic!("feature {feature_id} in {id}"))["settings"]
            .clone()
    };
    let version = |id: &str, version_id: &str| -> Value {
        by_id(id)["document"]["boardOutlines"]
            .as_array()
            .unwrap()
            .iter()
            .find(|state| state["boardId"] == "board")
            .unwrap()["versions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|version| version["id"] == version_id)
            .unwrap_or_else(|| panic!("version {version_id} in {id}"))
            .clone()
    };
    let version_ids = |id: &str| -> Vec<String> {
        by_id(id)["document"]["boardOutlines"][0]["versions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|version| version["id"].as_str().unwrap().to_owned())
            .collect()
    };
    let board_contours = |id: &str| -> Vec<usize> {
        by_id(id)["scene"]["boardContours"][0]["contours"]
            .as_array()
            .unwrap()
            .iter()
            .map(|contour| contour["points"].as_array().unwrap().len())
            .collect()
    };

    // Generate authors the envelope and its membership in one edit, and the envelope
    // resolves against the board's own component.
    assert_eq!(
        changed("outline-generate"),
        json!(["board", GENERATED]),
        "Generate reports the board and the envelope identity Core derived from the seed"
    );
    let envelope = &by_id("outline-generate")["document"]["outline"][0];
    assert_eq!(envelope["kind"], "part-envelope");
    assert_eq!(envelope["partIds"], json!(["outline-part"]));
    assert_eq!(envelope["margin"], json!(4.0));
    assert_eq!(envelope["settings"]["corners"], "fillet");
    assert_eq!(
        by_id("outline-generate")["document"]["boards"][0]["outlineIds"],
        json!([GENERATED]),
        "one edit registers the envelope on the board"
    );
    assert_eq!(
        gaps(&by_id("outline-generate")["scene"]["boardContours"][0]["contours"]),
        1
    );
    assert_eq!(
        board_contours("outline-generate"),
        vec![20],
        "a filleted envelope"
    );

    // Field edits change the envelope; an identical edit adds no revision step.
    assert_eq!(changed("outline-corners"), json!(["board", GENERATED]));
    assert_eq!(settings("outline-corners", GENERATED)["corners"], "sharp");
    assert_eq!(board_contours("outline-corners"), vec![4], "sharp corners");
    assert_eq!(changed("outline-corners-repeat"), json!([]));
    assert_eq!(
        revision("outline-corners-repeat"),
        revision("outline-corners"),
        "an equal field edit adds no revision step"
    );
    assert_eq!(changed("outline-margin"), json!(["board", GENERATED]));
    assert_eq!(settings("outline-margin", GENERATED)["corners"], "sharp");
    assert_eq!(
        by_id("outline-margin")["document"]["outline"][0]["margin"],
        json!(6.0)
    );
    assert_eq!(
        settings("outline-bridge", GENERATED)["bridgeWidth"],
        json!(3.0)
    );

    // An authored connection and the protected gap report their own identities.
    assert_eq!(
        changed("outline-connection-add"),
        json!(["board", GENERATED, "conn-a"])
    );
    let feature = &by_id("outline-connection-add")["document"]["outline"][0];
    assert_eq!(feature["connections"].as_array().unwrap().len(), 1);
    assert_eq!(feature["connections"][0]["id"], "conn-a");
    assert_eq!(feature["connections"][0]["width"], json!(3.0));
    assert_eq!(
        changed("outline-connection-remove"),
        json!(["board", GENERATED, "conn-a"])
    );
    assert!(
        by_id("outline-connection-remove")["document"]["outline"][0]
            .get("connections")
            .is_none(),
        "the removed connection is gone"
    );
    assert_eq!(
        changed("outline-gap-set"),
        json!(["board", GENERATED, "gap-a"])
    );
    assert_eq!(
        by_id("outline-gap-set")["document"]["outline"][0]["settings"]["repair"]["keepGaps"][0]["id"],
        "gap-a"
    );
    assert_eq!(
        gaps(&by_id("outline-gap-set")["scene"]["boardContours"][0]["contours"]),
        2,
        "a protected gap opens a second region in the resolved boundary"
    );
    assert_eq!(
        changed("outline-gap-remove"),
        json!(["board", GENERATED, "gap-a"])
    );
    assert!(
        by_id("outline-gap-remove")["document"]["outline"][0]["settings"]["repair"]
            .get("keepGaps")
            .is_none(),
        "the removed gap is gone"
    );
    assert_eq!(board_contours("outline-gap-remove"), vec![4]);

    // The reviewed perimeter edit creates one fixed copy, selected, whose contour is
    // the submitted shape; the version carries its own bridge width.
    assert_eq!(changed("outline-perimeter"), json!(["board", EDITED]));
    let edited = version("outline-perimeter", EDITED);
    assert_eq!(edited["name"], "Edited outline 1");
    assert!(
        edited.get("ownership").is_none(),
        "a perimeter edit is a fixed copy"
    );
    assert_eq!(edited["geometry"]["features"].as_array().unwrap().len(), 1);
    assert_eq!(edited["geometry"]["features"][0]["kind"], "polygon");
    assert_eq!(
        edited["geometry"]["features"][0]["points"],
        json!([{ "x": -12.0, "y": -12.0 }, { "x": 12.0, "y": -12.0 }, { "x": 12.0, "y": 12.0 }, { "x": -12.0, "y": 12.0 }])
    );
    assert_eq!(
        edited["geometry"]["features"][0]["id"],
        "board:board:version:outline-version-12:contour:0"
    );
    assert_eq!(changed("outline-version-bridge"), json!(["board", EDITED]));
    assert_eq!(
        version("outline-version-bridge", EDITED)["geometry"]["settings"]["bridgeWidth"],
        json!(1.5),
        "a version keeps its own settings"
    );
    assert_eq!(
        settings("outline-version-bridge", GENERATED)["bridgeWidth"],
        json!(3.0),
        "the generated envelope keeps its own bridge width"
    );

    // An authored feature is added to and removed from the version.
    assert_eq!(
        changed("outline-version-feature-add"),
        json!(["board", "cut-a", EDITED])
    );
    assert_eq!(
        version("outline-version-feature-add", EDITED)["geometry"]["features"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        version("outline-version-feature-add", EDITED)["geometry"]["features"][1]["operation"],
        "subtract"
    );
    assert_eq!(
        changed("outline-version-feature-remove"),
        json!(["board", "cut-a", EDITED])
    );
    assert_eq!(
        version("outline-version-feature-remove", EDITED)["geometry"]["features"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(changed("outline-version-rename"), json!(["board", EDITED]));
    assert_eq!(version("outline-version-rename", EDITED)["name"], "Renamed");

    // Activation returns the board to Generated, then a linked refinement follows the
    // board's current component with its own settings.
    assert_eq!(changed("outline-activate-generated"), json!(["board"]));
    assert!(
        by_id("outline-activate-generated")["document"]["boardOutlines"][0]
            .get("activeVersionId")
            .is_none(),
        "Generated is active again"
    );
    assert_eq!(changed("outline-refine"), json!(["board", REFINED]));
    let refined = version("outline-refine", REFINED);
    assert_eq!(refined["ownership"], "linked");
    assert_eq!(refined["geometry"]["features"][0]["kind"], "part-envelope");
    assert_eq!(
        refined["geometry"]["features"][0]["partIds"],
        json!(["outline-part"]),
        "the refinement resolves from the board's current components"
    );
    assert_eq!(
        by_id("outline-refine")["document"]["boardOutlines"][0]["activeVersionId"],
        REFINED
    );
    assert_eq!(changed("outline-refine-margin"), json!(["board", REFINED]));
    assert_eq!(
        version("outline-refine-margin", REFINED)["geometry"]["features"][0]["margin"],
        json!(7.0),
        "the refinement owns its margin"
    );
    assert_eq!(
        by_id("outline-refine-margin")["document"]["outline"][0]["margin"],
        json!(6.0),
        "the generated envelope's margin is unchanged"
    );

    // Freeze keeps the link and adds a fixed copy from the resolved projection.
    assert_eq!(
        changed("outline-freeze"),
        json!(["board", "outline-version-19"])
    );
    assert_eq!(version("outline-freeze", REFINED)["ownership"], "linked");
    let frozen = version("outline-freeze", "outline-version-19");
    assert_eq!(frozen["name"], "Frozen");
    assert!(
        frozen.get("ownership").is_none(),
        "freeze writes a fixed copy"
    );
    assert_eq!(frozen["source"]["versionId"], REFINED);
    assert_eq!(
        by_id("outline-freeze")["document"]["boardOutlines"][0]["activeVersionId"],
        "outline-version-19"
    );
    assert_eq!(
        changed("outline-activate-refined"),
        json!(["board", REFINED])
    );

    // Editing a linked refinement's perimeter retains the link and adds a fixed copy.
    assert_eq!(
        changed("outline-refine-perimeter"),
        json!(["board", "outline-version-21"])
    );
    assert_eq!(version_ids("outline-refine-perimeter").len(), 4);
    assert_eq!(
        version("outline-refine-perimeter", REFINED)["ownership"],
        "linked"
    );
    assert_eq!(
        version("outline-refine-perimeter", "outline-version-21")["name"],
        "Edited outline 1"
    );
    assert_eq!(
        version("outline-refine-perimeter", "outline-version-21")["geometry"]["features"][0]["points"],
        json!([{ "x": -13.0, "y": -13.0 }, { "x": 13.0, "y": -13.0 }, { "x": 13.0, "y": 13.0 }, { "x": -13.0, "y": 13.0 }])
    );

    // A copy records its origin and stays fixed; deleting records the retired
    // identity, and one Undo restores the version and the selection together.
    assert_eq!(changed("outline-copy"), json!(["board", "v-copy"]));
    assert_eq!(
        version("outline-copy", "v-copy")["source"]["versionId"],
        "outline-version-21",
        "the copy records the version that was active when it was taken"
    );
    assert!(version("outline-copy", "v-copy").get("ownership").is_none());
    assert_eq!(changed("outline-delete"), json!(["board", "v-copy"]));
    assert_eq!(
        by_id("outline-delete")["document"]["boardOutlines"][0]["retiredVersionIds"],
        json!(["v-copy"])
    );
    assert!(
        by_id("outline-delete")["document"]["boardOutlines"][0]
            .get("activeVersionId")
            .is_none()
    );
    assert_eq!(
        version_ids("outline-undo"),
        vec![
            EDITED.to_owned(),
            REFINED.to_owned(),
            "outline-version-19".to_owned(),
            "outline-version-21".to_owned(),
            "v-copy".to_owned(),
        ]
    );
    assert!(
        by_id("outline-undo")["document"]["boardOutlines"][0]
            .get("retiredVersionIds")
            .is_none(),
        "one Undo clears the reservation the delete recorded"
    );
    assert_eq!(
        by_id("outline-undo")["document"]["boardOutlines"][0]["activeVersionId"],
        "v-copy"
    );
    assert_eq!(changed("outline-rename-repeat"), json!([]));
    assert_eq!(
        revision("outline-rename-repeat"),
        revision("outline-undo"),
        "a rename to the accepted name adds no revision step"
    );

    // The final document shape: the generated envelope, the version order and each
    // version's persisted ownership and name.
    let snapshot = &by_id("outline-snapshot")["document"];
    assert_eq!(snapshot["boards"][0]["outlineIds"], json!([GENERATED]));
    assert_eq!(snapshot["boards"][0]["partIds"], json!(["outline-part"]));
    assert_eq!(version_ids("outline-snapshot").len(), 5);
    let names: Vec<&str> = snapshot["boardOutlines"][0]["versions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|version| version["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        vec!["Renamed", "Refined", "Frozen", "Edited outline 1", "Copy"]
    );
    let ownership: Vec<Option<&str>> = snapshot["boardOutlines"][0]["versions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|version| version.get("ownership").and_then(Value::as_str))
        .collect();
    assert_eq!(
        ownership,
        vec![None, Some("linked"), None, None, None],
        "only the refinement persists a link"
    );
    assert_eq!(snapshot["boardOutlines"][0]["activeVersionId"], "v-copy");
}

fn compact_reply(reply: &mut Value, full_body: bool) {
    if full_body {
        return;
    }
    let body = serde_json::to_vec(reply).expect("reply serializes");
    let keys = |name| {
        reply[name].as_object().map(|object| {
            let mut keys = object.keys().cloned().collect::<Vec<_>>();
            keys.sort();
            keys
        })
    };
    let mut compact = json!({
        "kind": reply["kind"],
        "bodyDigest": digest(&body),
    });
    for field in ["id", "message", "revision"] {
        if let Some(value) = reply.get(field) {
            compact[field] = value.clone();
        }
    }
    if let Some(keys) = keys("document") {
        compact["documentKeys"] = json!(keys);
    }
    if let Some(keys) = keys("scene") {
        compact["sceneKeys"] = json!(keys);
    }
    *reply = compact;
}

fn compact_replies(replies: &mut [Value]) {
    for reply in replies {
        let anchor = matches!(
            reply["id"].as_str(),
            Some("open" | "commit" | "invalid-draft" | "draft")
        );
        compact_reply(reply, anchor);
    }
}

fn compact_archive(transcript: &mut [Value]) {
    let mut kept_unpack = false;
    for entry in transcript {
        let is_unpack = entry["kind"] == "unpack-project" && entry["reply"]["kind"] == "unpacked";
        if is_unpack && !kept_unpack {
            kept_unpack = true;
            continue;
        }
        let reply = &mut entry["reply"];
        let body = serde_json::to_vec(reply).expect("archive reply serializes");
        let mut compact = json!({ "kind": reply["kind"], "bodyDigest": digest(&body) });
        if let Some(message) = reply.get("message") {
            compact["message"] = message.clone();
        }
        *reply = compact;
    }
}

/// Runs every request and archive case through the transports, asserting the
/// boundary's behaviour, and returns the transcript recorded as the golden.
pub fn run(core: CoreTransport, archive: ArchiveTransport) -> Value {
    let mut replies: Vec<Value> = core_requests()
        .iter()
        .map(|request| {
            serde_json::from_str(&core(&request.to_string())).expect("core reply is JSON")
        })
        .collect();
    check_core(&replies);
    check_outline(&replies);
    compact_replies(&mut replies);

    let derived_mirror = derived_mirror_case(&mut *core, &mut *archive);

    let mut transcript = Vec::new();
    let mut call = |request: Value, buffers: &[Vec<u8>]| -> (Value, Vec<Vec<u8>>) {
        let (reply, outputs) = archive(&request, buffers);
        transcript.push(json!({ "kind": request["kind"], "reply": reply, "buffers": outputs.iter().map(|bytes| digest(bytes)).collect::<Vec<_>>() }));
        (reply, outputs)
    };

    let mut project = document();
    project["assets"] = json!([{ "id": "asset-id", "name": "component.step", "mediaType": "model/step", "sha256": ASSET_SHA256 }]);
    project["futureField"] = json!({ "preserve": true });
    let project_json = project.to_string();
    let asset_path = format!("assets/{ASSET_SHA256}");
    let asset = ASSET.to_vec();

    let (reply, packed) = call(
        json!({ "kind": "pack-project", "projectJson": project_json, "archiveJson": json!({ "embedUsedModels": true }).to_string(), "assets": [{ "path": asset_path, "bufferIndex": 0 }] }),
        std::slice::from_ref(&asset),
    );
    assert_eq!(reply["kind"], "packed");
    let (reply, unpacked) = call(json!({ "kind": "unpack-project" }), &packed);
    assert_eq!(reply["projectJson"], project_json);
    assert_eq!(unpacked[0], asset);

    // Archives from another producer: an old project without archive.json,
    // wrong asset bytes, and a truncated archive.
    let (_, old) = call(
        json!({ "kind": "pack-files", "entries": [{ "path": "project.json", "bufferIndex": 0 }, { "path": asset_path, "bufferIndex": 1 }] }),
        &[project_json.clone().into_bytes(), asset.clone()],
    );
    let (reply, _) = call(json!({ "kind": "unpack-project" }), &old);
    assert_eq!(reply["projectJson"], project_json);
    let (_, wrong) = call(
        json!({ "kind": "pack-files", "entries": [{ "path": "project.json", "bufferIndex": 0 }, { "path": asset_path, "bufferIndex": 1 }] }),
        &[project_json.clone().into_bytes(), b"wrong".to_vec()],
    );
    assert_eq!(
        call(json!({ "kind": "unpack-project" }), &wrong).0["kind"],
        "error"
    );
    let truncated = vec![old[0][..old[0].len() - 12].to_vec()];
    assert_eq!(
        call(json!({ "kind": "unpack-project" }), &truncated).0["kind"],
        "error"
    );

    let (reply, generic) = call(
        json!({ "kind": "pack-files", "entries": [{ "path": "BoardStudio.pretty/part.kicad_mod", "bufferIndex": 0 }, { "path": "models/component.step", "bufferIndex": 1 }] }),
        &[b"(footprint \"part\")".to_vec(), asset.clone()],
    );
    assert_eq!(reply["kind"], "packed");
    assert_eq!(generic.len(), 1);
    let duplicate = call(
        json!({ "kind": "pack-files", "entries": [{ "path": "same", "bufferIndex": 0 }, { "path": "same", "bufferIndex": 1 }] }),
        &[asset.clone(), asset.clone()],
    );
    assert_eq!(duplicate.0["kind"], "error");

    let payload = payload();
    let mut doc = document();
    doc["assets"] = json!([{ "id": "memory-fixture", "sha256": PAYLOAD_SHA256 }]);
    let large = json!({ "kind": "pack-project", "projectJson": doc.to_string(), "assets": [{ "path": format!("assets/{PAYLOAD_SHA256}"), "bufferIndex": 0 }] });
    let (reply, measured) = call(large, &[payload]);
    assert_eq!(reply["kind"], "packed");
    assert_eq!(
        call(json!({ "kind": "unpack-project" }), &measured).0["kind"],
        "unpacked"
    );

    compact_archive(&mut transcript);

    json!({
        "core": replies,
        "archive": transcript,
        "derivedMirror": derived_mirror
    })
}

pub fn assert_golden(actual: &Value, golden: &Value) {
    equivalent(actual, golden, "transcript");
}
