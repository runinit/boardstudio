#[path = "test_fixtures.rs"]
pub(super) mod fixtures;
use fixtures::*;
mod outlines_tests {
    include!("outlines/tests.rs");
}

#[test]
fn matrix_membership_compares_unordered_member_multiplicity_and_presence() {
    let before = fixture_document().unwrap();
    let id = &before.matrices[0].id;
    let members = &before.matrices[0].part_ids;
    let mut after = before.clone();
    after.matrices[0].part_ids.reverse();
    assert!(!super::matrix_membership_changed(&before, &after, id));
    for member in ["new-part", &members[0]] {
        let mut added = before.clone();
        added.matrices[0].part_ids.push(member.into());
        assert!(super::matrix_membership_changed(&before, &added, id));
        assert!(super::matrix_membership_changed(&added, &before, id));
    }
    after.matrices[0].part_ids = vec![members[0].clone(); members.len()];
    assert!(super::matrix_membership_changed(&before, &after, id));
    after.matrices[0].part_ids.clear();
    let mut missing = after.clone();
    missing.matrices.remove(0);
    assert!(super::matrix_membership_changed(&after, &missing, id));
    assert!(!super::matrix_membership_changed(&missing, &missing, id));
}

#[test]
fn document_diff_reports_idless_nested_binding_paths() {
    let mut before = fixture_document().expect("the fixture is valid");
    let mut keymap = boardstudio_core_mechanical::keymap::KeymapConfiguration::default();
    keymap.layers[0].bindings.insert(
        "matrix/mirror-matrix/r0c0".into(),
        boardstudio_core_mechanical::keymap::KeyBinding::KeyPress {
            keycode: "A".into(),
        },
    );
    before.keymap = Some(keymap.clone());
    let mut after = before.clone();
    keymap.layers[0].bindings.insert(
        "matrix/mirror-matrix/r0c0".into(),
        boardstudio_core_mechanical::keymap::KeyBinding::KeyPress {
            keycode: "B".into(),
        },
    );
    after.keymap = Some(keymap);
    assert!(
        super::changed_document_ids(&before, &after, &[])
            .unwrap()
            .is_empty()
    );
}
