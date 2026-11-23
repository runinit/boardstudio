use super::*;

pub fn derived_mirror_case(core: CoreTransport<'_>, archive: ArchiveTransport<'_>) -> Value {
    let mut call = |request: Value| {
        serde_json::from_str::<Value>(&core(&request.to_string())).expect("Core reply")
    };
    let mut doc = serde_json::to_value(ProjectDoc::empty("mirror-boundary", "Mirror")).unwrap();
    doc["definitions"] =
        json!([{"id":"switch","name":"Switch","kind":"switch","pads":[],"courtyard":[]}]);
    doc["parts"] = json!([{"id":"authored","reference":"SW1","definitionId":"switch",
        "side":"front","pose":{"at":{"x":2,"y":3},"rotation":0}}]);
    doc["boards"] = json!([
        {"id":"source","name":"Source","partIds":["authored"],"netIds":[],"outlineIds":[],"thickness":1.6},
        {"id":"target","name":"Target","partIds":[],"netIds":[],"outlineIds":[],"thickness":1.6}
    ]);
    doc["mirroredPairs"] = json!([{"id":"pair","source":{"kind":"secondBoard","id":"source"},
        "destination":{"kind":"secondBoard","id":"target"},
        "mapping":{"at":{"x":10,"y":0},"rotation":0},"overrides":[]}]);
    let open = call(json!({"id":"mirror-open","kind":"open","document":doc}));
    assert_eq!(open["kind"], "scene", "{open}");
    let mirror = "4/pair/part/authored";
    let transform = |reply: &Value| {
        let entries = reply["scene"]["transforms"].as_array().unwrap();
        entries.iter().find(|part| part["id"] == mirror).unwrap()["pose"]["at"]["x"].clone()
    };
    let edit = |phase, target| {
        json!({"id":"mirror-edit","kind":"edit","command":{
            "transactionId":"move","baseRevision":0,"phase":phase,"targetIds":[target],
            "operation":{"kind":"move-parts","positions":[{"id":target,"at":{"x":4,"y":3}}]}
        }})
    };
    let refusal = call(edit("commit", mirror));
    assert_eq!(refusal["kind"], "error");
    assert!(
        refusal["message"]
            .as_str()
            .unwrap()
            .contains("edit authored")
    );
    let preview = call(edit("preview", "authored"));
    assert_eq!(preview["kind"], "preview", "{preview}");
    let unchanged = call(json!({"id":"mirror-snapshot","kind":"snapshot"}));
    assert_eq!(unchanged["document"], open["document"]);
    let committed = call(edit("commit", "authored"));
    assert_eq!(committed["kind"], "scene", "{committed}");
    assert_eq!(committed["document"]["parts"].as_array().unwrap().len(), 1);
    let undo = call(json!({"id":"mirror-undo","kind":"undo"}));
    let redo = call(json!({"id":"mirror-redo","kind":"redo"}));
    assert_eq!(transform(&undo), 8.0);
    assert_eq!(transform(&redo), 6.0);

    let text = redo["document"].to_string();
    let (packed, buffers) = archive(
        &json!({"kind":"pack-project","projectJson":text,"assets":[]}),
        &[],
    );
    assert_eq!(packed["kind"], "packed", "{packed}");
    let (unpacked, _) = archive(&json!({"kind":"unpack-project"}), &buffers);
    assert_eq!(unpacked["projectJson"], text);
    let reopened = call(json!({"id":"mirror-reopen","kind":"open",
        "document":serde_json::from_str::<Value>(&text).unwrap()}));
    assert_eq!(reopened["document"], redo["document"]);
    json!({"previewX":transform(&preview),"undoX":transform(&undo),"redoX":transform(&redo),
        "reopenedX":transform(&reopened), "formatVersion":reopened["document"]["formatVersion"]})
}
