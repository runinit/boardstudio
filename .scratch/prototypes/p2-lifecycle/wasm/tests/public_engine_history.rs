use p2_lifecycle_probe::engine::ProbeSession;

const FIXTURE: &str = include_str!("../../fixtures/reviung41.json");
const PART: &str = "matrix/main-right-keys/r0c0";

#[test]
fn public_engine_commits_one_drag_and_undoes_redoes_that_transaction() {
    let mut session = ProbeSession::open(FIXTURE).expect("copied fixture opens in public engine");
    let before = session
        .part_position(PART)
        .expect("selected fixture part exists");

    session
        .commit_drag(PART, (9.0, 6.0))
        .expect("public engine commits drag");
    let after = session
        .part_position(PART)
        .expect("part remains in snapshot");
    assert_eq!((after.0 - before.0, after.1 - before.1), (9.0, 6.0));

    session
        .undo()
        .expect("public engine undoes committed gesture");
    assert_eq!(session.part_position(PART), Some(before));

    session
        .redo()
        .expect("public engine redoes committed gesture");
    assert_eq!(session.part_position(PART), Some(after));
}
