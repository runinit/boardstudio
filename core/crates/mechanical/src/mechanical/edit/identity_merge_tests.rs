use super::*;
use serde_json::json;

fn configuration() -> MechanicalConfiguration {
    serde_json::from_value(json!({
        "boardId":"board","method":"printed","mount":"rigid",
        "plateThickness":1.5,"plateFoamThickness":0.5,"pcbThickness":1.6,
        "bottomFoamThickness":0.4,"batteryHeight":4.0,"bottomThickness":3.0,
        "plateToPcb":3.5,"wallThickness":2.0,"clearance":0.3,
        "mounts":[],"closureMounts":[],"profiles":[]
    }))
    .unwrap()
}

#[test]
fn reservation_merging_refuses_an_exhausted_floor() {
    for (label, floor) in [
        (
            "mount",
            IdentityFloor {
                mount: u64::MAX,
                ..IdentityFloor::default()
            },
        ),
        (
            "vertex",
            IdentityFloor {
                vertex: u64::MAX,
                ..IdentityFloor::default()
            },
        ),
        (
            "critical fit",
            IdentityFloor {
                critical_fit: u64::MAX,
                ..IdentityFloor::default()
            },
        ),
        (
            "hardware",
            IdentityFloor {
                hardware: u64::MAX,
                ..IdentityFloor::default()
            },
        ),
    ] {
        let mut configuration = configuration();
        if label == "vertex" {
            configuration.openings = Some(vec![MechanicalOpening {
                id: "opening-1".into(),
                vertices: vec![],
                z: 0.0,
                height: 1.0,
                next_vertex_identity: 1,
            }]);
        }
        let error = merge_identity_floor(&mut configuration, floor).unwrap_err();
        assert!(error.contains("exhausted"), "{label}: {error}");
    }
}

#[test]
fn merging_an_already_exhausted_counter_is_a_no_op() {
    let mut configuration = configuration();
    configuration.next_mount_identity = u64::MAX;
    merge_identity_floor(
        &mut configuration,
        IdentityFloor {
            mount: u64::MAX,
            ..IdentityFloor::default()
        },
    )
    .unwrap();
    assert_eq!(configuration.next_mount_identity, u64::MAX);
}
