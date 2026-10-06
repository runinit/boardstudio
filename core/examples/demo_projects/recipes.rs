use super::*;
use physical::Key;
fn hardware(split: bool) -> Value {
    json!({"topology":if split{"split"}else{"unibody"},"transport":if split{"wired"}else{"none"},"instances":[],"boards":[],"sharedConstruction":null})
}
fn board(
    doc: &mut Value,
    id: &str,
    ids: Vec<Value>,
    parts: Vec<Value>,
    controller: &str,
) -> Result<()> {
    let outline_id = format!("{id}-outline");
    push(doc, "outline", outline(&outline_id, json!(ids)));
    push(
        doc,
        "boards",
        json!({"id":id,"name":if id=="main"{"Keyboard PCB"}else if id=="left"{"Left PCB"}else{"Right PCB"},"thickness":1.6,"partIds":ids,"netIds":[],"outlineIds":[outline_id]}),
    );
    let layout = doc["layouts"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|l| l["boardId"] == id)
        .ok_or("Missing layout")?;
    layout["partIds"] = json!(parts.iter().map(|p| p["id"].clone()).collect::<Vec<_>>());
    push(
        &mut doc["hardware"],
        "instances",
        json!({"id":id,"name":id,"boardId":id,"half":id,"role":if id=="right"{"peripheral"}else{"central"},"flipped":false,"controllerPartId":controller,"mechanical":null,"constructionLinked":false}),
    );
    push(
        &mut doc["hardware"],
        "boards",
        json!({"boardId":id,"controllerPartId":controller,"mode":"matrix","locks":{},"assignments":{},"keyBindings":{},"jumperStates":{},"protectedHandoff":null}),
    );
    Ok(())
}
pub(super) fn keyboard(id: &str, layout: &Value) -> Result<Value> {
    let mut doc = empty(&format!("m1-{id}-copy"), s(&layout["name"]));
    doc["parameters"] = json!({"demo":id,"source":format!("{}/blob/{}/{}",s(&layout["repository"]),s(&layout["revision"]),s(&layout["path"])),"sourceSha256":layout["sha256"],"adaptation":"Measured key layout; library switches, diode matrix, nice!nano controller bay, and a live generated outline. Source peripherals and routing are not reproduced."});
    let split = layout["split"].as_bool().unwrap();
    let choc = layout["choc"].as_bool().unwrap();
    doc["hardware"] = hardware(split);
    let controller = definition(
        &mut doc,
        "mcu_nice_nano",
        "demo/mcu_nice_nano",
        json!({"include_extra_pins":true}),
    )?;
    let reset = definition(
        &mut doc,
        "reset_switch_tht_top",
        "demo/reset_switch_tht_top",
        json!({}),
    )?;
    let connector = if split {
        Some(definition(
            &mut doc,
            "trrs_pj320a",
            "demo/trrs_pj320a",
            json!({}),
        )?)
    } else {
        None
    };
    let keys = physical::keys(id, &layout["keys"]);
    let min_x = keys
        .iter()
        .map(|k| k.x - k.width * 9.525)
        .fold(f64::INFINITY, f64::min);
    let max_x = keys
        .iter()
        .map(|k| k.x + k.width * 9.525)
        .fold(f64::NEG_INFINITY, f64::max);
    let min_y = keys.iter().map(|k| k.y).fold(f64::INFINITY, f64::min);
    let width = max_x - min_x + 44.;
    let source_right = id == "lily58" || id == "klor";
    for board_id in if split {
        vec!["left", "right"]
    } else {
        vec!["main"]
    } {
        let mirror = split && ((board_id == "right") != source_right);
        let shift = if board_id == "right" { width + 25. } else { 0. };
        let mut clusters = Vec::new();
        for k in &keys {
            if !clusters.contains(&k.cluster) {
                clusters.push(k.cluster.clone());
            }
        }
        for cluster in clusters {
            let mut members: Vec<_> = keys
                .iter()
                .filter(|k| k.cluster == cluster)
                .cloned()
                .collect();
            let last = members.iter().map(|k| k.column).max().unwrap();
            for k in &mut members {
                k.column = if mirror { last - k.column } else { k.column };
                k.x = (if mirror { max_x - k.x } else { k.x - min_x }) + 6. + shift;
                k.y = min_y - k.y;
                if mirror {
                    k.rotation = -k.rotation;
                }
            }
            physical::append(
                &mut doc,
                &members,
                &format!("{board_id}-{cluster}"),
                board_id,
                if choc { "choc-hotswap" } else { "mx-hotswap" },
                if choc { (18., 17.) } else { (19.05, 19.05) },
            )?;
        }
        let bay = shift + width - 16.;
        let mut parts = vec![
            json!({"id":format!("{board_id}/U1"),"definitionId":controller,"reference":format!("{board_id}-U1"),"side":"front","pose":{"at":{"x":bay,"y":-15},"rotation":0}}),
            json!({"id":format!("{board_id}/RST"),"definitionId":reset,"reference":format!("{board_id}-RST"),"side":"front","pose":{"at":{"x":bay,"y":-45},"rotation":0}}),
        ];
        if let Some(connector) = &connector {
            parts.push(json!({"id":format!("{board_id}/TRRS"),"definitionId":connector,"reference":format!("{board_id}-TRRS"),"side":"front","pose":{"at":{"x":bay,"y":-65},"rotation":0}}));
        }
        for p in &parts {
            push(&mut doc, "parts", p.clone());
        }
        let ids = a(&doc["parts"])
            .iter()
            .filter(|p| {
                s(&p["id"]).starts_with(&format!("{board_id}/"))
                    || s(&p["id"]).starts_with(&format!("matrix/{board_id}-"))
            })
            .map(|p| p["id"].clone())
            .collect();
        board(&mut doc, board_id, ids, parts, &format!("{board_id}/U1"))?;
    }
    Ok(doc)
}
pub(super) fn sofle(variant: &str) -> Result<Value> {
    let measurements = read("content/layouts/sofle-layouts.json")?;
    let layout = &measurements["layouts"][variant];
    let name = match variant {
        "v2" => "Sofle v2",
        "rgb" => "Sofle RGB",
        _ => "Sofle Choc",
    };
    let mut doc = empty(&format!("m1-sofle-{variant}-copy"), name);
    doc["parameters"] = json!({"demo":format!("sofle-{variant}"),"source":format!("{}/blob/{}/{}",s(&measurements["repository"]),s(&measurements["revision"]),s(&layout["path"])),"sourceSha256":layout["sha256"]});
    doc["hardware"] = hardware(true);
    let mut definitions = BTreeMap::new();
    for source in [
        "mcu_nice_nano",
        "rotary_encoder_ec11_ec12",
        "display_ssd1306",
        "trrs_pj320a",
        "reset_switch_tht_top",
        "mounting_hole_npth",
    ] {
        definitions.insert(
            source,
            definition(
                &mut doc,
                source,
                &format!("sofle/{source}"),
                if source == "mcu_nice_nano" {
                    json!({"include_extra_pins":true})
                } else {
                    json!({})
                },
            )?,
        );
    }
    if variant == "rgb" {
        definitions.insert(
            "led_sk6812mini-e",
            definition(
                &mut doc,
                "led_sk6812mini-e",
                "sofle/led_sk6812mini-e",
                json!({"side":"B","reverse_mount":false}),
            )?,
        );
    }
    let min_x = a(&layout["outline"])
        .iter()
        .map(|p| n(&p["x"]))
        .fold(f64::INFINITY, f64::min);
    let max_x = a(&layout["outline"])
        .iter()
        .map(|p| n(&p["x"]))
        .fold(f64::NEG_INFINITY, f64::max);
    let min_y = a(&layout["outline"])
        .iter()
        .map(|p| n(&p["y"]))
        .fold(f64::INFINITY, f64::min);
    let width = max_x - min_x;
    let measured: Vec<_> = a(&layout["components"]).iter().map(Key::measured).collect();
    for half in ["left", "right"] {
        let start = a(&doc["parts"]).len();
        let pos = |x: f64, y: f64| {
            (
                if half == "left" {
                    max_x - x
                } else {
                    x - min_x + width + 30.
                },
                min_y - y,
            )
        };
        let angle = |r: f64| if half == "left" { -r } else { r };
        for cluster in ["keys", "thumbs"] {
            let mut keys: Vec<_> = measured
                .iter()
                .filter(|k| {
                    k.reference.starts_with("SW")
                        && k.reference != "SW25"
                        && (k.number() <= 24) == (cluster == "keys")
                })
                .cloned()
                .collect();
            for k in &mut keys {
                let num = k.number();
                let column = if cluster == "keys" {
                    (num - 1) % 6
                } else {
                    num - 26
                };
                (k.x, k.y) = pos(k.x, k.y);
                k.width = 1.;
                k.cluster = cluster.into();
                k.row = if cluster == "keys" {
                    3 - (num - 1) / 6
                } else {
                    0
                };
                k.column = if half == "left" {
                    if cluster == "keys" {
                        5 - column
                    } else {
                        4 - column
                    }
                } else {
                    column
                };
                k.rotation = angle(k.rotation - if variant == "v2" { 0. } else { 180. });
            }
            physical::append(
                &mut doc,
                &keys,
                &format!("{half}-{cluster}"),
                half,
                match variant {
                    "v2" => "mx-hotswap",
                    "rgb" => "mx-hotswap-rgb",
                    _ => "choc-hotswap-rgb",
                },
                (19.05, 19.05),
            )?;
        }
        let mut parts = Vec::new();
        let mut add = |reference: &str,
                       source: &str,
                       r: f64,
                       offset: (f64, f64),
                       side: &str|
         -> Result<()> {
            let m = measured
                .iter()
                .find(|k| k.reference == reference)
                .ok_or("Missing Sofle measurement")?;
            let (x, y) = pos(m.x + offset.0, m.y + offset.1);
            parts.push(json!({"id":format!("{half}/{reference}"),"reference":format!("{half}-{reference}"),"definitionId":definitions[source],"side":side,"pose":{"at":{"x":x,"y":y},"rotation":angle(r)}}));
            Ok(())
        };
        add("U1", "mcu_nice_nano", 0., (0., 0.), "front")?;
        add("SW25", "rotary_encoder_ec11_ec12", 0., (0., 0.), "front")?;
        add("J3", "display_ssd1306", 0., (3.81, -16.7), "front")?;
        add("J2", "trrs_pj320a", 90., (4., 0.), "front")?;
        add("RSW1", "reset_switch_tht_top", 90., (1., 0.), "front")?;
        for m in measured.iter().filter(|m| m.reference.starts_with("TH")) {
            add(&m.reference, "mounting_hole_npth", 0., (0., 0.), "front")?;
        }
        if variant == "rgb" {
            for m in measured.iter().filter(|m| {
                ["D31", "D32", "D33", "D34", "D35", "D36", "D37"].contains(&m.reference.as_str())
            }) {
                add(
                    &m.reference,
                    "led_sk6812mini-e",
                    m.rotation,
                    (0., 0.),
                    "back",
                )?;
            }
        }
        for p in &parts {
            push(&mut doc, "parts", p.clone());
        }
        let ids = a(&doc["parts"])[start..]
            .iter()
            .map(|p| p["id"].clone())
            .collect();
        board(&mut doc, half, ids, parts, &format!("{half}/U1"))?;
        doc["hardware"]["instances"]
            .as_array_mut()
            .unwrap()
            .last_mut()
            .unwrap()["name"] = json!(format!("{half} half"));
    }
    Ok(doc)
}
pub(super) fn gasket() -> Value {
    json!({"boardId":"left","method":"printed","mount":"gasket","integratedPlateFrame":false,"bottomStyle":"shell","middleFrame":false,"plateThickness":1.5,"plateFoamThickness":3,"pcbThickness":1.6,"bottomFoamThickness":2,"batteryHeight":0,"bottomThickness":3,"plateToPcb":3.5,"wallThickness":2,"clearance":0.3,"mounts":[],"partProcesses":[{"partId":"plate","method":"printed","material":"PLA","thickness":1.5,"constraintsVersion":"2026-09-24"},{"partId":"plate-foam","method":"cut-sheet","material":"EVA","thickness":3,"constraintsVersion":"2026-09-24"},{"partId":"bottom-foam","method":"cut-sheet","material":"EVA","thickness":2,"constraintsVersion":"2026-09-24"},{"partId":"bottom","method":"printed","material":"PLA","thickness":3,"constraintsVersion":"2026-09-24"}],"profiles":[],"gasketTravel":0.3})
}
