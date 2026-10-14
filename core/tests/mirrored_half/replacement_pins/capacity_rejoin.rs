//! Capacity and Rejoin acceptance coverage for mirror-only electrical replacements.
use super::*;

fn raw_request(engine: &mut CoreEngine, input: Value) -> Value {
    serde_json::from_str(&engine.request(&input.to_string())).unwrap()
}

fn edit(
    engine: &mut CoreEngine,
    document: &Value,
    transaction: &str,
    phase: &str,
    operation: Value,
) -> Value {
    raw_request(
        engine,
        json!({"id":transaction,"kind":"edit","command":{
            "baseRevision":document["revision"],
            "transactionId":transaction,
            "phase":phase,
            "targetIds":[],
            "operation":operation
        }}),
    )
}

fn exception(pair_id: &str, mirror_id: &str, state: &str) -> Value {
    json!({
        "kind":"set-half-exception",
        "pairId":pair_id,
        "itemKind":"part",
        "mirrorId":mirror_id,
        "state":state
    })
}

fn linked_display_fixture() -> (CoreEngine, Value, String, String) {
    let (_, mut document, ssd_id) = supported_mirror_pair();
    let pair_id = document["hardware"]["pairs"][0]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let spi_definition = catalogue_definition("ceoloide/display_nice_view");
    let spi_definition_id = spi_definition.id.clone();
    document["definitions"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::to_value(spi_definition).unwrap());
    let (mut engine, document) = open(&document);

    let added = edit(
        &mut engine,
        &document,
        "source-spi-add",
        "commit",
        json!({
            "kind":"add-part",
            "boardId":"left",
            "part":{
                "id":"left/NICEVIEW",
                "definitionId":spi_definition_id,
                "reference":"OLED1",
                "pose":{"at":{"x":-42.0,"y":-22.0},"rotation":0.0},
                "side":"front"
            }
        }),
    );
    assert_eq!(added["kind"], "scene", "{added}");
    let document = added["document"].clone();

    let resolved = raw_request(
        &mut engine,
        json!({"id":"resolve-source-electrical","kind":"resolve-electrical","request":{
            "document":document,
            "boardId":"left",
            "instanceId":"left",
            "mode":"matrix",
            "locks":{},
            "controllerPartId":"left/U1",
            "controllerProfile":"ceoloide/mcu_nice_nano"
        }}),
    );
    assert_eq!(resolved["kind"], "electrical-resolved", "{resolved}");
    assert!(
        resolved["plan"]["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .all(|diagnostic| diagnostic["severity"] != "error"),
        "source seed must resolve on the reviewed controller: {resolved}"
    );
    let accepted = snapshot(&mut engine);
    let applied = raw_request(
        &mut engine,
        json!({"id":"apply-source-electrical","kind":"apply-electrical",
            "baseRevision":accepted["revision"],"plan":resolved["plan"],"draft":false}),
    );
    assert_eq!(applied["kind"], "scene", "{applied}");
    let document = applied["document"].clone();
    // A padless mirror-only addition runs the ordinary prepared/reconcile route while the source
    // display is still linked, so the accepted mirror config contains its mapped CS lock before
    // the Omit transition under test.
    let reconciled = edit(
        &mut engine,
        &document,
        "reconcile-linked-electrical",
        "commit",
        add_display_operation("right", "right/MARKER", "synthetic/led"),
    );
    assert_eq!(reconciled["kind"], "scene", "{reconciled}");
    (engine, reconciled["document"].clone(), pair_id, ssd_id)
}

fn board_config(document: &Value, board_id: &str) -> Value {
    document["hardware"]["boards"]
        .as_array()
        .unwrap()
        .iter()
        .find(|board| board["boardId"] == board_id)
        .unwrap()
        .clone()
}

fn source_projection(document: &Value) -> Value {
    json!({
        "parts":document["parts"].as_array().unwrap().iter()
            .filter(|part| part["id"].as_str().unwrap().starts_with("left/")).cloned().collect::<Vec<_>>(),
        "board":board_config(document, "left"),
        "nets":document["nets"].as_array().unwrap().iter()
            .filter(|net| net["id"].as_str().unwrap().starts_with("left/")).cloned().collect::<Vec<_>>()
    })
}

fn without_revision(document: &Value) -> Value {
    let mut document = document.clone();
    document.as_object_mut().unwrap().remove("revision");
    document
}

fn mirror_pin_assignments(document: &Value) -> Value {
    board_config(document, "right")["assignments"].clone()
}

fn mirror_locks(document: &Value) -> Value {
    board_config(document, "right")["locks"].clone()
}

fn assert_capacity_refusal(reply: &Value, context: &str) {
    assert_eq!(reply["kind"], "error", "{context}: {reply}");
    assert!(
        reply["message"]
            .as_str()
            .unwrap()
            .contains("No available GPIO"),
        "{context} should surface the resolver's capacity finding: {reply}"
    );
}

fn add_mirror_spi_operation(id: &str, definition_id: &str) -> Value {
    json!({
        "kind":"add-mirror-only-part",
        "part":{
            "id":id,"definitionId":definition_id,"reference":"OLEDX",
            "pose":{"at":{"x":-45.0,"y":-25.0},"rotation":0.0},"side":"front"
        },
        "boardId":"right","definition":null,"layoutId":null
    })
}

fn omit_source_display(engine: &mut CoreEngine, document: &Value, pair_id: &str) -> Value {
    let omitted = edit(
        engine,
        document,
        "omit-source-spi",
        "commit",
        exception(pair_id, "right/NICEVIEW", "omitted"),
    );
    assert_eq!(omitted["kind"], "scene", "{omitted}");
    omitted["document"].clone()
}

fn assign_mirror_replacement(engine: &mut CoreEngine, document: &Value, ssd_id: &str) -> Value {
    let operation = add_display_operation("right", "right/REPLACEMENT", ssd_id);
    let preview = edit(
        engine,
        document,
        "replacement-preview",
        "preview",
        operation.clone(),
    );
    assert_eq!(preview["kind"], "preview", "{preview}");
    assert_eq!(
        snapshot(engine),
        *document,
        "Preview must not publish placement or pins"
    );
    let committed = edit(engine, document, "replacement-place", "commit", operation);
    assert_eq!(committed["kind"], "scene", "{committed}");
    committed["document"].clone()
}

#[test]
fn omitted_spi_replacement_is_pinned_then_rejoin_reclaims_and_repins_atomically() {
    let (mut engine, document, pair_id, ssd_id) = linked_display_fixture();
    let source_before = source_projection(&document);
    let original_source_assignments = board_config(&document, "left")["assignments"].clone();
    let mirror_before_omit = mirror_pin_assignments(&document);
    let mirror_locks_before_omit = mirror_locks(&document);
    let omitted = omit_source_display(&mut engine, &document, &pair_id);
    assert_eq!(source_projection(&omitted), source_before);
    let omitted_assignments = mirror_pin_assignments(&omitted);
    let omitted_locks = mirror_locks(&omitted);
    let shared_spi_functions = ["spi/MOSI", "spi/SCK"];
    for function in shared_spi_functions {
        assert!(
            omitted_assignments.get(function).is_none(),
            "Omit releases the inactive derived shared bus assignment {function}: {omitted_assignments}"
        );
        assert!(
            omitted_locks.get(function).is_none(),
            "Omit releases the inactive synthesized shared bus lock {function}: {omitted_locks}"
        );
    }
    let mirror_cs = "peripheral/right/NICEVIEW/CS";
    let source_cs = "peripheral/left/NICEVIEW/CS";
    let released_cs_pin = original_source_assignments[source_cs]
        .as_str()
        .or_else(|| mirror_before_omit[mirror_cs].as_str())
        .expect("source-backed SPI display has a per-part CS assignment");
    assert_eq!(mirror_before_omit[mirror_cs], released_cs_pin);
    assert!(
        omitted_assignments.get(mirror_cs).is_none(),
        "Omit drops the per-Part mirror CS assignment, unlike shared MOSI/SCK: {omitted_assignments}"
    );
    assert!(
        omitted_locks.get(mirror_cs).is_none(),
        "Omit releases the per-Part mirror CS lock: {omitted_locks}"
    );
    assert!(
        !omitted_locks
            .as_object()
            .unwrap()
            .values()
            .any(|value| value == released_cs_pin),
        "the released CS pin is not held by a remaining lock: pin={released_cs_pin}, locks={omitted_locks}"
    );
    assert!(mirror_locks_before_omit["spi/MOSI"].as_str().is_some());
    assert!(mirror_locks_before_omit["spi/SCK"].as_str().is_some());

    let placed = assign_mirror_replacement(&mut engine, &omitted, &ssd_id);
    assert_eq!(source_projection(&placed), source_before);
    let placed_board = board_config(&placed, "right");
    let sda = placed_board["assignments"]["i2c/SDA"].as_str().unwrap();
    let scl = placed_board["assignments"]["i2c/SCL"].as_str().unwrap();
    assert_ne!(sda, scl);
    let assigned_replacement_pins = [sda, scl];
    let cs_was_used_by_replacement = assigned_replacement_pins.contains(&released_cs_pin);
    let cs_remains_available = !placed_board["assignments"]
        .as_object()
        .unwrap()
        .values()
        .any(|value| value == released_cs_pin);
    assert!(
        cs_was_used_by_replacement || cs_remains_available,
        "the Omit-released CS pin is either assigned to the replacement or remains allocator-free: cs={released_cs_pin}, assignments={}, locks={}",
        placed_board["assignments"],
        placed_board["locks"]
    );
    for function in shared_spi_functions {
        assert!(placed_board["locks"].get(function).is_none());
        assert!(placed_board["assignments"].get(function).is_none());
    }
    for (function, pin) in omitted_assignments.as_object().unwrap() {
        if !function.starts_with("i2c/") {
            if shared_spi_functions.contains(&function.as_str())
                && placed_board["assignments"].get(function).is_none()
            {
                assert_eq!(placed_board["locks"][function], *pin);
            } else {
                assert_eq!(
                    placed_board["assignments"][function], *pin,
                    "Omit's retained mirror assignment remains stable for {function}"
                );
            }
        }
    }
    for (function, pin) in placed_board["locks"].as_object().unwrap() {
        assert_eq!(
            omitted_locks[function], *pin,
            "placement preserves the mapped lock identity for {function}"
        );
        if let Some(assignment) = placed_board["assignments"].get(function) {
            assert_eq!(
                assignment, pin,
                "a materialized mapped source assignment remains on its locked pin: {function}"
            );
        }
    }
    let pins_before_rejoin = placed_board["assignments"].clone();
    let (_, reopened_before_rejoin) = open(&placed);
    let controller_id = placed_board["controllerPartId"].as_str().unwrap();
    assert_eq!(
        part(&reopened_before_rejoin, controller_id)["generatorParameters"],
        part(&placed, controller_id)["generatorParameters"],
        "Open must retain controller terminal labels derived from the mirror's generated wiring"
    );
    let controller_parameters = &part(&placed, controller_id)["generatorParameters"];
    assert_eq!(
        controller_parameters[sda], "I2C_SDA",
        "{controller_parameters}"
    );
    assert_eq!(
        controller_parameters[scl], "I2C_SCL",
        "{controller_parameters}"
    );
    assert!(
        !controller_parameters
            .as_object()
            .unwrap()
            .values()
            .any(|value| value == "SPI_MOSI" || value == "SPI_SCK")
    );
    let generated_mirror_nets = |document: &Value| {
        document["nets"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|net| {
                net["id"]
                    .as_str()
                    .unwrap()
                    .starts_with("generated/electrical/right/")
            })
            .cloned()
            .collect::<Vec<_>>()
    };
    assert_eq!(
        generated_mirror_nets(&reopened_before_rejoin),
        generated_mirror_nets(&placed),
        "Open must preserve the replacement's materialized generated nets"
    );
    assert_eq!(
        board_config(&reopened_before_rejoin, "right")["assignments"],
        pins_before_rejoin,
        "Open must retain replacement assignments while the source display is still Omitted"
    );
    assert_eq!(source_projection(&reopened_before_rejoin), source_before);

    // A normal prepared source edit also runs the shared reconciliation projection. It must not
    // restore inactive source SPI assignments or lose the mirror-authored I2C wiring.
    let moved = edit(
        &mut engine,
        &placed,
        "ordinary-source-edit-after-replacement",
        "commit",
        json!({"kind":"move-parts","positions":[
            {"id":"left/U1","at":{"x":-22.0,"y":4.0}}
        ]}),
    );
    assert_eq!(moved["kind"], "scene", "{moved}");
    let before_rejoin = moved["document"].clone();
    let source_after_edit = source_projection(&before_rejoin);
    let (_, reopened_after_source_edit) = open(&before_rejoin);
    assert_eq!(
        source_projection(&reopened_after_source_edit),
        source_after_edit
    );
    assert_eq!(
        board_config(&reopened_after_source_edit, "right")["assignments"],
        board_config(&before_rejoin, "right")["assignments"],
        "ordinary source edit followed by Open keeps replacement assignments"
    );
    assert_eq!(
        board_config(&reopened_after_source_edit, "right")["locks"],
        board_config(&before_rejoin, "right")["locks"],
        "ordinary source edit followed by Open keeps lock metadata"
    );
    assert_eq!(
        generated_mirror_nets(&reopened_after_source_edit),
        generated_mirror_nets(&before_rejoin),
        "ordinary source edit followed by Open keeps generated target nets"
    );
    assert_eq!(
        part(&reopened_after_source_edit, controller_id)["generatorParameters"],
        part(&before_rejoin, controller_id)["generatorParameters"],
        "ordinary source edit followed by Open keeps generated controller terminal labels"
    );

    let rejoin_operation = exception(&pair_id, "right/NICEVIEW", "linked");
    let preview = edit(
        &mut engine,
        &before_rejoin,
        "replacement-rejoin-preview",
        "preview",
        rejoin_operation.clone(),
    );
    assert_eq!(preview["kind"], "preview", "{preview}");
    assert_eq!(
        snapshot(&mut engine),
        before_rejoin,
        "Rejoin Preview must not publish"
    );
    let rejoined = edit(
        &mut engine,
        &before_rejoin,
        "replacement-rejoin",
        "commit",
        rejoin_operation,
    );
    assert_eq!(rejoined["kind"], "scene", "{rejoined}");
    let rejoined_document = rejoined["document"].clone();
    assert_eq!(source_projection(&rejoined_document), source_after_edit);
    let assignments = &board_config(&rejoined_document, "right")["assignments"];
    for function in shared_spi_functions {
        if let Some(pin) = original_source_assignments[function].as_str() {
            assert_eq!(assignments[function], pin, "Rejoin restores {function}");
        }
    }
    assert_eq!(
        assignments[mirror_cs], released_cs_pin,
        "Rejoin restores the mapped CS pin"
    );
    for function in ["i2c/SDA", "i2c/SCL"] {
        assert!(
            assignments[function].as_str().is_some(),
            "{function} remains allocated"
        );
        assert_ne!(assignments[function], pins_before_rejoin["spi/MOSI"]);
        assert_ne!(assignments[function], pins_before_rejoin["spi/SCK"]);
    }
    let undo = request(
        &mut engine,
        json!({"id":"replacement-rejoin-undo","kind":"undo"}),
    );
    assert_eq!(undo["kind"], "scene", "{undo}");
    assert_eq!(
        without_revision(&undo["document"]),
        without_revision(&before_rejoin)
    );
    let redo = request(
        &mut engine,
        json!({"id":"replacement-rejoin-redo","kind":"redo"}),
    );
    assert_eq!(redo["kind"], "scene", "{redo}");
    assert_eq!(
        without_revision(&redo["document"]),
        without_revision(&rejoined_document)
    );
    let (_, reopened) = open(&redo["document"]);
    assert_eq!(source_projection(&reopened), source_after_edit);
    assert!(
        reopened["parts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|part| part["id"] == "right/REPLACEMENT")
    );
}

#[test]
fn resolver_capacity_refuses_replacement_placement_and_rejoin_without_publication() {
    let (mut engine, document, pair_id, _ssd_id) = linked_display_fixture();
    let source_before = source_projection(&document);
    let released_source_cs =
        board_config(&document, "left")["assignments"]["peripheral/left/NICEVIEW/CS"]
            .as_str()
            .unwrap()
            .to_owned();
    let omitted = omit_source_display(&mut engine, &document, &pair_id);
    let nice_view = catalogue_definition("ceoloide/display_nice_view");
    let mut accepted = omitted;
    let mut placed_count = 0;
    let mut placement_refusal = None;
    let mut rejoin_capacity_refused = false;
    for index in 0..16 {
        let part_id = format!("right/CAPACITY{index}");
        let operation = add_mirror_spi_operation(&part_id, &nice_view.id);
        let attempt = edit(
            &mut engine,
            &accepted,
            &format!("capacity-fill-{index}"),
            "commit",
            operation,
        );
        if attempt["kind"] == "error" {
            placement_refusal = Some((attempt, accepted.clone()));
            break;
        }
        assert_eq!(attempt["kind"], "scene", "{attempt}");
        accepted = attempt["document"].clone();
        placed_count += 1;
    }
    assert!(
        placed_count > 0,
        "fixture should admit some real profile-backed loads"
    );
    let (placement_refusal, accepted_before_refusal) =
        placement_refusal.expect("reviewed Nice Nano capacity should be exhausted");
    assert_capacity_refusal(&placement_refusal, "placement capacity refusal");
    let placement_preview = edit(
        &mut engine,
        &accepted_before_refusal,
        "replacement-placement-capacity-preview",
        "preview",
        add_mirror_spi_operation("right/CAPACITY-PREVIEW", &nice_view.id),
    );
    assert_capacity_refusal(&placement_preview, "placement capacity Preview");
    assert!(
        placement_preview["message"]
            .as_str()
            .unwrap()
            .starts_with("No available GPIO for peripheral/right/CAPACITY-PREVIEW/CS."),
        "placement Preview identifies the blocked peripheral function: {placement_preview}"
    );
    assert_eq!(snapshot(&mut engine), accepted_before_refusal);
    assert_eq!(source_projection(&accepted_before_refusal), source_before);
    assert!(
        accepted_before_refusal["hardware"]["boards"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|board| board["boardId"] == "right")
            .flat_map(|board| board["assignments"].as_object().unwrap().values())
            .any(|pin| pin.as_str() == Some(&released_source_cs)),
        "catalogue-backed CS loads consume the CS pin released by Omit: {released_source_cs}; mirror assignments={}",
        board_config(&accepted_before_refusal, "right")["assignments"]
    );

    // Use a fresh accepted document for Rejoin exhaustion. Start with the replacement already
    // assigned, then consume real profile pins until the allocator refuses another placement.
    let (mut engine, document, pair_id, ssd_id) = linked_display_fixture();
    let source_before = source_projection(&document);
    let omitted = omit_source_display(&mut engine, &document, &pair_id);
    let replacement = assign_mirror_replacement(&mut engine, &omitted, &ssd_id);
    let mut saturated = replacement;
    for index in 0..16 {
        let part_id = format!("right/REJOINCAPACITY{index}");
        let attempt = edit(
            &mut engine,
            &saturated,
            &format!("rejoin-capacity-fill-{index}"),
            "commit",
            add_mirror_spi_operation(&part_id, &nice_view.id),
        );
        if attempt["kind"] == "error" {
            rejoin_capacity_refused = true;
            assert_eq!(
                snapshot(&mut engine),
                saturated,
                "failed capacity-fill placement is atomic"
            );
            break;
        }
        assert_eq!(attempt["kind"], "scene", "{attempt}");
        saturated = attempt["document"].clone();
    }
    assert!(
        rejoin_capacity_refused,
        "profile-backed catalogue loads should exhaust the reviewed controller"
    );
    let accepted_before_rejoin = saturated.clone();
    let rejoin_preview = edit(
        &mut engine,
        &accepted_before_rejoin,
        "replacement-rejoin-capacity-preview",
        "preview",
        exception(&pair_id, "right/NICEVIEW", "linked"),
    );
    assert_capacity_refusal(&rejoin_preview, "Rejoin capacity Preview");
    assert!(
        rejoin_preview["message"]
            .as_str()
            .unwrap()
            .starts_with("No available GPIO for i2c/SCL."),
        "Rejoin Preview identifies the blocked I2C function: {rejoin_preview}"
    );
    assert_eq!(snapshot(&mut engine), accepted_before_rejoin);
    let refused = edit(
        &mut engine,
        &saturated,
        "replacement-rejoin-without-spare",
        "commit",
        exception(&pair_id, "right/NICEVIEW", "linked"),
    );
    assert_capacity_refusal(&refused, "Rejoin capacity refusal");
    assert_eq!(
        snapshot(&mut engine),
        accepted_before_rejoin,
        "Rejoin refusal is atomic"
    );
    assert_eq!(source_projection(&accepted_before_rejoin), source_before);
}

#[test]
fn inactive_spi_user_lock_metadata_survives_open_without_reserving_gpio_and_rejoin_reclaims_it() {
    let (_, mut document, pair_id, ssd_id) = linked_display_fixture();
    let source_config = document["hardware"]["boards"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|board| board["boardId"] == "left")
        .unwrap();
    let mosi_pin = source_config["assignments"]["spi/MOSI"].clone();
    source_config["locks"]["spi/MOSI"] = mosi_pin.clone();
    let source_before = source_projection(&document);
    let (mut engine, document) = open(&document);

    let omitted = omit_source_display(&mut engine, &document, &pair_id);
    let omitted_board = board_config(&omitted, "right");
    assert!(omitted_board["assignments"].get("spi/MOSI").is_none());
    assert_eq!(
        omitted_board["locks"]["spi/MOSI"], mosi_pin,
        "explicit source lock metadata remains serialized while its role is inactive"
    );
    let placed = assign_mirror_replacement(&mut engine, &omitted, &ssd_id);
    let placed_board = board_config(&placed, "right");
    assert_eq!(placed_board["assignments"]["i2c/SDA"], mosi_pin);
    assert_eq!(placed_board["locks"]["spi/MOSI"], mosi_pin);
    let (_, reopened) = open(&placed);
    let reopened_board = board_config(&reopened, "right");
    assert_eq!(
        reopened_board["assignments"], placed_board["assignments"],
        "an inactive explicit SPI lock does not reserve its pin during Open"
    );
    assert_eq!(reopened_board["locks"]["spi/MOSI"], mosi_pin);
    assert_eq!(source_projection(&reopened), source_before);

    let rejoined = edit(
        &mut engine,
        &placed,
        "locked-source-spi-rejoin",
        "commit",
        exception(&pair_id, "right/NICEVIEW", "linked"),
    );
    assert_eq!(rejoined["kind"], "scene", "{rejoined}");
    let rejoined_doc = &rejoined["document"];
    let rejoined_board = board_config(rejoined_doc, "right");
    assert_eq!(rejoined_board["assignments"]["spi/MOSI"], mosi_pin);
    assert_eq!(rejoined_board["locks"]["spi/MOSI"], mosi_pin);
    assert_ne!(rejoined_board["assignments"]["i2c/SDA"], mosi_pin);
    assert_ne!(rejoined_board["assignments"]["i2c/SCL"], mosi_pin);
    assert_eq!(source_projection(rejoined_doc), source_before);
}

#[test]
fn local_mirror_spi_consumer_keeps_shared_roles_active_when_linked_consumer_is_omitted() {
    let (mut engine, document, pair_id, _) = linked_display_fixture();
    let nice_view_id = catalogue_definition("ceoloide/display_nice_view").id;
    let added = edit(
        &mut engine,
        &document,
        "add-second-source-nice-view",
        "commit",
        json!({
            "kind":"add-part","boardId":"left",
            "part":{"id":"left/NICEVIEW2","definitionId":nice_view_id,
                "reference":"OLED2","pose":{"at":{"x":-55.0,"y":-20.0},"rotation":0.0},
                "side":"front"}
        }),
    );
    assert_eq!(added["kind"], "scene", "{added}");
    let local = edit(
        &mut engine,
        &added["document"],
        "keep-second-nice-view-local",
        "commit",
        exception(&pair_id, "right/NICEVIEW2", "local"),
    );
    assert_eq!(local["kind"], "scene", "{local}");
    let before_omit = local["document"].clone();
    let mapped_mosi = board_config(&before_omit, "right")["assignments"]["spi/MOSI"].clone();
    let omitted = omit_source_display(&mut engine, &before_omit, &pair_id);
    let mirror_board = board_config(&omitted, "right");
    assert_eq!(mirror_board["assignments"]["spi/MOSI"], mapped_mosi);
    assert_eq!(mirror_board["locks"]["spi/MOSI"], mapped_mosi);
    assert_eq!(
        mirror_board["assignments"]["spi/SCK"],
        board_config(&before_omit, "right")["assignments"]["spi/SCK"]
    );
    let (_, reopened) = open(&omitted);
    assert_eq!(
        board_config(&reopened, "right")["assignments"],
        mirror_board["assignments"],
        "the Local mirror consumer keeps the shared SPI assignments active through Open"
    );
    assert_eq!(source_projection(&reopened), source_projection(&omitted));
}

#[test]
fn protected_mirror_replacement_baseline_survives_open_and_requires_review_before_rejoin() {
    let (mut engine, document, pair_id, ssd_id) = linked_display_fixture();
    let source_before = source_projection(&document);
    let original_source_assignments = board_config(&document, "left")["assignments"].clone();
    let omitted = omit_source_display(&mut engine, &document, &pair_id);
    let placed = assign_mirror_replacement(&mut engine, &omitted, &ssd_id);
    let right = board_config(&placed, "right");
    let sda_pin = right["assignments"]["i2c/SDA"].as_str().unwrap();

    let resolved = raw_request(
        &mut engine,
        json!({"id":"resolve-protected-mirror","kind":"resolve-electrical","request":{
            "document":placed,
            "boardId":"right","instanceId":null,"mode":"matrix","locks":{},
            "controllerPartId":"right/U1","controllerProfile":"ceoloide/mcu_nice_nano"
        }}),
    );
    assert_eq!(resolved["kind"], "electrical-resolved", "{resolved}");
    let protected = raw_request(
        &mut engine,
        json!({"id":"protect-right","kind":"protect-electrical-handoff",
            "baseRevision":placed["revision"],"boardId":"right","plan":resolved["plan"]}),
    );
    assert_eq!(protected["kind"], "scene", "{protected}");
    let protected_document = protected["document"].clone();
    let protected_baseline = board_config(&protected_document, "right")["protectedHandoff"].clone();
    assert_eq!(protected_baseline["assignments"]["i2c/SDA"], sda_pin);
    assert_eq!(source_projection(&protected_document), source_before);
    assert_eq!(
        board_config(&protected_document, "left")["protectedHandoff"],
        board_config(&document, "left")["protectedHandoff"],
        "protecting the mirror does not change the source baseline"
    );

    let (mut engine, opened) = open(&protected_document);
    assert_eq!(
        board_config(&opened, "right")["protectedHandoff"],
        protected_baseline,
        "Open must retain the mirror's separate physical handoff baseline"
    );
    assert_eq!(source_projection(&opened), source_before);

    let edited = edit(
        &mut engine,
        &opened,
        "ordinary-source-edit-protected-replacement",
        "commit",
        json!({"kind":"move-parts","positions":[
            {"id":"left/U1","at":{"x":-22.0,"y":4.0}}
        ]}),
    );
    assert_eq!(edited["kind"], "scene", "{edited}");
    let accepted = edited["document"].clone();
    assert_eq!(
        board_config(&accepted, "right")["protectedHandoff"],
        protected_baseline,
        "ordinary source reconciliation retains the mirror baseline"
    );

    let refused = edit(
        &mut engine,
        &accepted,
        "protected-rejoin-refusal",
        "commit",
        exception(&pair_id, "right/NICEVIEW", "linked"),
    );
    assert_eq!(refused["kind"], "error", "{refused}");
    assert_eq!(
        refused["message"],
        "peripheral/right/NICEVIEW/CS conflicts with another assignment to P19"
    );
    assert_eq!(
        snapshot(&mut engine),
        accepted,
        "protected Rejoin refusal is atomic"
    );
    assert_eq!(
        board_config(&snapshot(&mut engine), "right")["protectedHandoff"],
        protected_baseline
    );

    let reviewed = raw_request(
        &mut engine,
        json!({"id":"review-right-remap","kind":"review-electrical-remap",
            "baseRevision":accepted["revision"],"boardId":"right",
            "expectedFingerprint":protected_baseline["fingerprint"]}),
    );
    assert_eq!(reviewed["kind"], "scene", "{reviewed}");
    let reviewed_document = reviewed["document"].clone();
    assert_eq!(
        board_config(&reviewed_document, "right")["protectedHandoff"],
        Value::Null
    );
    let rejoined = edit(
        &mut engine,
        &reviewed_document,
        "reviewed-protected-rejoin",
        "commit",
        exception(&pair_id, "right/NICEVIEW", "linked"),
    );
    assert_eq!(rejoined["kind"], "scene", "{rejoined}");
    let rejoined_document = rejoined["document"].clone();
    let rejoined_board = board_config(&rejoined_document, "right");
    assert_eq!(rejoined_board["protectedHandoff"], Value::Null);
    for function in ["i2c/SDA", "i2c/SCL"] {
        assert!(rejoined_board["assignments"][function].as_str().is_some());
        assert_ne!(
            rejoined_board["assignments"][function],
            original_source_assignments["spi/MOSI"]
        );
        assert_ne!(
            rejoined_board["assignments"][function],
            original_source_assignments["spi/SCK"]
        );
    }
    assert_eq!(
        source_projection(&rejoined_document),
        source_projection(&accepted)
    );
}
