use super::{ConnectorAdmission, connector_admission};
use wasm_bindgen_test::wasm_bindgen_test;

#[wasm_bindgen_test]
fn connector_admission_reuses_local_host_and_rejects_foreign_automatic_ids() {
    let automatic_id = "module-a/vik-host-connector";

    assert_eq!(
        connector_admission("host-on-board", automatic_id, true, true),
        ConnectorAdmission::Reuse
    );
    assert_eq!(
        connector_admission(automatic_id, automatic_id, false, true),
        ConnectorAdmission::Reject
    );
    assert_eq!(
        connector_admission("wrong-role", automatic_id, false, true),
        ConnectorAdmission::Reject
    );
    assert_eq!(
        connector_admission("missing-host", automatic_id, false, false),
        ConnectorAdmission::Reject
    );
    assert_eq!(
        connector_admission(automatic_id, automatic_id, false, false),
        ConnectorAdmission::CreateAutomatic
    );
}
