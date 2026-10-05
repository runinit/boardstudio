use boardstudio_core::{CoreEngine, artifact, model::*};
use serde_json::{Value, json};

fn document() -> Value {
    let mut doc = serde_json::to_value(ProjectDoc::empty("modules", "Modules")).unwrap();
    doc["boards"] = json!([{"id":"host","name":"Host","partIds":[],"netIds":[],"outlineIds":["outline"],"thickness":1.6}]);
    doc["outline"] = json!([{"kind":"rect","id":"outline","center":{"x":0,"y":0},"size":{"x":80,"y":80},"radius":0,"operation":"add"}]);
    doc["moduleDefinitions"] = json!([{
        "id":"reference","name":"Mounted module","family":"test","variant":"support",
        "source":{"repository":"https://example.invalid/module","revision":"test","path":"pcb","license":"CC0-1.0"},
        "board":{"thickness":1.6,"contours":[{"hole":false,"points":[{"x":-12.5,"y":-12.5},{"x":12.5,"y":-12.5},{"x":12.5,"y":12.5},{"x":-12.5,"y":12.5}]}]},
        "mounts":[{"sourceId":"mh1","at":{"x":5,"y":-5},"diameter":2.2}],"volumes":[],"openings":[],"models":[],"gates":[],"interfaces":[],
        "electrical":{"protocol":"pass-through","requiredSignals":[]},"constituents":[]
    }]);
    doc["modules"] = json!([{
        "id":"module-1","definitionId":"reference","hostBoardId":"host","hostFace":"front","facingFace":"front",
        "at":{"x":20,"y":-5},"rotation":90,"gap":3,"attachment":"board",
        "mountSupports":[{"mountId":"mh1","outerDiameter":6,"holeDiameter":2.8,"z":0.8,"height":3}]
    }]);
    doc["mechanical"] = json!({"boardId":"host","method":"printed","mount":"rigid","integratedPlateFrame":false,"bottomStyle":"shell",
        "plateThickness":1.5,"plateFoamThickness":0,"pcbThickness":1.6,"bottomFoamThickness":0,"batteryHeight":0,"bottomThickness":2,
        "plateToPcb":6,"wallThickness":2,"clearance":0.2,"profiles":[],"mounts":[]});
    doc
}

fn request(engine: &mut CoreEngine, value: Value) -> Value {
    serde_json::from_str(&engine.request(&value.to_string())).unwrap()
}

fn host_outline() -> Vec<Contour> {
    vec![Contour {
        hole: false,
        points: vec![
            Vec2 { x: -40.0, y: -40.0 },
            Vec2 { x: 40.0, y: -40.0 },
            Vec2 { x: 40.0, y: 40.0 },
            Vec2 { x: -40.0, y: 40.0 },
        ],
    }]
}

#[test]
fn board_attached_supports_export_host_npth_holes_and_real_standoff_solids() {
    let doc = document();
    let module_scene = request(
        &mut CoreEngine::new(),
        json!({"id":"modules","kind":"resolve-modules","document":doc.clone(),"boardId":"host","previewTopZ":0.8}),
    );
    assert_eq!(module_scene["kind"], "modules-resolved", "{module_scene}");
    let resolved_support = &module_scene["result"]["modules"][0]["mountSupports"][0];
    assert_eq!(resolved_support["mountId"], "mh1");
    assert!((resolved_support["at"]["x"].as_f64().unwrap() - 25.0).abs() < 1e-8);
    assert!((resolved_support["at"]["y"].as_f64().unwrap() + 10.0).abs() < 1e-8);
    assert!((resolved_support["holeDiameter"].as_f64().unwrap() - 2.8).abs() < 1e-8);
    let support_preview = module_scene["result"]["preview"]["bodies"]
        .as_array()
        .unwrap()
        .iter()
        .find(|body| body["body"]["id"] == "module-standoff/module-1/mh1")
        .expect("the standalone standoff is visible in the board-attached module preview");
    assert!((support_preview["body"]["z"].as_f64().unwrap() - 0.8).abs() < 1e-8);
    assert!((support_preview["body"]["thickness"].as_f64().unwrap() - 3.0).abs() < 1e-8);

    let resolved = request(
        &mut CoreEngine::new(),
        json!({"id":"case","kind":"resolve-mechanical","document":doc,"contours":host_outline()}),
    );
    assert_eq!(resolved["kind"], "mechanical-resolved", "{resolved}");
    let assembly = &resolved["assembly"];
    assert_eq!(
        assembly["generationBlocked"], false,
        "{}",
        assembly["diagnostics"]
    );

    let standoff = assembly["case"]["bodies"]
        .as_array()
        .unwrap()
        .iter()
        .find(|body| body["body"]["id"] == "module-standoff/module-1/mh1")
        .expect("a separate exported standoff body");
    assert_eq!(standoff["body"]["kind"], "plate");
    assert!((standoff["body"]["z"].as_f64().unwrap() - 0.0).abs() < 1e-8);
    assert!((standoff["body"]["thickness"].as_f64().unwrap() - 3.0).abs() < 1e-8);
    assert!(
        standoff["contours"]
            .as_array()
            .unwrap()
            .iter()
            .any(|contour| contour["hole"] == true)
    );
    let prepared = request(
        &mut CoreEngine::new(),
        json!({"id":"standoff","kind":"prepare-case","ir":{"revision":0,"bodies":[standoff]}}),
    );
    assert_eq!(prepared["kind"], "case-prepared", "{prepared}");
    let prepared_standoff = &prepared["ir"]["bodies"][0];
    assert_eq!(
        prepared_standoff["body"]["id"],
        "module-standoff/module-1/mh1"
    );
    assert_eq!(
        prepared_standoff["regions"][0]["holes"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let outer = prepared_standoff["regions"][0]["outer"].as_array().unwrap();
    let outer_width = outer
        .iter()
        .map(|p| p["x"].as_f64().unwrap())
        .fold(f64::NEG_INFINITY, f64::max)
        - outer
            .iter()
            .map(|p| p["x"].as_f64().unwrap())
            .fold(f64::INFINITY, f64::min);
    assert!((outer_width - 6.0).abs() < 0.05, "{prepared_standoff}");
    let inner = prepared_standoff["regions"][0]["holes"][0]
        .as_array()
        .unwrap();
    let inner_width = inner
        .iter()
        .map(|p| p["x"].as_f64().unwrap())
        .fold(f64::NEG_INFINITY, f64::max)
        - inner
            .iter()
            .map(|p| p["x"].as_f64().unwrap())
            .fold(f64::INFINITY, f64::min);
    assert!((inner_width - 2.8).abs() < 0.05, "{prepared_standoff}");

    let pcb_reference = &assembly["pcbReference"];
    let drilled_hole = pcb_reference["contours"]
        .as_array()
        .unwrap()
        .iter()
        .find(|contour| {
            contour["hole"] == true
                && contour["points"].as_array().unwrap().iter().all(|point| {
                    let x = point["x"].as_f64().unwrap();
                    let y = point["y"].as_f64().unwrap();
                    ((x - 25.0).powi(2) + (y + 10.0).powi(2)).sqrt() > 1.3
                })
        })
        .expect("the host PCB reference has a hole at the transformed source mount");
    let hole_points = drilled_hole["points"].as_array().unwrap();
    assert!(
        (hole_points
            .iter()
            .map(|p| p["x"].as_f64().unwrap())
            .fold(f64::INFINITY, f64::min)
            - 23.6)
            .abs()
            < 0.05
    );
    assert!(
        (hole_points
            .iter()
            .map(|p| p["x"].as_f64().unwrap())
            .fold(f64::NEG_INFINITY, f64::max)
            - 26.4)
            .abs()
            < 0.05
    );
    assert!(
        (hole_points
            .iter()
            .map(|p| p["y"].as_f64().unwrap())
            .fold(f64::INFINITY, f64::min)
            + 11.4)
            .abs()
            < 0.05
    );
    assert!(
        (hole_points
            .iter()
            .map(|p| p["y"].as_f64().unwrap())
            .fold(f64::NEG_INFINITY, f64::max)
            + 8.6)
            .abs()
            < 0.05
    );

    let project: ProjectDoc = serde_json::from_value(doc).unwrap();
    let plan = artifact::kicad::prepare_export(PrepareExportRequest {
        snapshot_token: "module-support-host-drill".into(),
        expected_revision: 0,
        document: project,
        target: ExportTarget::Board {
            board_id: "host".into(),
        },
        contours: host_outline(),
        model_paths: Default::default(),
    })
    .unwrap();
    let exported = artifact::kicad::finish_export(FinishExportRequest {
        plan,
        results: vec![],
    })
    .unwrap();
    let board = &exported
        .files
        .iter()
        .find(|file| file.filename.ends_with(".kicad_pcb"))
        .unwrap()
        .content;
    assert!(board.contains("(pad \"\" np_thru_hole circle"), "{board}");
    assert!(board.contains("(at 25 10)"), "{board}");
    assert!(board.contains("(size 2.8 2.8)"), "{board}");
    assert!(board.contains("(drill 2.8)"), "{board}");
    assert!(
        board.contains("exclude_from_pos_files exclude_from_bom"),
        "{board}"
    );
    assert_eq!(
        board.matches("(footprint ").count(),
        1,
        "only the explicit host mounting hole is exported"
    );
    let preview: Value = serde_json::from_str(&artifact::request(
        &json!({"id":"preview","kind":"preview-board","revision":0,"source":board}).to_string(),
    ))
    .unwrap();
    assert_eq!(preview["kind"], "preview-board", "{preview}");
    assert!(
        preview["result"]["holes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|hole| {
                let points = hole.as_array().unwrap();
                let min_x = points
                    .iter()
                    .map(|p| p["x"].as_f64().unwrap())
                    .fold(f64::INFINITY, f64::min);
                let max_x = points
                    .iter()
                    .map(|p| p["x"].as_f64().unwrap())
                    .fold(f64::NEG_INFINITY, f64::max);
                let min_y = points
                    .iter()
                    .map(|p| p["y"].as_f64().unwrap())
                    .fold(f64::INFINITY, f64::min);
                let max_y = points
                    .iter()
                    .map(|p| p["y"].as_f64().unwrap())
                    .fold(f64::NEG_INFINITY, f64::max);
                (min_x - 23.6).abs() < 0.01
                    && (max_x - 26.4).abs() < 0.01
                    && (min_y + 11.4).abs() < 0.01
                    && (max_y + 8.6).abs() < 0.01
            }),
        "{preview}"
    );
}

#[test]
fn board_attached_supports_contact_below_host_with_back_facing_module() {
    let mut doc = document();
    doc["modules"][0]["hostFace"] = json!("back");
    doc["modules"][0]["facingFace"] = json!("back");
    doc["modules"][0]["mountSupports"][0]["z"] = json!(-3.8);

    let resolved = request(
        &mut CoreEngine::new(),
        json!({"id":"case","kind":"resolve-mechanical","document":doc,"contours":host_outline()}),
    );
    assert_eq!(resolved["kind"], "mechanical-resolved", "{resolved}");
    let assembly = &resolved["assembly"];
    assert_eq!(
        assembly["generationBlocked"], false,
        "{}",
        assembly["diagnostics"]
    );
    let standoff = assembly["case"]["bodies"]
        .as_array()
        .unwrap()
        .iter()
        .find(|body| body["body"]["id"] == "module-standoff/module-1/mh1")
        .expect("a separate below-board standoff body");
    assert!((standoff["body"]["z"].as_f64().unwrap() + 4.6).abs() < 1e-8);
    assert!((standoff["body"]["thickness"].as_f64().unwrap() - 3.0).abs() < 1e-8);
    let drilled_hole = assembly["pcbReference"]["contours"]
        .as_array()
        .unwrap()
        .iter()
        .find(|contour| {
            contour["hole"] == true && {
                let points = contour["points"].as_array().unwrap();
                let x = points.iter().map(|p| p["x"].as_f64().unwrap()).sum::<f64>()
                    / points.len() as f64;
                let y = points.iter().map(|p| p["y"].as_f64().unwrap()).sum::<f64>()
                    / points.len() as f64;
                (x - 25.0).abs() < 0.01 && (y + 10.0).abs() < 0.01
            }
        })
        .expect("the below-module mount also becomes a host PCB drill");
    let points = drilled_hole["points"].as_array().unwrap();
    let max_y = points
        .iter()
        .map(|p| p["y"].as_f64().unwrap())
        .fold(f64::NEG_INFINITY, f64::max);
    assert!((max_y + 8.6).abs() < 0.05);
}

#[test]
fn invalid_board_standoff_endpoint_is_blocked_without_guessing_a_host_drill() {
    let mut doc = document();
    doc["modules"][0]["mountSupports"][0]["z"] = json!(0.7);
    let module_scene = request(
        &mut CoreEngine::new(),
        json!({"id":"modules","kind":"resolve-modules","document":doc.clone(),"boardId":"host"}),
    );
    assert!(
        module_scene["result"]["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|finding| {
                finding["id"] == "module/module-1/host-drill"
                    && finding["scope"] == "pcb"
                    && finding["severity"] == "error"
            }),
        "{}",
        module_scene["result"]["findings"]
    );
    let resolved = request(
        &mut CoreEngine::new(),
        json!({"id":"case","kind":"resolve-mechanical","document":doc,"contours":host_outline()}),
    );
    assert_eq!(resolved["kind"], "mechanical-resolved", "{resolved}");
    let assembly = &resolved["assembly"];
    assert_eq!(
        assembly["generationBlocked"], true,
        "{}",
        assembly["diagnostics"]
    );
    assert!(
        assembly["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|finding| {
                finding["id"] == "mechanical:module/module-1/mount-support"
                    && finding["message"]
                        .as_str()
                        .unwrap()
                        .contains("span the selected module-to-host gap")
            }),
        "{}",
        assembly["diagnostics"]
    );
    assert!(
        !assembly["case"]["bodies"]
            .as_array()
            .unwrap()
            .iter()
            .any(|body| { body["body"]["id"] == "module-standoff/module-1/mh1" })
    );
    assert!(
        !assembly["pcbReference"]["contours"]
            .as_array()
            .unwrap()
            .iter()
            .any(|contour| {
                contour["hole"] == true
                    && contour["points"].as_array().unwrap().iter().any(|point| {
                        ((point["x"].as_f64().unwrap() - 25.0).powi(2)
                            + (point["y"].as_f64().unwrap() + 10.0).powi(2))
                        .sqrt()
                            < 1.5
                    })
            })
    );
}

#[test]
fn missing_board_mount_configuration_blocks_mechanical_and_pcb_fabrication() {
    let mut doc = document();
    doc["modules"][0]["mountSupports"] = json!([]);
    let resolved = request(
        &mut CoreEngine::new(),
        json!({"id":"case","kind":"resolve-mechanical","document":doc.clone(),"contours":host_outline()}),
    );
    assert_eq!(resolved["kind"], "mechanical-resolved", "{resolved}");
    let assembly = &resolved["assembly"];
    assert_eq!(
        assembly["generationBlocked"], true,
        "{}",
        assembly["diagnostics"]
    );
    assert!(
        assembly["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|finding| {
                finding["id"] == "module/module-1/board-support/mh1" && finding["scope"] == "pcb"
            }),
        "{}",
        assembly["diagnostics"]
    );
    assert!(
        !assembly["case"]["bodies"]
            .as_array()
            .unwrap()
            .iter()
            .any(|body| { body["body"]["id"] == "module-standoff/module-1/mh1" })
    );

    let project: ProjectDoc = serde_json::from_value(doc).unwrap();
    let plan = artifact::kicad::prepare_export(PrepareExportRequest {
        snapshot_token: "missing-module-support".into(),
        expected_revision: 0,
        document: project,
        target: ExportTarget::Board {
            board_id: "host".into(),
        },
        contours: host_outline(),
        model_paths: Default::default(),
    })
    .unwrap();
    let preview = finish_board_preview(&plan);
    assert_eq!(preview["kind"], "preview-board", "{preview}");
    assert!(
        preview["result"]["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|diagnostic| {
                diagnostic
                    .as_str()
                    .unwrap()
                    .contains("omits 1 unconfigured source mounting drill")
            }),
        "{}",
        preview["result"]["diagnostics"]
    );
    assert!(
        preview["result"]["holes"].as_array().unwrap().is_empty(),
        "preview must omit the nonexistent host drill"
    );
    let error = artifact::kicad::finish_export(FinishExportRequest {
        plan,
        results: vec![],
    })
    .expect_err("a source-mounted board module cannot silently omit its host drills");
    assert!(error.message.contains("Every source mount"), "{error:?}");
}

#[test]
fn partial_board_mount_configuration_blocks_pcb_fabrication() {
    let mut doc = document();
    doc["moduleDefinitions"][0]["mounts"] = json!([
        {"sourceId":"mh1","at":{"x":5,"y":-5},"diameter":2.2},
        {"sourceId":"mh2","at":{"x":-5,"y":5},"diameter":2.2}
    ]);
    let project: ProjectDoc = serde_json::from_value(doc).unwrap();
    let plan = artifact::kicad::prepare_export(PrepareExportRequest {
        snapshot_token: "partial-module-support".into(),
        expected_revision: 0,
        document: project,
        target: ExportTarget::Board {
            board_id: "host".into(),
        },
        contours: host_outline(),
        model_paths: Default::default(),
    })
    .unwrap();
    let preview = finish_board_preview(&plan);
    assert_eq!(preview["kind"], "preview-board", "{preview}");
    assert!(
        preview["result"]["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|diagnostic| {
                diagnostic
                    .as_str()
                    .unwrap()
                    .contains("omits 1 unconfigured source mounting drill")
            }),
        "{}",
        preview["result"]["diagnostics"]
    );
    assert_eq!(
        preview["result"]["holes"].as_array().unwrap().len(),
        1,
        "preview includes its configured host drill and omits the missing one"
    );
    let error = artifact::kicad::finish_export(FinishExportRequest {
        plan,
        results: vec![],
    })
    .expect_err("a source-mounted board module cannot export a partial host-drill pattern");
    assert!(error.message.contains("Every source mount"), "{error:?}");
}

fn finish_board_preview(plan: &ExportPlan) -> Value {
    serde_json::from_str(&artifact::request(
        &json!({
            "id":"finish-preview",
            "kind":"finish-preview",
            "request":{"plan":plan,"results":[]}
        })
        .to_string(),
    ))
    .unwrap()
}

#[test]
fn pinned_splitter_edge_mounts_keep_standoffs_with_partial_contact_warning() {
    let source = include_str!(
        "../../catalogue/modules/sources/sadekbaroudi-vik/pcb/vik-splitter/vik-splitter.kicad_pcb"
    );
    let imported: Value = serde_json::from_str(&artifact::request(&json!({
        "id":"import","kind":"import-module-board","definitionId":"vik:splitter","name":"VIK splitter",
        "source":source,
        "provenance":{"repository":"https://github.com/sadekbaroudi/vik","revision":"cd5d16e4cd9137a229fc673412a89d75f4e64553","path":"pcb/vik-splitter","license":"CC-BY-SA-4.0"},
        "family":"expansion","variant":"source population"
    }).to_string())).unwrap();
    assert_eq!(imported["kind"], "import-module-board", "{imported}");
    let mut definition = imported["result"].clone();
    definition["id"] = json!("vik:splitter");
    let mounts = definition["mounts"].as_array().unwrap();
    assert_eq!(mounts.len(), 2);
    let supports = mounts
        .iter()
        .map(|mount| {
            json!({
                "mountId":mount["sourceId"],"outerDiameter":6,"holeDiameter":2.8,"z":-3.8,"height":3
            })
        })
        .collect::<Vec<_>>();

    let mut doc = document();
    doc["moduleDefinitions"] = json!([definition]);
    doc["modules"] = json!([{
        "id":"splitter","definitionId":"vik:splitter","hostBoardId":"host","hostFace":"front","facingFace":"back",
        "at":{"x":29,"y":20},"rotation":0,"gap":3,"attachment":"board","mountSupports":supports
    }]);
    let mut outline = host_outline();
    outline[0].points = vec![
        Vec2 { x: -60.0, y: -60.0 },
        Vec2 { x: 60.0, y: -60.0 },
        Vec2 { x: 60.0, y: 60.0 },
        Vec2 { x: -60.0, y: 60.0 },
    ];
    let resolved = request(
        &mut CoreEngine::new(),
        json!({"id":"case","kind":"resolve-mechanical","document":doc,"contours":outline}),
    );
    assert_eq!(resolved["kind"], "mechanical-resolved", "{resolved}");
    let assembly = &resolved["assembly"];
    let support_findings = assembly["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|finding| {
            finding["id"]
                .as_str()
                .unwrap_or("")
                .contains("board-support")
        })
        .collect::<Vec<_>>();
    assert_eq!(
        support_findings.len(),
        2,
        "each edge mount should disclose partial contact: {support_findings:?}"
    );
    assert!(
        support_findings
            .iter()
            .all(|finding| finding["severity"] == "warning"),
        "{support_findings:?}"
    );
    assert!(
        support_findings.iter().all(|finding| finding["message"]
            .as_str()
            .unwrap()
            .contains("overhangs the module PCB")),
        "{support_findings:?}"
    );
    let bodies = assembly["case"]["bodies"].as_array().unwrap();
    assert_eq!(
        bodies
            .iter()
            .filter(|body| body["body"]["id"]
                .as_str()
                .unwrap_or("")
                .starts_with("module-standoff/splitter/"))
            .count(),
        2
    );
    assert!(
        assembly["pcbReference"]["contours"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|contour| contour["hole"] == true)
            .count()
            >= 2,
        "host NPTH geometry is retained for both source mounts"
    );
    assert!(
        assembly["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|finding| finding["id"] == "mechanical:module/splitter/gate/assembled-envelope"),
        "source assembly qualification gates remain intact"
    );
}

#[test]
fn board_standoff_without_module_pcb_contact_is_blocked() {
    let mut doc = document();
    doc["moduleDefinitions"][0]["mounts"][0]["at"]["x"] = json!(20.0);
    doc["modules"][0]["at"] = json!({"x":-20.0,"y":5.0});
    let resolved = request(
        &mut CoreEngine::new(),
        json!({"id":"case","kind":"resolve-mechanical","document":doc,"contours":host_outline()}),
    );
    assert_eq!(resolved["kind"], "mechanical-resolved", "{resolved}");
    let assembly = &resolved["assembly"];
    assert!(
        assembly["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|finding| {
                finding["id"] == "module/module-1/board-support/mh1"
                    && finding["severity"] == "error"
                    && finding["message"]
                        .as_str()
                        .unwrap()
                        .contains("no meaningful contact")
            }),
        "{}",
        assembly["diagnostics"]
    );
    assert!(
        !assembly["case"]["bodies"]
            .as_array()
            .unwrap()
            .iter()
            .any(|body| { body["body"]["id"] == "module-standoff/module-1/mh1" })
    );
}
