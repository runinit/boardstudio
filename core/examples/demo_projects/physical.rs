use super::*;
#[derive(Clone)]
pub(super) struct Key {
    pub reference: String,
    pub x: f64,
    pub y: f64,
    pub rotation: f64,
    pub width: f64,
    pub row: usize,
    pub column: usize,
    pub cluster: String,
}
impl Key {
    pub fn measured(v: &Value) -> Self {
        Self {
            reference: s(&v["reference"]).into(),
            x: n(&v["x"]),
            y: n(&v["y"]),
            rotation: n(&v["rotation"]),
            width: v["width"].as_f64().unwrap_or(1.),
            row: 0,
            column: 0,
            cluster: "keys".into(),
        }
    }
    pub fn number(&self) -> usize {
        self.reference
            .chars()
            .rev()
            .take_while(|c| c.is_ascii_digit())
            .collect::<String>()
            .chars()
            .rev()
            .collect::<String>()
            .parse()
            .unwrap()
    }
}
pub(super) fn keys(id: &str, values: &Value) -> Vec<Key> {
    let mut keys: Vec<_> = a(values).iter().map(Key::measured).collect();
    let top = keys.iter().map(|k| k.y).fold(f64::INFINITY, f64::min);
    let bottom = keys.iter().map(|k| k.y).fold(f64::NEG_INFINITY, f64::max);
    for k in &mut keys {
        let num = k.number();
        let mut regular = |cols: usize, rows: usize, index: usize| {
            k.row = rows - 1 - index / cols;
            k.column = index % cols;
        };
        match id {
            "corne" | "lily58" | "sweep" | "chocofi" => {
                let rows = if id == "lily58" { 4 } else { 3 };
                if num > rows * 6 {
                    k.cluster = "thumbs".into();
                } else {
                    regular(6, rows, num - 1);
                    if id == "sweep" || id == "chocofi" {
                        k.column -= 1;
                    }
                }
            }
            "cantor" => {
                if num >= 30 {
                    k.cluster = "thumbs".into();
                } else {
                    k.row = 2 - num / 10;
                    k.column = num % 10;
                }
            }
            "totem" => {
                if num >= 17 {
                    k.cluster = "thumbs".into();
                } else if num == 16 {
                    k.cluster = "outer".into();
                } else {
                    regular(5, 3, num - 1);
                }
            }
            "klor" => {
                if num >= 19 {
                    k.cluster = "thumbs".into();
                } else {
                    k.row = if num <= 5 {
                        2
                    } else if num <= 11 {
                        1
                    } else {
                        0
                    };
                    k.column = if num <= 5 {
                        num - 1
                    } else if num <= 11 {
                        num - 6
                    } else {
                        num - 12
                    };
                }
            }
            "reviung41" => {
                if num >= 37 {
                    k.cluster = "thumbs".into();
                } else {
                    regular(6, 3, (num - 1) % 18);
                    k.cluster = if num <= 18 { "left-keys" } else { "right-keys" }.into();
                }
            }
            _ => {
                if id == "lumberjack" {
                    k.cluster = if k.x < 150. {
                        "left-keys"
                    } else {
                        "right-keys"
                    }
                    .into();
                }
                if ["mysterium", "voyager104", "voyager97"].contains(&id) {
                    let nav = if id == "voyager97" { 320. } else { 330. };
                    let num = if id == "voyager104" {
                        390.
                    } else if id == "voyager97" {
                        350.
                    } else {
                        f64::INFINITY
                    };
                    k.cluster = if k.x >= num {
                        "numpad"
                    } else if k.x >= nav {
                        if k.y >= bottom - 20. {
                            "arrows"
                        } else {
                            "navigation"
                        }
                    } else if k.y < top + 5. {
                        "function"
                    } else {
                        "keys"
                    }
                    .into();
                    if id == "voyager97"
                        && ["MX_LEFT1", "MX_DOWN1", "MX_RIGHT1", "MX_UP1"]
                            .contains(&k.reference.as_str())
                    {
                        k.cluster = "arrows".into();
                    }
                }
            }
        }
    }
    if ![
        "corne",
        "lily58",
        "sweep",
        "chocofi",
        "cantor",
        "totem",
        "klor",
        "reviung41",
    ]
    .contains(&id)
    {
        let mut clusters = Vec::new();
        for k in &keys {
            if !clusters.contains(&k.cluster) {
                clusters.push(k.cluster.clone());
            }
        }
        // Preserve the JavaScript Map's cluster grouping and source member order.
        keys = clusters
            .iter()
            .flat_map(|c| keys.iter().filter(move |k| &k.cluster == c).cloned())
            .collect();
        for c in clusters {
            let members: Vec<_> = keys.iter().filter(|k| k.cluster == c).cloned().collect();
            let mut sorted = members.clone();
            sorted.sort_by(|a, b| b.y.total_cmp(&a.y));
            let mut levels: Vec<f64> = Vec::new();
            for k in sorted {
                if !levels.iter().any(|y| (y - k.y).abs() < 0.5) {
                    levels.push(k.y);
                }
            }
            let bottom = members
                .iter()
                .map(|k| k.y)
                .fold(f64::NEG_INFINITY, f64::max);
            let left = members.iter().map(|k| k.x).fold(f64::INFINITY, f64::min);
            for k in keys.iter_mut().filter(|k| k.cluster == c) {
                k.row = if c == "numpad" {
                    ((bottom - k.y) / 19.05).round() as usize
                } else {
                    levels.iter().position(|y| (y - k.y).abs() < 0.5).unwrap()
                };
                let mut row: Vec<_> = members.iter().filter(|o| (o.y - k.y).abs() < 0.5).collect();
                row.sort_by(|a, b| a.x.total_cmp(&b.x));
                k.column = if c == "numpad" || c == "arrows" {
                    ((k.x - left) / 19.05).round() as usize
                } else {
                    row.iter().position(|o| o.reference == k.reference).unwrap()
                };
            }
        }
    }
    let mut thumbs: Vec<_> = keys
        .iter()
        .enumerate()
        .filter(|(_, k)| k.cluster == "thumbs")
        .map(|(i, k)| (i, k.x))
        .collect();
    thumbs.sort_by(|a, b| a.1.total_cmp(&b.1));
    for (column, (i, _)) in thumbs.into_iter().enumerate() {
        keys[i].column = column;
    }
    keys
}
fn rotate(p: (f64, f64), degrees: f64, o: (f64, f64)) -> (f64, f64) {
    let r = degrees * std::f64::consts::PI / 180.;
    let (x, y) = (p.0 - o.0, p.1 - o.1);
    (
        o.0 + x * r.cos() - y * r.sin(),
        o.1 + x * r.sin() + y * r.cos(),
    )
}
fn inverse(
    mut p: (f64, f64),
    through: usize,
    splays: &[f64],
    origins: &[(f64, f64)],
) -> (f64, f64) {
    for col in 0..through {
        p = rotate(p, -splays[col], origins[col]);
    }
    p
}
fn point(p: (f64, f64)) -> Value {
    json!({"x":p.0,"y":p.1})
}
pub(super) fn append(
    doc: &mut Value,
    keys: &[Key],
    id: &str,
    board: &str,
    preset: &str,
    pitch: (f64, f64),
) -> Result<()> {
    let rows = keys.iter().map(|k| k.row).max().unwrap() + 1;
    let columns = keys.iter().map(|k| k.column).max().unwrap() + 1;
    let origin = (
        keys.iter().map(|k| k.x).fold(f64::INFINITY, f64::min),
        keys.iter().map(|k| k.y).fold(f64::INFINITY, f64::min),
    );
    let angles: Vec<_> = (0..columns)
        .map(|c| {
            keys.iter()
                .find(|k| k.column == c)
                .map_or(0., |k| k.rotation)
        })
        .collect();
    let splays: Vec<_> = angles
        .iter()
        .enumerate()
        .map(|(i, r)| r - if i == 0 { 0. } else { angles[i - 1] })
        .collect();
    let mut origins = Vec::new();
    let mut offsets = Vec::new();
    let mut staggers = Vec::new();
    for col in 0..columns {
        let bottom = keys
            .iter()
            .filter(|k| k.column == col)
            .min_by_key(|k| k.row);
        let base = if let Some(k) = bottom {
            let rise = rotate((0., k.row as f64 * pitch.1), angles[col], (0., 0.));
            (k.x - origin.0 - rise.0, k.y - origin.1 - rise.1)
        } else {
            (col as f64 * pitch.0, 0.)
        };
        let local = inverse(base, col, &splays, &origins);
        offsets.push(point((local.0 - col as f64 * pitch.0, 0.)));
        staggers.push(local.1 - origins.last().map_or(0., |p: &(f64, f64)| p.1));
        origins.push(local);
    }
    let choc = preset.starts_with("choc");
    let led = preset.ends_with("rgb");
    let hotswap = preset.contains("hotswap");
    let base = format!("assembly-preset-{preset}-south-{id}-0/definition");
    let switch = definition(
        doc,
        if choc {
            "switch_choc_v1_v2"
        } else {
            "switch_mx"
        },
        &format!("{base}/switch"),
        if choc {
            json!({"hotswap":hotswap,"solder":!hotswap,"reversible":false,"side":"B","include_keycap":true,"choc_v1_support":true,"choc_v2_support":false,"include_choc_v1_led_cutout_marks":true})
        } else {
            json!({"hotswap":hotswap,"solder":!hotswap,"reversible":false,"side":"B","include_keycap":true})
        },
    )?;
    let diode = definition(
        doc,
        "diode_tht_sod123",
        &format!("{base}/diode"),
        json!({"side":"B","reversible":false,"include_tht":false}),
    )?;
    let led_id = if led {
        Some(definition(
            doc,
            "led_sk6812mini-e",
            &format!("{base}/led"),
            json!({"side":"B","reverse_mount":true,"reversible":false}),
        )?)
    } else {
        None
    };
    let imported = read("catalogue/parts/imported-parts.json")?;
    let mut cells = Vec::new();
    let mut part_ids = Vec::new();
    for row in 0..rows {
        for col in 0..columns {
            let key = keys.iter().find(|k| k.row == row && k.column == col);
            let local = key.map_or(origins[col], |k| {
                inverse((k.x - origin.0, k.y - origin.1), col + 1, &splays, &origins)
            });
            let mut members = vec![
                json!({"id":"diode","definitionId":diode,"offset":{"x":7.4,"y":-1.5},"rotation":90,"side":"back"}),
            ];
            if let Some(id) = &led_id {
                members.push(json!({"id":"led","definitionId":id,"offset":{"x":0,"y":if choc{-4.7}else{-4.75}},"rotation":180,"side":"back"}));
            }
            if let Some(k) = key.filter(|k| k.width >= 2.) {
                let suffix = if k.width == 6.25 { "6.25u" } else { "2u" };
                let d = a(&imported["parts"])
                    .iter()
                    .map(|p| &p["definition"])
                    .find(|d| s(&d["id"]).ends_with(suffix))
                    .ok_or("Missing stabilizer")?;
                if !a(&doc["definitions"]).iter().any(|o| o["id"] == d["id"]) {
                    push(doc, "definitions", d.clone());
                }
                members.push(json!({"id":"stabilizer","definitionId":d["id"],"offset":{"x":0,"y":0},"rotation":0,"side":"front"}));
            }
            cells.push(json!({"row":row,"column":col,"enabled":key.is_some(),"offset":point((local.0-origins[col].0,local.1-origins[col].1-row as f64*pitch.1)),"rotation":key.map_or(0.,|k|k.rotation-angles[col]),"definitionId":switch,"assembliesLocal":true,"assemblies":members,"variant":format!("preset/{preset}/south")}));
        }
    }
    // Matrix cells determine placement order; member references retain source indices.
    let mut ordered: Vec<_> = keys.iter().collect();
    ordered.sort_by_key(|k| (k.row, k.column));
    for k in ordered {
        let index = keys
            .iter()
            .position(|item| item.reference == k.reference)
            .unwrap();
        let part_id = format!("matrix/{id}/r{}c{}", k.row, k.column);
        let mut p = json!({"id":part_id,"definitionId":switch,"reference":format!("{board}-{}-SW{}",k.cluster,k.row*columns+k.column+1),"properties":{"sourceReference":k.reference},"pose":{"at":{"x":k.x,"y":k.y},"rotation":k.rotation},"side":"front"});
        if k.width != 1. {
            p["keycap"] = json!({"x":18.+(k.width-1.)*19.05,"y":18});
        }
        part_ids.push(json!(part_id));
        push(doc, "parts", p);
        for m in a(&cells[k.row * columns + k.column]["assemblies"]) {
            let offset = rotate(
                (n(&m["offset"]["x"]), n(&m["offset"]["y"])),
                k.rotation,
                (0., 0.),
            );
            let member_id = format!("{part_id}/{}", s(&m["id"]));
            part_ids.push(json!(member_id));
            push(
                doc,
                "parts",
                json!({"id":member_id,"definitionId":m["definitionId"],"reference":format!("{board}-{}-{}-{}",s(&m["id"]),index+1,keys[0].cluster),"side":m["side"],"pose":{"at":{"x":k.x+offset.0,"y":k.y+offset.1},"rotation":k.rotation+n(&m["rotation"])}}),
            );
        }
    }
    let name = keys[0].cluster.replace('-', " ");
    push(
        doc,
        "matrices",
        json!({"id":id,"name":name,"boardId":board,"rows":rows,"columns":columns,"origin":point(origin),"pitch":point(pitch),"definitionId":switch,"partIds":part_ids,"columnOffsets":offsets,"columnStaggers":staggers,"columnSplays":splays,"columnOrigins":origins.iter().map(|p|point(*p)).collect::<Vec<_>>(),"cells":cells,"diodeDirection":"col2row"}),
    );
    if doc["layouts"].is_null() {
        doc["layouts"] = json!([]);
    }
    push(
        doc,
        "layouts",
        json!({"id":format!("{id}-layout"),"name":name,"boardId":board,"matrixId":id,"partIds":[]}),
    );
    Ok(())
}
