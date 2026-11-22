use super::*;

pub fn core_requests() -> Vec<Value> {
    let matrix = matrix();
    let with = |overrides: Value| {
        let mut copy = matrix.clone();
        for (key, value) in overrides.as_object().unwrap() {
            copy[key] = value.clone();
        }
        copy
    };
    let mut drafts = Vec::new();
    for mirror in ["none", "x", "y"] {
        for (index, origin) in [json!({ "x": 0, "y": 0 }), json!({ "x": 42, "y": -17 })]
            .into_iter()
            .enumerate()
        {
            drafts.push(with(json!({ "id": format!("draft-{mirror}-{index}"), "origin": origin, "mirror": mirror, "rotation": 37 })));
        }
    }
    let mut requests = vec![
        json!({ "id": "open", "kind": "open", "document": document() }),
        json!({ "id": "snapshot", "kind": "snapshot" }),
        json!({ "id": "draft", "kind": "project-matrices", "baseRevision": 0, "matrices": drafts }),
        json!({ "id": "invalid-draft", "kind": "project-matrices", "baseRevision": 0, "matrices": [with(json!({ "rows": 0 }))] }),
        json!({ "id": "preview", "kind": "edit", "command": move_command("preview") }),
        json!({ "id": "unchanged", "kind": "snapshot" }),
        json!({ "id": "commit", "kind": "edit", "command": move_command("commit") }),
        json!({ "id": "stale-draft", "kind": "project-matrices", "baseRevision": 0, "matrices": [matrix.clone()] }),
        json!({ "id": "undo", "kind": "undo" }),
        json!({ "id": "redo", "kind": "redo" }),
        json!({ "id": "large-draft", "kind": "project-matrices", "baseRevision": 3, "matrices": [with(json!({ "rows": 50, "columns": 10, "cells": [], "partIds": [] }))] }),
    ];
    for phase in ["preview", "commit"] {
        requests.push(json!({
            "id": format!("splay-{phase}"), "kind": "edit",
            "command": { "baseRevision": 3, "transactionId": "splay", "phase": phase, "targetIds": ["matrix"],
                "operation": { "kind": "set-matrix-splay", "matrixId": "matrix", "column": 1, "change": { "kind": "origin", "world": { "x": 5, "y": -12 } } } },
        }));
    }
    for (index, operation) in [
        json!({ "kind": "set-key-binding", "boardId": "board", "keyId": "matrix/matrix/r0c0", "binding": "&kp A" }),
        json!({ "kind": "set-matrix-keycaps", "matrixId": "matrix", "change": { "kind": "profile", "value": "dsa" } }),
        json!({ "kind": "set-keycap-board", "boardId": "board", "change": { "kind": "color", "value": "#123456" } }),
        json!({ "kind": "set-keycap-key", "keyId": "matrix/matrix/r0c0", "change": { "kind": "legend", "value": "" } }),
    ].into_iter().enumerate() {
        requests.push(json!({
            "id": format!("keymap-{index}"), "kind": "edit",
            "command": { "baseRevision": 4 + index, "transactionId": format!("keymap-{index}"), "phase": "commit", "targetIds": [], "operation": operation },
        }));
    }
    // Named placement changes cross the native/WASM seam for every variant.
    let mut revision = 8;
    for (name, change) in [
        (
            "stagger",
            json!({ "kind": "column-stagger", "column": 2, "value": 4.5 }),
        ),
        (
            "column-offset",
            json!({ "kind": "column-offset", "column": 1, "value": { "x": 4, "y": -2 } }),
        ),
        (
            "row-offset",
            json!({ "kind": "row-offset", "row": 0, "value": { "x": 3, "y": 5 } }),
        ),
    ] {
        for phase in ["preview", "commit"] {
            requests.push(json!({
                "id": format!("placement-{name}-{phase}"), "kind": "edit",
                "command": { "baseRevision": revision, "transactionId": name, "phase": phase, "targetIds": ["matrix"],
                    "operation": { "kind": "set-matrix-placement", "matrixId": "matrix", "change": change.clone() } },
            }));
        }
        revision += 1;
    }
    // Generated companions cross the seam with each placement mode.
    requests.push(json!({
        "id": "companion-matrix", "kind": "edit",
        "command": { "baseRevision": revision, "transactionId": "companion", "phase": "commit", "targetIds": ["matrix"],
            "operation": { "kind": "set-matrix",
                "matrix": with(json!({
                    "id": "matrix",
                    "cells": [{ "row": 0, "column": 0, "enabled": true,
                        "assemblies": [
                            { "id": "diode", "definitionId": "diode", "offset": { "x": 2, "y": 5 } },
                            { "id": "led", "definitionId": "led", "offset": { "x": -3, "y": 6 } },
                            { "id": "sensor", "definitionId": "sensor", "offset": { "x": 1, "y": -4 } }
                        ] }],
                    "partIds": [],
                })),
                "definitions": [
                    { "id": "diode", "name": "Diode", "kind": "passive", "pads": [], "courtyard": [] },
                    { "id": "led", "name": "LED", "kind": "passive", "pads": [], "courtyard": [] },
                    { "id": "sensor", "name": "Sensor", "kind": "passive", "pads": [], "courtyard": [] }
                ] } },
    }));
    requests.push(json!({
        "id": "companion-constraint", "kind": "edit",
        "command": { "baseRevision": revision + 1, "transactionId": "companion-constraint", "phase": "commit", "targetIds": [],
            "operation": { "kind": "set-constraint",
                "constraint": { "kind": "offset", "id": "sensor-key", "sourcePartId": "matrix/matrix/r0c0",
                    "targetPartId": "matrix/matrix/r0c0/sensor", "offset": { "x": 1, "y": -4 }, "rotation": 0 } } },
    }));
    requests.push(json!({
        "id": "companion-author", "kind": "edit",
        "command": { "baseRevision": revision + 2, "transactionId": "companion-author", "phase": "commit", "targetIds": [],
            "operation": { "kind": "move-parts", "positions": [{ "id": "matrix/matrix/r0c0/led", "at": { "x": -10, "y": 8 } }] } },
    }));
    requests.push(json!({ "id": "companion-snapshot", "kind": "snapshot" }));
    // A typed mechanical edit crosses the native/WASM seam carrying its target,
    // its typed change and the normalized mounting-hole catalogue definition the
    // generated closure clearance is projected from. `Enable` creates the
    // canonical configuration this project does not have yet; the closure
    // request then adds two suggested closure mounts and projects their
    // generated clearance; the identical repeat is a no-op with no revision
    // step; one Undo removes the mounts and every generated part together.
    requests.push(json!({
        "id": "mechanical-enable", "kind": "edit",
        "command": { "baseRevision": 14, "transactionId": "mechanical-enable", "phase": "commit", "targetIds": ["board"],
            "operation": mechanical_operation(json!("enable")) },
    }));
    let closure = json!({ "add-suggested-closure-mounts": [
        { "id": "left-front", "at": { "x": -30, "y": 25 }, "kind": "hole", "holeDiameter": 2.2 },
        { "id": "left-back", "at": { "x": 12.5, "y": -7.25 }, "kind": "hole", "holeDiameter": 2.2 },
    ] });
    for phase in ["preview", "commit"] {
        requests.push(json!({
            "id": format!("mechanical-closure-{phase}"), "kind": "edit",
            "command": { "baseRevision": 15, "transactionId": "mechanical-closure", "phase": phase, "targetIds": ["board"],
                "operation": mechanical_operation(closure.clone()) },
        }));
    }
    requests.push(json!({
        "id": "mechanical-closure-repeat", "kind": "edit",
        "command": { "baseRevision": 16, "transactionId": "mechanical-closure", "phase": "commit", "targetIds": ["board"],
            "operation": mechanical_operation(closure.clone()) },
    }));
    requests.push(json!({ "id": "mechanical-undo", "kind": "undo" }));
    requests.extend(outline_requests());
    // One-shot conversion crosses the seam last, so earlier requests keep their
    // revisions. Column 0 has zero Splay, Column rotation and carried Bend, as
    // spec §6 requires for conversion; column 2 stays splayed for the refusal.
    let placement = |id: &str, base: u64, phase: &str, change: Value| {
        json!({
            "id": id, "kind": "edit",
            "command": { "baseRevision": base, "transactionId": id, "phase": phase, "targetIds": ["matrix"],
                "operation": { "kind": "set-matrix-placement", "matrixId": "matrix", "change": change } },
        })
    };
    let convert = |column: u64| json!({ "kind": "column-offset-to-stagger", "column": column });
    requests.push(placement(
        "convert-offset-1",
        42,
        "commit",
        json!({ "kind": "column-offset", "column": 0, "value": { "x": 0, "y": 3 } }),
    ));
    requests.push(placement("convert-preview", 43, "preview", convert(0)));
    requests.push(placement("convert-commit", 43, "commit", convert(0)));
    requests.push(placement(
        "convert-offset-2",
        44,
        "commit",
        json!({ "kind": "column-offset", "column": 2, "value": { "x": 0, "y": 1 } }),
    ));
    requests.push(placement("convert-refused", 45, "commit", convert(2)));
    // The mirrored-half operation crosses the seam in both modes. This fixture has no physical
    // hardware, so each request is refused: the wire shape and the refusal are what is recorded.
    for (id, mode) in [
        (
            "mirrored-half-new-board",
            json!({ "kind": "new-board", "boardId": "mirror", "instanceId": "mirror" }),
        ),
        (
            "mirrored-half-reuse-board",
            json!({ "kind": "reuse-board", "instanceId": "flipped" }),
        ),
    ] {
        requests.push(json!({
            "id": id, "kind": "edit",
            "command": { "baseRevision": 45, "transactionId": id, "phase": "commit", "targetIds": [],
                "operation": { "kind": "create-mirrored-half", "pairId": "pair", "sourceInstanceId": "board",
                    "mode": mode, "mapping": { "at": { "x": 0, "y": 0 }, "rotation": 0 }, "idRule": "board-prefix" } },
        }));
    }
    requests
}
