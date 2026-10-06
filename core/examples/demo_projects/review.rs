use super::*;
fn starter() -> Result<Value> {
    let mut doc = empty(
        "m1-vik-module-review-copy",
        "VIK module review · above and below",
    );
    let id = definition(
        &mut doc,
        "switch_mx",
        "generator:ceoloide/switch_mx",
        json!({"hotswap":false,"solder":true}),
    )?;
    doc["definitions"][0]["name"] = json!("MX switch");
    let terminals = doc["definitions"][0]["terminals"].clone();
    let mut rows = vec![Vec::new(); 3];
    let mut cols = vec![Vec::new(); 5];
    let mut ids = Vec::new();
    for (row, row_pins) in rows.iter_mut().enumerate() {
        for (col, col_pins) in cols.iter_mut().enumerate() {
            let part_id = format!("matrix/matrix/r{row}c{col}");
            ids.push(json!(part_id));
            push(
                &mut doc,
                "parts",
                json!({"id":part_id,"definitionId":id,"reference":format!("SW{}",row*5+col+1),"pose":{"at":{"x":col as f64*19.05,"y":row as f64*19.05},"rotation":0},"side":"front"}),
            );
            for pad in a(&terminals["from"]) {
                row_pins.push(json!({"partId":part_id,"padId":pad}));
            }
            for pad in a(&terminals["to"]) {
                col_pins.push(json!({"partId":part_id,"padId":pad}));
            }
        }
    }
    doc["matrices"] = json!([{"id":"matrix","boardId":"main-board","rows":3,"columns":5,"pitch":{"x":19.05,"y":19.05},"origin":{"x":0,"y":0},"definitionId":id,"partIds":ids}]);
    for (row, pins) in rows.iter().enumerate() {
        push(
            &mut doc,
            "nets",
            json!({"id":format!("row-{row}"),"name":format!("ROW{row}"),"pins":pins}),
        );
    }
    for (col, pins) in cols.iter().enumerate() {
        push(
            &mut doc,
            "nets",
            json!({"id":format!("col-{col}"),"name":format!("COL{col}"),"pins":pins}),
        );
    }
    doc["outline"] = json!([outline("board-envelope", json!(ids))]);
    doc["boards"] = json!([{"id":"main-board","name":"Main board","outlineIds":["board-envelope"],"partIds":ids,"netIds":a(&doc["nets"]).iter().map(|n|n["id"].clone()).collect::<Vec<_>>(),"thickness":1.6}]);
    doc["materials"] = json!([{"id":"pla","name":"PLA","thickness":3}]);
    doc["caseBodies"] = json!([{"id":"switch-plate","name":"Switch plate","boardId":"main-board","kind":"plate","thickness":3,"clearance":0.5,"materialId":"pla"}]);
    Ok(doc)
}
pub(super) fn build(engine: &mut CoreEngine) -> Result<Value> {
    let mut doc = starter()?;
    let metadata = read("content/layouts/module-review.json")?;
    doc["parameters"] = metadata["parameters"].clone();
    doc["keymap"] = metadata["keymap"].clone();
    let catalogue = read("catalogue/modules/imported-modules.json")?;
    let modules = a(&catalogue["modules"]);
    let get = |row: &str, variant: Option<&str>| -> Result<Value> {
        Ok(modules
            .iter()
            .find(|m| m["row"] == row && variant.is_none_or(|v| m["definition"]["variant"] == v))
            .ok_or("Missing review module")?["definition"]
            .clone())
    };
    let splitter = get("vik-splitter", None)?;
    let haptic = get(
        "haptic-drv2605l",
        Some("pcb/haptic-drv2605l/haptic-drv2605l · 3V3 pullups, JP1 bridged"),
    )?;
    let rotary = get("ec11-evqwgd001", None)?;
    let parts = read("catalogue/parts/imported-parts.json")?;
    for id in [
        "thqwgd001:rotation-reversible",
        "thqwgd001:c-2pin-reversible",
        "thqwgd001:c-4pin-reversible",
    ] {
        let d = a(&parts["parts"])
            .iter()
            .find(|p| p["definition"]["id"] == id)
            .ok_or("Missing THQ definition")?;
        push(&mut doc, "definitions", d["definition"].clone());
    }
    push(
        &mut doc,
        "parts",
        json!({"id":"review/wheel-rotation-only","definitionId":"thqwgd001:rotation-reversible","reference":"ENC1","pose":{"at":{"x":101,"y":18},"rotation":0},"side":"front"}),
    );
    doc["boards"][0]["partIds"]
        .as_array_mut()
        .unwrap()
        .push(json!("review/wheel-rotation-only"));
    doc["outline"][0]["partIds"]
        .as_array_mut()
        .unwrap()
        .push(json!("review/wheel-rotation-only"));
    doc["moduleDefinitions"] = json!([splitter, haptic, rotary]);
    doc["modules"] = json!([
        {"id":"review/splitter-above","definitionId":splitter["id"],"hostBoardId":"main-board","hostFace":"front","facingFace":"back","at":{"x":29,"y":20},"rotation":0,"gap":3,"attachment":"board","detached":false,"serviceClearance":4},
        {"id":"review/splitter-below","definitionId":splitter["id"],"hostBoardId":"main-board","hostFace":"back","facingFace":"front","at":{"x":57,"y":20},"rotation":180,"gap":3,"attachment":"board","detached":false,"serviceClearance":4},
        {"id":"review/ec11-rotary","definitionId":rotary["id"],"hostBoardId":"main-board","hostFace":"front","facingFace":"back","at":{"x":85,"y":20},"rotation":0,"gap":3,"attachment":"board","detached":false,"serviceClearance":4}]);
    doc = open(engine, doc)?;
    let connector = host_connector(modules)?;
    for original in a(&doc["modules"]).clone() {
        let mut instance = original;
        let definition = a(&doc["moduleDefinitions"])
            .iter()
            .find(|d| d["id"] == instance["definitionId"])
            .ok_or("Missing mounted definition")?;
        let z = if instance["facingFace"] == "front" {
            0.8
        } else {
            -3.8
        };
        instance["mountSupports"]=json!(a(&definition["mounts"]).iter().take(2).map(|m|json!({"mountId":m["sourceId"],"outerDiameter":6,"holeDiameter":2.8,"z":z,"height":3})).collect::<Vec<_>>());
        instance["connection"] = json!({"hostConnectorPartId":"","modulePortId":a(&definition["interfaces"]).iter().find(|p|p["role"]=="module").map(|p|p["id"].clone()).unwrap_or(json!("")),"busId":format!("review/{}/vik",s(&instance["id"])),"assignments":{},"cableType":"type-a-12-0.5","railVoltages":{}});
        let transaction = format!("review/attach/{}", s(&instance["id"]));
        doc = edit(
            engine,
            doc,
            json!({"kind":"set-mounted-module","instance":instance,"definition":null,"hostConnectorDefinition":connector}),
            &transaction,
        )?;
    }
    let mut matrix = doc["matrices"][0].clone();
    matrix["cells"] = json!(
        (0..15)
            .map(|index| {
                let row = index / 5;
                let column = index % 5;
                let mut cell = json!({"row":row,"column":column,"enabled":true});
                if row == 0 && column < 2 {
                    let id = if column == 0 {
                        "thqwgd001:c-2pin-reversible"
                    } else {
                        "thqwgd001:c-4pin-reversible"
                    };
                    cell["definitionId"] = json!(id);
                    cell["variant"] = json!(id);
                }
                cell
            })
            .collect::<Vec<_>>()
    );
    doc = edit(
        engine,
        doc,
        json!({"kind":"set-matrix","matrix":matrix}),
        "review/replace-matrix-encoders",
    )?;
    doc = edit(
        engine,
        doc,
        json!({"kind":"embed-module-circuit","id":"review/embedded-haptic","definition":haptic,"hostBoardId":"main-board","pose":{"at":{"x":101,"y":18},"rotation":0},"side":"front","joins":{}}),
        "review/embedded-haptic",
    )?;
    let mut feature = doc["outline"][0].clone();
    let circuit = a(&doc["embeddedCircuits"])
        .iter()
        .find(|c| c["id"] == "review/embedded-haptic")
        .ok_or("Missing embedded circuit")?;
    for id in a(&circuit["partIds"]) {
        if !a(&feature["partIds"]).contains(id) {
            feature["partIds"].as_array_mut().unwrap().push(id.clone());
        }
    }
    doc = edit(
        engine,
        doc,
        json!({"kind":"set-outline","feature":feature}),
        "review/outline-circuit",
    )?;
    Ok(doc)
}
fn host_connector(modules: &[Value]) -> Result<Value> {
    let mut d = modules
        .iter()
        .filter_map(|m| m["definition"]["circuit"]["definitions"].as_array())
        .flatten()
        .find(|d| {
            d["hardwareProfile"]["vikRole"] == "host"
                && s(&d["name"]).to_lowercase().contains("horizontal")
        })
        .ok_or("Missing horizontal VIK connector")?
        .clone();
    d["id"] = json!("vik:source:horizontal-host-connector");
    d["name"] = json!("VIK horizontal host connector");
    d["models"] = json!([{"assetId":"bundled-model:vik/sadekbaroudi-vik/kicad/3dmodels/vik-connector-horizontal.stp","offset":{"x":-2.75,"y":2.3,"z":0},"rotation":{"x":0,"y":0,"z":0},"scale":{"x":1,"y":1,"z":1}}]);
    if let Some(text) = d["kicadSource"]["source"].as_str() {
        d["kicadSource"]["source"] = json!(without_embedded_model(text)?);
    }
    Ok(d)
}

// Keep source artwork spans byte-for-byte, removing only the model replaced above.
fn without_embedded_model(source: &str) -> Result<String> {
    let marker = "(model \"../../kicad/3dmodels/vik-connector-horizontal.stp\"";
    let Some(start) = source.find(marker) else {
        return Ok(source.into());
    };
    let mut depth = 0;
    let mut quoted = false;
    let mut escaped = false;
    for (offset, character) in source[start..].char_indices() {
        if quoted {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                quoted = false;
            }
        } else if character == '"' {
            quoted = true;
        } else if character == '(' {
            depth += 1;
        } else if character == ')' {
            depth -= 1;
            if depth == 0 {
                return Ok(format!(
                    "{}{}",
                    &source[..start],
                    &source[start + offset + 1..]
                ));
            }
        }
    }
    Err("Malformed embedded VIK host connector model".into())
}
