//! Archive-route Open repair and stale-record acceptance for parent ticket 19.
use super::*;

const PAIR_ID: &str = "parent19-archive-open-pair";

fn archive_round_trip(document: &Value) -> Value {
    let (packed_reply, packed_buffers) = boardstudio_core::archive::request(
        &json!({
            "kind":"pack-project",
            "projectJson":document.to_string(),
            "assets":[]
        })
        .to_string(),
        &[],
    );
    let packed: Value = serde_json::from_str(&packed_reply).unwrap();
    assert_eq!(packed["kind"], "packed", "{packed}");
    assert_eq!(packed_buffers.len(), 1, "project Save returns one archive");

    let (unpacked_reply, asset_buffers) =
        boardstudio_core::archive::request(r#"{"kind":"unpack-project"}"#, &packed_buffers);
    let unpacked: Value = serde_json::from_str(&unpacked_reply).unwrap();
    assert_eq!(unpacked["kind"], "unpacked", "{unpacked}");
    assert!(asset_buffers.is_empty(), "fixture has no embedded assets");
    serde_json::from_str(unpacked["projectJson"].as_str().unwrap()).unwrap()
}

fn open_with_findings(document: &Value) -> (CoreEngine, Value, Value) {
    let mut engine = CoreEngine::new();
    let reply = request(
        &mut engine,
        json!({"id":"parent19-archive-open","kind":"open","document":document}),
    );
    assert_eq!(reply["kind"], "scene", "{reply}");
    let document = reply["document"].clone();
    let findings = reply["scene"]["findings"].clone();
    let undo: Value = serde_json::from_str(
        &engine.request(&json!({"id":"parent19-open-history-check","kind":"undo"}).to_string()),
    )
    .unwrap();
    assert_eq!(
        undo["kind"], "error",
        "Open repair must not create history: {undo}"
    );
    assert_eq!(undo["message"], "History is empty");
    assert_eq!(
        snapshot(&mut engine),
        document,
        "history check leaves Open state intact"
    );
    (engine, document, findings)
}

fn commit(engine: &mut CoreEngine, document: &Value, transaction: &str, operation: Value) -> Value {
    request(
        engine,
        json!({"id":transaction,"kind":"edit","command":{
            "baseRevision":document["revision"],
            "transactionId":transaction,
            "phase":"commit",
            "targetIds":[],
            "operation":operation
        }}),
    )
}

/// The actual project archive path must preserve the source and report archive-authored drift on
/// the first Open, then leave an already repaired project stable and warning-free.
#[test]
fn parent19_archive_open_repairs_linked_geometry_once_and_preserves_source() {
    let (mut engine, seed) = seed(&runtime_supported_document(), "left");
    let created = create(
        &mut engine,
        &seed,
        PAIR_ID,
        "left",
        new_board("right", "right"),
        mapping(200.0),
    )
    .unwrap();
    let mut hand_edited = created["document"].clone();
    let source_part = part(&hand_edited, "left/U1").clone();
    let mirror = hand_edited["parts"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|part| part["id"] == "right/U1")
        .unwrap();
    mirror["pose"]["at"]["x"] = json!(231.0);

    let archived_input = archive_round_trip(&hand_edited);
    assert_eq!(
        archived_input, hand_edited,
        "archive retains the edited input"
    );
    let (_opened_engine, repaired, findings) = open_with_findings(&archived_input);
    assert_eq!(part(&repaired, "right/U1")["pose"]["at"]["x"], 224.0);
    assert_eq!(part(&repaired, "left/U1"), &source_part);
    let warning_id = format!("mirrored-half:open-repaired:{PAIR_ID}");
    let warning = findings
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["id"] == warning_id)
        .unwrap_or_else(|| panic!("located repair warning {warning_id} is missing: {findings}"));
    assert_eq!(warning["severity"], "warning");
    assert!(
        warning["targetIds"]
            .as_array()
            .unwrap()
            .contains(&json!("right/U1"))
    );

    let second_archive_input = archive_round_trip(&repaired);
    assert_eq!(second_archive_input, repaired);
    let (_second_engine, reopened, second_findings) = open_with_findings(&second_archive_input);
    assert_eq!(reopened, repaired, "second archive Open is idempotent");
    assert!(
        second_findings
            .as_array()
            .unwrap()
            .iter()
            .all(|finding| finding["id"] != warning_id),
        "stable Open must not report a repair warning again: {second_findings}"
    );
}

/// Removing one linked Part cleans its record while preserving the unrelated pair records;
/// a subsequent public mechanical edit still commits through the normal Core route.
#[test]
fn parent19_removed_item_records_do_not_poison_a_later_mechanical_edit() {
    let (mut engine, seed) = seed(&runtime_supported_document(), "left");
    let created = create(
        &mut engine,
        &seed,
        PAIR_ID,
        "left",
        new_board("right", "right"),
        mapping(200.0),
    )
    .unwrap();
    let linked = created["document"].clone();
    let unrelated_correspondences: Vec<_> = pair(&linked, PAIR_ID)["halves"]["correspondences"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|record| record["sourceId"] != "left/RST" && record["mirrorId"] != "right/RST")
        .cloned()
        .collect();

    let removed = commit(
        &mut engine,
        &linked,
        "parent19-remove-linked-rst",
        json!({"kind":"remove-parts","ids":["left/RST"]}),
    );
    assert_eq!(removed["kind"], "scene", "{removed}");
    let after_removal = removed["document"].clone();
    for id in ["left/RST", "right/RST"] {
        assert!(
            after_removal["parts"]
                .as_array()
                .unwrap()
                .iter()
                .all(|part| part["id"] != id),
            "removed linked part {id} must be absent"
        );
    }
    let records = pair(&after_removal, PAIR_ID)["halves"]["correspondences"]
        .as_array()
        .unwrap();
    assert!(
        records.iter().all(|record| {
            record["sourceId"] != "left/RST" && record["mirrorId"] != "right/RST"
        })
    );
    assert_eq!(records, &unrelated_correspondences);
    assert!(
        pair(&after_removal, PAIR_ID)["halves"]["exceptions"]
            .as_array()
            .is_none_or(|exceptions| exceptions
                .iter()
                .all(|record| record["mirrorId"] != "right/RST"))
    );

    let moved = commit(
        &mut engine,
        &after_removal,
        "parent19-move-after-record-cleanup",
        json!({"kind":"move-parts","positions":[{
            "id":"left/U1","at":{"x":-17.0,"y":14.0}
        }]}),
    );
    assert_eq!(
        moved["kind"], "scene",
        "valid mechanical edit after cleanup: {moved}"
    );
    let edited = &moved["document"];
    assert_eq!(
        part(edited, "left/U1")["pose"]["at"],
        json!({"x":-17.0,"y":14.0})
    );
    assert_eq!(
        part(edited, "right/U1")["pose"]["at"],
        json!({"x":217.0,"y":14.0})
    );
    assert_eq!(
        pair(edited, PAIR_ID)["halves"]["correspondences"],
        pair(&after_removal, PAIR_ID)["halves"]["correspondences"],
        "the later edit retains all remaining correspondence records"
    );
}

/// ReplaceDocument prunes a stale correspondence and exception after their Parts disappear,
/// preserving unrelated records; a later accepted edit proves the pair remains usable.
#[test]
fn parent19_replace_document_cleans_stale_exception_and_correspondence_records() {
    let (mut engine, seed) = seed(&runtime_supported_document(), "left");
    let created = create(
        &mut engine,
        &seed,
        PAIR_ID,
        "left",
        new_board("right", "right"),
        mapping(200.0),
    )
    .unwrap();
    let linked = created["document"].clone();
    let local = commit(
        &mut engine,
        &linked,
        "parent19-replace-local-rst",
        json!({
            "kind":"set-half-exception","pairId":PAIR_ID,"itemKind":"part",
            "mirrorId":"right/RST","state":"local"
        }),
    );
    assert_eq!(local["kind"], "scene", "{local}");
    let before_replace = local["document"].clone();
    let retained_records: Vec<_> = pair(&before_replace, PAIR_ID)["halves"]["correspondences"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|record| record["sourceId"] != "left/RST" && record["mirrorId"] != "right/RST")
        .cloned()
        .collect();

    // Model a prepared project replacement that removed this independent Part but carried
    // forward the old association metadata. Core must reconcile the final replacement draft.
    let mut replacement = before_replace.clone();
    replacement["parts"]
        .as_array_mut()
        .unwrap()
        .retain(|part| part["id"] != "left/RST" && part["id"] != "right/RST");
    for board in replacement["boards"].as_array_mut().unwrap() {
        board["partIds"]
            .as_array_mut()
            .unwrap()
            .retain(|id| id != "left/RST" && id != "right/RST");
    }
    for outline in replacement["outline"].as_array_mut().unwrap() {
        if let Some(part_ids) = outline["partIds"].as_array_mut() {
            part_ids.retain(|id| id != "left/RST" && id != "right/RST");
        }
    }
    let replaced = commit(
        &mut engine,
        &before_replace,
        "parent19-replace-document-remove-rst",
        json!({"kind":"replace-document","document":replacement}),
    );
    assert_eq!(
        replaced["kind"], "scene",
        "ReplaceDocument cleanup: {replaced}"
    );
    let after_replace = replaced["document"].clone();
    assert!(find_part(&after_replace, "left/RST").is_none());
    assert!(find_part(&after_replace, "right/RST").is_none());
    assert_eq!(
        pair(&after_replace, PAIR_ID)["halves"]["correspondences"],
        json!(retained_records),
        "only the removed Part correspondence is retired"
    );
    assert!(
        pair(&after_replace, PAIR_ID)["halves"]["exceptions"]
            .as_array()
            .is_none_or(|exceptions| exceptions
                .iter()
                .all(|record| record["mirrorId"] != "right/RST")),
        "the removed Part's exception is retired"
    );

    let moved = commit(
        &mut engine,
        &after_replace,
        "parent19-replace-followup-move",
        json!({"kind":"move-parts","positions":[{
            "id":"left/U1","at":{"x":-29.0,"y":12.0}
        }]}),
    );
    assert_eq!(
        moved["kind"], "scene",
        "follow-up edit after ReplaceDocument: {moved}"
    );
    assert_eq!(
        part(&moved["document"], "left/U1")["pose"]["at"],
        json!({"x":-29.0,"y":12.0})
    );
    assert_eq!(
        part(&moved["document"], "right/U1")["pose"]["at"],
        json!({"x":229.0,"y":12.0})
    );
    assert_eq!(
        pair(&moved["document"], PAIR_ID)["halves"]["correspondences"],
        pair(&after_replace, PAIR_ID)["halves"]["correspondences"]
    );
}

#[test]
fn parent19_replace_document_keeps_local_mirror_when_source_is_removed() {
    let (mut engine, seed) = seed(&runtime_supported_document(), "left");
    let created = create(
        &mut engine,
        &seed,
        PAIR_ID,
        "left",
        new_board("right", "right"),
        mapping(200.0),
    )
    .unwrap();
    let linked = created["document"].clone();
    let local = commit(
        &mut engine,
        &linked,
        "parent19-replace-local-survivor-rst",
        json!({
            "kind":"set-half-exception","pairId":PAIR_ID,"itemKind":"part",
            "mirrorId":"right/RST","state":"local"
        }),
    );
    assert_eq!(local["kind"], "scene", "{local}");
    let before_replace = local["document"].clone();
    let local_target = part(&before_replace, "right/RST").clone();
    let mut replacement = before_replace.clone();
    replacement["parts"]
        .as_array_mut()
        .unwrap()
        .retain(|part| part["id"] != "left/RST");
    for board in replacement["boards"].as_array_mut().unwrap() {
        if board["id"] == "left" {
            board["partIds"]
                .as_array_mut()
                .unwrap()
                .retain(|id| id != "left/RST");
        }
    }
    for outline in replacement["outline"].as_array_mut().unwrap() {
        if let Some(part_ids) = outline["partIds"].as_array_mut() {
            part_ids.retain(|id| id != "left/RST");
        }
    }

    let replaced = commit(
        &mut engine,
        &before_replace,
        "parent19-replace-document-remove-local-source-rst",
        json!({"kind":"replace-document","document":replacement}),
    );
    assert_eq!(
        replaced["kind"], "scene",
        "ReplaceDocument cleanup: {replaced}"
    );
    let after_replace = &replaced["document"];
    assert!(find_part(after_replace, "left/RST").is_none());
    assert_eq!(part(after_replace, "right/RST"), &local_target);
    assert!(
        pair(after_replace, PAIR_ID)["halves"]["correspondences"]
            .as_array()
            .unwrap()
            .iter()
            .all(|record| record["sourceId"] != "left/RST")
    );
    assert!(
        pair(after_replace, PAIR_ID)["halves"]["exceptions"]
            .as_array()
            .is_none_or(|exceptions| exceptions
                .iter()
                .all(|record| record["mirrorId"] != "right/RST"))
    );

    let moved = commit(
        &mut engine,
        after_replace,
        "parent19-replace-local-survivor-followup-move",
        json!({"kind":"move-parts","positions":[{
            "id":"left/U1","at":{"x":-31.0,"y":9.0}
        }]}),
    );
    assert_eq!(moved["kind"], "scene", "follow-up edit: {moved}");
    assert_eq!(part(&moved["document"], "right/RST"), &local_target);
    assert_eq!(
        part(&moved["document"], "right/U1")["pose"]["at"],
        json!({"x":231.0,"y":9.0})
    );
}

fn rename_to_other_left(value: &mut Value, preserve_half_label: bool) {
    match value {
        Value::String(text) => {
            if preserve_half_label && text == "left" {
                return;
            }
            *text = if text == "left" {
                "other-left".into()
            } else if text == "left-synthetic-keys-layout" {
                "other-left-synthetic-keys-layout".into()
            } else if text == "left-synthetic-keys" {
                "other-left-synthetic-keys".into()
            } else if let Some(suffix) = text.strip_prefix("matrix/left-synthetic-keys/") {
                format!("matrix/other-left-synthetic-keys/{suffix}")
            } else if let Some(suffix) = text.strip_prefix("left-") {
                format!("other-left-{suffix}")
            } else if let Some(suffix) = text.strip_prefix("left/") {
                format!("other-left/{suffix}")
            } else if let Some(suffix) = text.strip_prefix("synthetic/column/") {
                format!("other-synthetic/column/{suffix}")
            } else if text == "synthetic-boundary" {
                "other-left/synthetic-boundary".into()
            } else if text.contains("Synthetic left") {
                text.replace("Synthetic left", "Synthetic other-left")
            } else {
                text.clone()
            };
        }
        Value::Array(items) => {
            for item in items {
                rename_to_other_left(item, preserve_half_label);
            }
        }
        Value::Object(object) => {
            let old = std::mem::take(object);
            for (mut key, mut item) in old {
                let is_half = key == "half";
                let mut key_value = Value::String(key.clone());
                rename_to_other_left(&mut key_value, false);
                key = key_value.as_str().unwrap().to_owned();
                rename_to_other_left(&mut item, preserve_half_label || is_half);
                object.insert(key, item);
            }
        }
        _ => {}
    }
}

fn duplicate_source_half(document: &mut Value) {
    let clone_matching = |items: &[Value], predicate: &dyn Fn(&Value) -> bool| {
        items
            .iter()
            .filter(|item| predicate(item))
            .cloned()
            .collect::<Vec<_>>()
    };
    for field in ["parts", "matrices", "layouts", "nets", "outline"] {
        let Some(items) = document[field].as_array().cloned() else {
            continue;
        };
        let clones = match field {
            "parts" => clone_matching(&items, &|item| {
                item["id"].as_str().is_some_and(|id| {
                    id.starts_with("left/") || id.starts_with("matrix/left-synthetic-keys/")
                })
            }),
            "matrices" => clone_matching(&items, &|item| item["id"] == "left-synthetic-keys"),
            "layouts" => clone_matching(&items, &|item| item["id"] == "left-synthetic-keys-layout"),
            "nets" => clone_matching(&items, &|item| {
                item["id"]
                    .as_str()
                    .is_some_and(|id| id.starts_with("synthetic/column/"))
            }),
            "outline" => clone_matching(&items, &|item| {
                item["id"] == "synthetic-boundary" || item["id"] == "left/synthetic-envelope"
            }),
            _ => unreachable!(),
        };
        for mut item in clones {
            rename_to_other_left(&mut item, false);
            document[field].as_array_mut().unwrap().push(item);
        }
    }
    let board = document["boards"]
        .as_array()
        .unwrap()
        .iter()
        .find(|board| board["id"] == "left")
        .unwrap()
        .clone();
    let mut board = board;
    rename_to_other_left(&mut board, false);
    document["boards"].as_array_mut().unwrap().push(board);
    let hardware = document["hardware"].as_object_mut().unwrap();
    for field in ["instances", "boards"] {
        let original = hardware[field]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| {
                item[if field == "instances" {
                    "id"
                } else {
                    "boardId"
                }] == "left"
            })
            .unwrap()
            .clone();
        let mut duplicate = original;
        rename_to_other_left(&mut duplicate, false);
        hardware[field].as_array_mut().unwrap().push(duplicate);
    }
}

fn commit_named(engine: &mut CoreEngine, before: &Value, id: &str, operation: Value) -> Value {
    request(
        engine,
        json!({"id":id,"kind":"edit","command":{
            "baseRevision":before["revision"],"transactionId":id,"phase":"commit",
            "targetIds":[],"operation":operation
        }}),
    )
}

#[test]
fn parent19_replace_document_retires_rows_for_two_pairs_in_one_batch() {
    let mut input = runtime_supported_document();
    duplicate_source_half(&mut input);
    let (mut engine, seed) = open(&input);
    let first = create(
        &mut engine,
        &seed,
        "parent19-replace-two-pair-a",
        "left",
        new_board("right", "right"),
        mapping(200.0),
    )
    .unwrap();
    let second = commit_named(
        &mut engine,
        &first["document"],
        "parent19-replace-two-pair-b",
        operation(
            "parent19-replace-two-pair-b",
            "other-left",
            new_board("other-right", "other-right"),
            mapping(600.0),
        ),
    );
    assert_eq!(second["kind"], "scene", "second independent pair: {second}");
    let paired = second["document"].clone();
    let instance_ids: BTreeSet<_> = paired["hardware"]["instances"]
        .as_array()
        .unwrap()
        .iter()
        .map(|instance| instance["id"].as_str().unwrap())
        .collect();
    assert_eq!(
        instance_ids,
        BTreeSet::from(["left", "right", "other-left", "other-right"]),
        "the two mirror pairs use four distinct physical instances"
    );
    assert_eq!(
        pair(&paired, "parent19-replace-two-pair-a")["members"],
        json!(["left", "right"])
    );
    assert_eq!(
        pair(&paired, "parent19-replace-two-pair-b")["members"],
        json!(["other-left", "other-right"])
    );
    let local = commit(
        &mut engine,
        &paired,
        "parent19-replace-two-pair-local-target",
        json!({"kind":"set-half-exception","pairId":"parent19-replace-two-pair-a",
            "itemKind":"part","mirrorId":"right/RST","state":"local"}),
    );
    assert_eq!(local["kind"], "scene", "Local target setup: {local}");
    let before_replace = local["document"].clone();
    let local_target = part(&before_replace, "right/RST").clone();
    let mut replacement = before_replace.clone();
    // Pair A loses a non-Part Layout endpoint while pair B loses an independent Part. The
    // carried Layout row proves this batches lifecycle retirement across pairs and item kinds.
    replacement["layouts"]
        .as_array_mut()
        .unwrap()
        .retain(|layout| {
            layout["id"] != "left-synthetic-keys-layout"
                && layout["id"] != "right-synthetic-keys-layout"
        });
    let removed = ["left/RST", "other-left/RST", "other-right/RST"];
    replacement["parts"]
        .as_array_mut()
        .unwrap()
        .retain(|candidate| !removed.iter().any(|id| candidate["id"] == *id));
    for board in replacement["boards"].as_array_mut().unwrap() {
        board["partIds"]
            .as_array_mut()
            .unwrap()
            .retain(|id| !removed.iter().any(|removed_id| id == *removed_id));
    }
    for outline in replacement["outline"].as_array_mut().unwrap() {
        if let Some(ids) = outline["partIds"].as_array_mut() {
            ids.retain(|id| !removed.iter().any(|removed_id| id == *removed_id));
        }
    }
    let replaced = commit(
        &mut engine,
        &before_replace,
        "parent19-replace-document-two-pair-rst",
        json!({"kind":"replace-document","document":replacement}),
    );
    assert_eq!(
        replaced["kind"], "scene",
        "two-pair ReplaceDocument: {replaced}"
    );
    let after = &replaced["document"];
    assert!(find_part(after, "left/RST").is_none());
    assert!(find_part(after, "other-left/RST").is_none());
    assert!(find_part(after, "other-right/RST").is_none());
    assert_eq!(part(after, "right/RST"), &local_target);
    for pair_id in ["parent19-replace-two-pair-a", "parent19-replace-two-pair-b"] {
        let halves = pair(after, pair_id)["halves"].clone();
        assert!(
            halves["correspondences"]
                .as_array()
                .unwrap()
                .iter()
                .all(|row| {
                    row["sourceId"] != "left/RST" && row["sourceId"] != "other-left/RST"
                })
        );
        assert!(halves["exceptions"].as_array().is_none_or(|rows| {
            rows.iter()
                .all(|row| row["mirrorId"] != "right/RST" && row["mirrorId"] != "other-right/RST")
        }));
    }
    assert!(
        pair(after, "parent19-replace-two-pair-a")["halves"]["correspondences"]
            .as_array()
            .unwrap()
            .iter()
            .all(|row| row["sourceId"] != "left-synthetic-keys-layout")
    );
    assert!(
        pair(after, "parent19-replace-two-pair-b")["halves"]["correspondences"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["sourceId"] == "other-left-synthetic-keys-layout")
    );
    let moved = commit(
        &mut engine,
        after,
        "parent19-replace-two-pair-followup",
        json!({"kind":"move-parts","positions":[
            {"id":"left/U1","at":{"x":-21.0,"y":8.0}},
            {"id":"other-left/U1","at":{"x":-25.0,"y":11.0}}
        ]}),
    );
    assert_eq!(
        moved["kind"], "scene",
        "both pairs remain editable: {moved}"
    );
    assert_eq!(part(&moved["document"], "right/RST"), &local_target);
}
