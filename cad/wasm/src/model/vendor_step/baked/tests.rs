use super::*;
use sha2::{Digest, Sha256};

const PIN_SOCKET_1X4: &[u8] = include_bytes!(
    "../../../../../../ergogen/library/vendor/infused-kim/3d_models/PinSocket_2.54mm_5mm_1x-4.step"
);
const PIN_SOCKET_1X5: &[u8] = include_bytes!(
    "../../../../../../ergogen/library/vendor/infused-kim/3d_models/PinSocket_2.54mm_5mm_1x-5.step"
);
const PIN_SOCKET_2X12: &[u8] = include_bytes!(
    "../../../../../../ergogen/library/vendor/infused-kim/3d_models/PinSocket_2.54mm_5mm_2x-12.step"
);
const LEGACY_PROJECTED_LINE: &[u8] =
    include_bytes!("../../../../../monstertruck/tests/fixtures/repair-projected-line.step");
const CHOC_HOTSWAP: &[u8] = include_bytes!(
    "../../../../../../ergogen/library/vendor/infused-kim/3d_models/Choc_V1_Hotswap.step"
);

#[test]
fn baked_default_previews_preserve_retained_face_movement_and_metadata() {
    for (bytes, expected_faces, expects_movement) in [
        (PIN_SOCKET_1X4, 153, true),
        (PIN_SOCKET_1X5, 189, true),
        (PIN_SOCKET_2X12, 882, true),
        // This tetrahedron needs a projected LINE attachment on the legacy route.
        // That route keeps whole-asset repair unknown and retains no face measurement.
        (LEGACY_PROJECTED_LINE, 4, false),
        // Keep the metadata on a shipped legacy asset with relaxed precision and fallback faces.
        (CHOC_HOTSWAP, 482, false),
    ] {
        let sha = format!("{:x}", Sha256::digest(bytes));
        let live = ReadStepAsset::import_bytes(
            bytes.to_vec(),
            ReadStepOptions {
                source_sha256: sha.clone(),
                preview_repair_budget_mm: None,
                target_deflection_mm: None,
            },
        )
        .unwrap();
        let payload = encode(bytes.to_vec(), sha.clone()).unwrap();
        let baked = decode(&payload, &sha).unwrap();

        let qualification::PreparedAnalysisView::Vendor(live_view) = live.analysis.export_view()
        else {
            unreachable!();
        };
        let qualification::PreparedAnalysisView::Vendor(baked_view) = baked.analysis.export_view()
        else {
            unreachable!();
        };
        assert_eq!(live_view.face_occurrences.len(), expected_faces);
        assert_eq!(live_view.positions_mm_f64, baked_view.positions_mm_f64);
        assert_eq!(live_view.source_faces, baked_view.source_faces);
        assert_eq!(live_view.face_occurrences, baked_view.face_occurrences);
        assert_eq!(
            live_view.triangle_face_occurrence_indices,
            baked_view.triangle_face_occurrence_indices
        );
        let mut moved = false;
        for (live_face, baked_face) in live_view
            .face_occurrences
            .iter()
            .zip(baked_view.face_occurrences)
        {
            let movement = live_face.repair_displacement_mm;
            assert_eq!(
                movement.to_bits(),
                baked_face.repair_displacement_mm.to_bits()
            );
            moved |= movement > 0.0;
        }
        assert_eq!(moved, expects_movement, "{sha}");

        assert_eq!(baked.mesh.positions, live.mesh.positions);
        assert_eq!(baked.mesh.normals, live.mesh.normals);
        assert_eq!(baked.bounds.min, live.bounds.min);
        assert_eq!(baked.bounds.max, live.bounds.max);
        assert_eq!(baked.vendor_step.source_sha256, sha);
        // Compare serialized metadata in addition to the private analysis emission.
        assert_eq!(
            serde_json::to_vec(&baked.vendor_step).unwrap(),
            serde_json::to_vec(&live.vendor_step).unwrap()
        );
        assert_eq!(baked.vendor_step.accuracy.max_repair_displacement_mm, None);
        assert_eq!(baked.vendor_step.accuracy.analysis_error_bound_mm, None);
        assert!(matches!(
            baked.analysis.source_qualification(),
            SourceQualification::Unknown { .. }
        ));
        assert_eq!(
            baked.analysis.source_qualification(),
            live.analysis.source_qualification()
        );
        assert_eq!(
            baked.analysis.analysis_mesh().settings,
            live.analysis.analysis_mesh().settings
        );
    }
}

#[test]
fn baked_preview_rejects_invalid_movement_old_format_and_wrong_source() {
    let sha = format!("{:x}", Sha256::digest(PIN_SOCKET_1X4));
    let mut payload = encode(PIN_SOCKET_1X4.to_vec(), sha.clone()).unwrap();
    assert!(decode(&payload, &"0".repeat(64)).is_err());
    assert!(decode(&payload[..payload.len() - 1], &sha).is_err());
    let metadata_size = u32::from_le_bytes(payload[8..12].try_into().unwrap()) as usize;
    let first_movement = 12 + metadata_size;
    for movement in [-1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        payload[first_movement..first_movement + 8].copy_from_slice(&movement.to_le_bytes());
        assert!(decode(&payload, &sha).is_err(), "accepted {movement}");
    }
    payload[..8].copy_from_slice(b"BSSTEP01");
    payload.drain(first_movement..first_movement + 153 * 8);
    assert!(decode(&payload, &sha).is_err());
}
