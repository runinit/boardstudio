use super::*;

/// Exercises automatic pin assignment through the exact Core edit and archive
/// transports shared by native tests and the browser/WASM transcript replay.
pub fn electrical_mirror_replacement_case(
    core: CoreTransport<'_>,
    archive: ArchiveTransport<'_>,
) -> Value {
    let (mut accepted, pair_id, _nice_view_id, ssd1306_id) =
        replacement_pair_fixture(&mut *core, "boundary-electrical-replacement");
    let pair_before = accepted["hardware"]["pairs"][0].clone();
    let source_before = replacement_source_projection(&accepted);
    let source_assignments = replacement_board_config(&accepted, "left")["assignments"].clone();
    let mirror_config_before = replacement_board_config(&accepted, "right");
    let freed_cs = mirror_config_before["assignments"]["peripheral/right/NICEVIEW/CS"]
        .as_str()
        .expect("the source-linked Nice View has a mapped mirror CS pin")
        .to_owned();
    for function in ["spi/MOSI", "spi/SCK"] {
        assert_eq!(
            mirror_config_before["assignments"][function], source_assignments[function],
            "mapped bus function {function} starts on the source pin"
        );
    }

    let omitted = replacement_core_call(
        &mut *core,
        replacement_edit(
            "boundary-electrical-replacement-omit",
            &accepted["revision"],
            "commit",
            boundary_exception(&pair_id, "right/NICEVIEW", "omitted"),
        ),
    );
    assert_eq!(omitted["kind"], "scene", "{omitted}");
    accepted = omitted["document"].clone();
    let pair_after_omit = accepted["hardware"]["pairs"][0].clone();
    let omitted_config = replacement_board_config(&accepted, "right");
    assert_ne!(
        omitted_config["assignments"]["peripheral/right/NICEVIEW/CS"], freed_cs,
        "Omit releases the source-linked display's mirror CS function"
    );
    for function in ["spi/MOSI", "spi/SCK"] {
        assert!(
            omitted_config["assignments"][function].is_null(),
            "the last omitted SPI consumer releases inactive bus assignment {function}"
        );
        assert!(
            omitted_config["locks"][function].is_null(),
            "the last omitted SPI consumer releases derived inactive bus lock {function}"
        );
    }
    assert_eq!(replacement_source_projection(&accepted), source_before);

    let mirror_id = "right/BOUNDARY-OLED";
    let operation = mirror_only_display_operation("right", mirror_id, &ssd1306_id);
    let preview = replacement_core_call(
        &mut *core,
        replacement_edit(
            "boundary-electrical-replacement-preview",
            &accepted["revision"],
            "preview",
            operation.clone(),
        ),
    );
    assert_eq!(preview["kind"], "preview", "{preview}");
    assert_eq!(
        replacement_core_call(
            &mut *core,
            json!({"id":"boundary-electrical-replacement-after-preview",
        "kind":"snapshot"})
        )["document"],
        accepted,
        "placement Preview must not publish Part or electrical assignments"
    );
    let placed = replacement_core_call(
        &mut *core,
        replacement_edit(
            "boundary-electrical-replacement-place",
            &accepted["revision"],
            "commit",
            operation,
        ),
    );
    assert_eq!(placed["kind"], "scene", "{placed}");
    accepted = placed["document"].clone();
    assert_eq!(replacement_source_projection(&accepted), source_before);
    assert_eq!(
        accepted["hardware"]["pairs"][0], pair_after_omit,
        "explicit placement creates no pair provenance"
    );
    assert_eq!(
        accepted["parts"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|part| part["id"] == mirror_id)
            .count(),
        1
    );
    let placed_config = replacement_board_config(&accepted, "right");
    let sda = placed_config["assignments"]["i2c/SDA"]
        .as_str()
        .expect("the mirror SSD1306 receives SDA");
    let scl = placed_config["assignments"]["i2c/SCL"]
        .as_str()
        .expect("the mirror SSD1306 receives SCL");
    assert_ne!(sda, scl, "SDA and SCL use distinct pins");
    assert!(
        [sda, scl].contains(&freed_cs.as_str()),
        "the pin released from per-Part CS is a valid I2C allocation"
    );
    assert_eq!(
        placed_config["locks"], omitted_config["locks"],
        "placement preserves the surviving mapped/user lock set"
    );
    for function in ["spi/MOSI", "spi/SCK"] {
        assert!(
            placed_config["locks"][function].is_null(),
            "inactive derived bus lock {function} remains absent without an explicit source lock"
        );
        assert!(
            placed_config["assignments"][function].is_null(),
            "inactive shared bus function {function} is omitted from materialized assignments"
        );
    }

    let mirror_nets_before_open = replacement_generated_electrical_nets(&accepted, "right");
    let controller_parameters_before_open = replacement_generator_parameters(&accepted, "right/U1");
    let (packed_omitted, packed_omitted_buffers) = archive(
        &json!({"kind":"pack-project","projectJson":accepted.to_string(),
            "archiveJson":json!({"embedUsedModels":false}).to_string(),"assets":[]}),
        &[],
    );
    assert_eq!(packed_omitted["kind"], "packed", "{packed_omitted}");
    let (unpacked_omitted, unpacked_omitted_buffers) =
        archive(&json!({"kind":"unpack-project"}), &packed_omitted_buffers);
    assert_eq!(unpacked_omitted["kind"], "unpacked", "{unpacked_omitted}");
    assert!(unpacked_omitted_buffers.is_empty());
    let omitted_archive: Value =
        serde_json::from_str(unpacked_omitted["projectJson"].as_str().unwrap())
            .expect("omitted replacement archive project JSON");
    let opened_omitted = replacement_core_call(
        &mut *core,
        json!({"id":"boundary-electrical-replacement-open-before-rejoin",
            "kind":"open","document":omitted_archive}),
    );
    assert_eq!(opened_omitted["kind"], "scene", "{opened_omitted}");
    let opened_omitted_doc = opened_omitted["document"].clone();
    let opened_omitted_config = replacement_board_config(&opened_omitted_doc, "right");
    assert_eq!(
        opened_omitted_config["assignments"], placed_config["assignments"],
        "Open before Rejoin retains replacement and inactive-role assignments"
    );
    assert_eq!(
        opened_omitted_config["locks"], placed_config["locks"],
        "Open before Rejoin retains effective and serialized lock state"
    );
    assert_eq!(
        replacement_generated_electrical_nets(&opened_omitted_doc, "right"),
        mirror_nets_before_open,
        "Open before Rejoin retains target-authored generated electrical nets"
    );
    assert_eq!(
        replacement_generator_parameters(&opened_omitted_doc, "right/U1"),
        controller_parameters_before_open,
        "Open before Rejoin retains controller terminal parameters derived from target nets"
    );
    assert_eq!(
        replacement_source_projection(&opened_omitted_doc),
        source_before,
        "Open before Rejoin leaves source electrical projection unchanged"
    );
    assert_eq!(
        opened_omitted_doc["hardware"]["pairs"][0], pair_after_omit,
        "Open before Rejoin preserves Omit lifecycle state"
    );
    accepted = opened_omitted_doc;

    let rejoin_operation = boundary_exception(&pair_id, "right/NICEVIEW", "linked");
    let rejoin_preview = replacement_core_call(
        &mut *core,
        replacement_edit(
            "boundary-electrical-replacement-rejoin-preview",
            &accepted["revision"],
            "preview",
            rejoin_operation.clone(),
        ),
    );
    assert_eq!(rejoin_preview["kind"], "preview", "{rejoin_preview}");
    assert_eq!(
        replacement_core_call(
            &mut *core,
            json!({"id":"boundary-electrical-replacement-after-rejoin-preview",
        "kind":"snapshot"})
        )["document"],
        accepted,
        "Rejoin Preview must not publish assignments"
    );
    let rejoined = replacement_core_call(
        &mut *core,
        replacement_edit(
            "boundary-electrical-replacement-rejoin",
            &accepted["revision"],
            "commit",
            rejoin_operation,
        ),
    );
    assert_eq!(rejoined["kind"], "scene", "{rejoined}");
    let rejoined_doc = rejoined["document"].clone();
    assert_eq!(replacement_source_projection(&rejoined_doc), source_before);
    assert_eq!(rejoined_doc["hardware"]["pairs"][0], pair_before);
    let rejoined_config = replacement_board_config(&rejoined_doc, "right");
    assert_eq!(
        rejoined_config["assignments"]["peripheral/right/NICEVIEW/CS"],
        source_assignments["peripheral/left/NICEVIEW/CS"],
        "Rejoin restores the source-linked Part's source CS pin"
    );
    for function in ["spi/MOSI", "spi/SCK"] {
        assert_eq!(
            rejoined_config["assignments"][function], source_assignments[function],
            "Rejoin keeps mapped bus function {function} on the source pin"
        );
    }
    for function in ["i2c/SDA", "i2c/SCL"] {
        let pin = rejoined_config["assignments"][function]
            .as_str()
            .unwrap_or_else(|| panic!("Rejoin retains assignment for {function}"));
        assert_ne!(
            pin, freed_cs,
            "the replacement is re-pinned after Rejoin reclaims its former pin"
        );
    }
    let i2c_net_prefix = "generated/electrical/right/i2c/";
    assert!(
        rejoined_doc["nets"]
            .as_array()
            .unwrap()
            .iter()
            .any(|net| net["id"]
                .as_str()
                .is_some_and(|id| id.starts_with(i2c_net_prefix))
                && net["pins"]
                    .as_array()
                    .is_some_and(|pins| pins.iter().any(|pin| pin["partId"] == mirror_id))),
        "the replacement's generated mirror I2C net remains materialized"
    );

    let undo = replacement_core_call(
        &mut *core,
        json!({"id":"boundary-electrical-replacement-undo","kind":"undo"}),
    );
    assert_eq!(undo["kind"], "scene", "{undo}");
    assert_eq!(
        replacement_board_config(&undo["document"], "right")["assignments"],
        placed_config["assignments"],
        "one Undo restores pre-Rejoin pins"
    );
    let redo = replacement_core_call(
        &mut *core,
        json!({"id":"boundary-electrical-replacement-redo","kind":"redo"}),
    );
    assert_eq!(redo["kind"], "scene", "{redo}");
    assert_eq!(
        replacement_board_config(&redo["document"], "right")["assignments"],
        rejoined_config["assignments"],
        "one Redo restores re-pinned assignments"
    );

    let (packed, packed_buffers) = archive(
        &json!({"kind":"pack-project","projectJson":redo["document"].to_string(),
            "archiveJson":json!({"embedUsedModels":false}).to_string(),"assets":[]}),
        &[],
    );
    assert_eq!(packed["kind"], "packed", "{packed}");
    let (unpacked, unpacked_buffers) = archive(&json!({"kind":"unpack-project"}), &packed_buffers);
    assert_eq!(unpacked["kind"], "unpacked", "{unpacked}");
    assert!(unpacked_buffers.is_empty());
    let archived: Value = serde_json::from_str(unpacked["projectJson"].as_str().unwrap())
        .expect("archive project JSON");
    assert_eq!(
        archived, redo["document"],
        "archive roundtrip preserves all assignment state"
    );
    let reopened = replacement_core_call(
        &mut *core,
        json!({"id":"boundary-electrical-replacement-archive-open","kind":"open",
            "document":archived}),
    );
    assert_eq!(reopened["kind"], "scene", "{reopened}");
    assert!(
        reopened["document"]["parts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|part| part["id"] == mirror_id)
    );
    assert_eq!(
        replacement_board_config(&reopened["document"], "right")["assignments"],
        rejoined_config["assignments"],
        "Save/Open retains replacement re-pin"
    );
    assert_eq!(
        replacement_source_projection(&reopened["document"]),
        source_before
    );

    // A fresh fixture admits real catalogued loads until the same reviewed
    // Nice Nano profile is genuinely exhausted. Then Rejoin must refuse without
    // publishing the lifecycle change or any partial electrical plan.
    let (mut capacity_doc, capacity_pair_id, capacity_nice_view_id, capacity_ssd_id) =
        replacement_pair_fixture(&mut *core, "boundary-electrical-capacity");
    let capacity_source_before = replacement_source_projection(&capacity_doc);
    let capacity_omit = replacement_core_call(
        &mut *core,
        replacement_edit(
            "boundary-electrical-capacity-omit",
            &capacity_doc["revision"],
            "commit",
            boundary_exception(&capacity_pair_id, "right/NICEVIEW", "omitted"),
        ),
    );
    assert_eq!(capacity_omit["kind"], "scene", "{capacity_omit}");
    capacity_doc = capacity_omit["document"].clone();
    let capacity_place = replacement_core_call(
        &mut *core,
        replacement_edit(
            "boundary-electrical-capacity-place",
            &capacity_doc["revision"],
            "commit",
            mirror_only_display_operation("right", "right/CAPACITY-OLED", &capacity_ssd_id),
        ),
    );
    assert_eq!(capacity_place["kind"], "scene", "{capacity_place}");
    capacity_doc = capacity_place["document"].clone();
    let mut capacity_fill_refused = false;
    for index in 0..16 {
        let id = format!("right/BOUNDARY-CAPACITY{index}");
        let attempt = replacement_core_call(
            &mut *core,
            replacement_edit(
                &format!("boundary-electrical-capacity-fill-{index}"),
                &capacity_doc["revision"],
                "commit",
                mirror_only_display_operation("right", &id, &capacity_nice_view_id),
            ),
        );
        if attempt["kind"] == "error" {
            assert!(
                attempt["message"]
                    .as_str()
                    .is_some_and(|message| message.starts_with("No available GPIO for ")),
                "filling the reviewed profile refuses from pin capacity: {attempt}"
            );
            assert_eq!(
                replacement_core_call(
                    &mut *core,
                    json!({"id":"boundary-electrical-capacity-after-fill-refusal","kind":"snapshot"})
                )["document"],
                capacity_doc,
                "failed filler placement leaves accepted state untouched"
            );
            capacity_fill_refused = true;
            break;
        }
        assert_eq!(attempt["kind"], "scene", "{attempt}");
        capacity_doc = attempt["document"].clone();
    }
    assert!(
        capacity_fill_refused,
        "real Nice Nano GPIO capacity is exhausted by catalogued peripheral functions"
    );
    assert_eq!(
        replacement_source_projection(&capacity_doc),
        capacity_source_before
    );
    let rejoin_refusal = replacement_core_call(
        &mut *core,
        replacement_edit(
            "boundary-electrical-capacity-rejoin",
            &capacity_doc["revision"],
            "commit",
            boundary_exception(&capacity_pair_id, "right/NICEVIEW", "linked"),
        ),
    );
    assert_eq!(rejoin_refusal["kind"], "error", "{rejoin_refusal}");
    assert!(
        rejoin_refusal["message"]
            .as_str()
            .is_some_and(|message| message.starts_with("No available GPIO for ")),
        "capacity Rejoin returns the resolver pin finding: {rejoin_refusal}"
    );
    assert_eq!(
        replacement_core_call(
            &mut *core,
            json!({"id":"boundary-electrical-capacity-after-rejoin-refusal","kind":"snapshot"})
        )["document"],
        capacity_doc,
        "failed Rejoin publishes no lifecycle/electrical changes"
    );
    assert_eq!(
        replacement_source_projection(&capacity_doc),
        capacity_source_before
    );

    json!({
        "supportedController":"ceoloide/mcu_nice_nano",
        "omitReleasesPerPartCs":true,
        "mirrorOnlyPlacementAutomaticallyPinned":true,
        "freedCsPinWasAllocatedToReplacement":true,
        "rejoinRestoresSourcePinsAndRepinsReplacement":true,
        "sourceUnchanged":true,
        "previewNonpublishing":true,
        "singleStepUndoRedo":true,
        "archiveSaveOpenPreservesAssignments":true,
        "omittedReplacementArchiveOpenStable":true,
        "capacityPlacementRefusalAtomic":true,
        "capacityRejoinRefusalAtomic":true
    })
}

/// Proves ticket 160's exact Rejoin pin/net/registration loss through the same
/// JSON Core transport used by the native golden and the browser/WASM replay.
pub(super) fn paired_electrical_preview_case(mut core: CoreTransport<'_>) -> Value {
    let call = |core: &mut CoreTransport<'_>, request: Value| {
        serde_json::from_str::<Value>(&core(&request.to_string())).expect("core reply is JSON")
    };
    let mut source = supported_synthetic_half();
    source["definitions"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|definition| definition["id"] == "synthetic/controller")
        .unwrap()["pads"] = json!([{
        "id":"pad-1","number":"1","at":{"x":0.0,"y":0.0},
        "size":{"x":1.0,"y":1.0},"shape":"circle"
    }]);
    source["nets"].as_array_mut().unwrap().push(json!({
        "id":"left/omit-controller-net","name":"OMIT_CONTROLLER",
        "pins":[{"partId":"left/RST","padId":"pad-1"}]
    }));
    source["boards"][0]["netIds"]
        .as_array_mut()
        .unwrap()
        .push(json!("left/omit-controller-net"));

    let open = call(
        &mut core,
        json!({"id":"electrical-open","kind":"open","document":source}),
    );
    assert_eq!(open["kind"], "scene", "{open}");
    let created = call(
        &mut core,
        json!({
            "id":"electrical-create",
            "kind":"edit",
            "command":{
                "baseRevision":open["document"]["revision"],
                "transactionId":"electrical-create",
                "phase":"commit",
                "targetIds":[],
                "operation":synthetic_half::create_mirrored_half_operation(
                    "boundary-electrical-pair",
                    "right"
                )
            }
        }),
    );
    assert_eq!(created["kind"], "scene", "{created}");
    let pair_id = "boundary-electrical-pair";
    let source_net_id = "left/omit-controller-net";
    let mirror_net_id = created["document"]["hardware"]["pairs"][0]["halves"]["correspondences"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["kind"] == "net" && item["sourceId"] == source_net_id)
        .unwrap_or_else(|| panic!("plain net correspondence for {source_net_id} is missing"))["mirrorId"]
        .as_str()
        .unwrap()
        .to_owned();

    let localized = call(
        &mut core,
        json!({
            "id":"electrical-local-controller","kind":"edit",
            "command":{"baseRevision":created["document"]["revision"],
                "transactionId":"electrical-local-controller","phase":"commit",
                "targetIds":["right/U1"],
                "operation":boundary_exception(pair_id,"right/U1","local")}
        }),
    );
    assert_eq!(localized["kind"], "scene", "{localized}");
    let mut accepted_with_local_controller = localized["document"].clone();
    let mirror_net = accepted_with_local_controller["nets"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|net| net["id"] == mirror_net_id)
        .unwrap_or_else(|| panic!("mirror net {mirror_net_id} is missing"));
    mirror_net["pins"].as_array_mut().unwrap().push(json!({
        "partId":"right/U1","padId":"pad-1"
    }));
    let seeded = call(
        &mut core,
        json!({
            "id":"electrical-seed-mirror-pin","kind":"edit",
            "command":{"baseRevision":accepted_with_local_controller["revision"],
                "transactionId":"electrical-seed-mirror-pin","phase":"commit","targetIds":[],
                "operation":{"kind":"replace-document","document":accepted_with_local_controller}}
        }),
    );
    assert_eq!(seeded["kind"], "scene", "{seeded}");
    let accepted_with_mirror_pin = seeded["document"].clone();
    assert!(
        accepted_with_mirror_pin["nets"]
            .as_array()
            .unwrap()
            .iter()
            .find(|net| net["id"] == mirror_net_id)
            .unwrap()["pins"]
            .as_array()
            .unwrap()
            .iter()
            .any(|pin| pin["partId"] == "right/U1" && pin["padId"] == "pad-1")
    );

    let omitted = call(
        &mut core,
        json!({
            "id":"electrical-omit-reset","kind":"edit",
            "command":{"baseRevision":accepted_with_mirror_pin["revision"],
                "transactionId":"electrical-omit-reset","phase":"commit",
                "targetIds":["right/RST"],
                "operation":boundary_exception(pair_id,"right/RST","omitted")}
        }),
    );
    assert_eq!(omitted["kind"], "scene", "{omitted}");
    let accepted = omitted["document"].clone();
    let right_board_name = accepted["boards"]
        .as_array()
        .unwrap()
        .iter()
        .find(|board| board["id"] == "right")
        .unwrap()["name"]
        .as_str()
        .unwrap();
    let mirror_net_name = accepted["nets"]
        .as_array()
        .unwrap()
        .iter()
        .find(|net| net["id"] == mirror_net_id)
        .unwrap()["name"]
        .as_str()
        .unwrap();
    let controller_reference = accepted["parts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|part| part["id"] == "right/U1")
        .unwrap()["reference"]
        .as_str()
        .unwrap();

    let preview = call(
        &mut core,
        json!({
            "id":"electrical-rejoin-preview","kind":"edit",
            "command":{"baseRevision":accepted["revision"],
                "transactionId":"electrical-rejoin","phase":"preview",
                "targetIds":["right/U1"],
                "operation":boundary_exception(pair_id,"right/U1","linked")}
        }),
    );
    assert_eq!(preview["kind"], "preview", "{preview}");
    let mut expected_notices = vec![
        json!({
            "id":electrical_notice_id("pin", &[&mirror_net_id,"right/U1","pad-1"]),
            "severity":"info","scope":"pcb",
            "message":format!("Preview removes pad pad-1 on {controller_reference} (right/U1) from net {mirror_net_name} ({mirror_net_id})."),
            "targetIds":[mirror_net_id,"right/U1"]
        }),
        json!({
            "id":electrical_notice_id("net", &[&mirror_net_id]),
            "severity":"info","scope":"pcb",
            "message":format!("Preview removes net {mirror_net_name} ({mirror_net_id})."),
            "targetIds":[mirror_net_id]
        }),
        json!({
            "id":electrical_notice_id("registration", &["right",&mirror_net_id]),
            "severity":"info","scope":"pcb",
            "message":format!("Preview removes net {mirror_net_name} ({mirror_net_id}) from board {right_board_name} (right)."),
            "targetIds":["right",mirror_net_id]
        }),
    ];
    expected_notices.sort_by(|left, right| left["id"].as_str().cmp(&right["id"].as_str()));
    let mut electrical_notices: Vec<Value> = preview["scene"]["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|finding| {
            finding["id"]
                .as_str()
                .is_some_and(|id| id.starts_with("preview:electrical:"))
        })
        .cloned()
        .collect();
    electrical_notices.sort_by(|left, right| left["id"].as_str().cmp(&right["id"].as_str()));
    assert_eq!(electrical_notices, expected_notices, "{preview}");
    let after_preview = call(
        &mut core,
        json!({"id":"electrical-after-preview","kind":"snapshot"}),
    );
    assert_eq!(
        after_preview["document"], accepted,
        "Preview must not publish Rejoin"
    );

    let committed = call(
        &mut core,
        json!({
            "id":"electrical-rejoin-commit","kind":"edit",
            "command":{"baseRevision":accepted["revision"],
                "transactionId":"electrical-rejoin","phase":"commit",
                "targetIds":["right/U1"],
                "operation":boundary_exception(pair_id,"right/U1","linked")}
        }),
    );
    assert_eq!(committed["kind"], "scene", "{committed}");
    assert!(
        committed["document"]["nets"]
            .as_array()
            .unwrap()
            .iter()
            .all(|net| net["id"] != mirror_net_id)
    );
    assert!(
        committed["document"]["boards"]
            .as_array()
            .unwrap()
            .iter()
            .find(|board| board["id"] == "right")
            .unwrap()["netIds"]
            .as_array()
            .unwrap()
            .iter()
            .all(|net_id| net_id != &mirror_net_id)
    );
    assert!(
        committed["scene"]["findings"]
            .as_array()
            .unwrap()
            .iter()
            .all(|finding| finding["id"]
                .as_str()
                .is_none_or(|id| !id.starts_with("preview:electrical:")))
    );

    json!({
        "open":{"kind":open["kind"]},
        "created":{"kind":created["kind"],"pairId":pair_id,"mirrorNetId":mirror_net_id},
        "preview":{"kind":preview["kind"],"findings":electrical_notices,"acceptedDocumentUnchanged":true},
        "afterPreview":{"kind":after_preview["kind"],"documentUnchanged":true},
        "commit":{"kind":committed["kind"],"mirrorNetRemoved":true,"previewNoticeCount":0}
    })
}
