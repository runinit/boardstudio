//! Product-context gestures on a generated outline: the panel's own controls must
//! land through real Session/Core, never a silent dead click.
use super::*;

/// Product-context gestures on a generated outline: the panel's own controls must land
/// through real Session/Core, never a silent dead click.
fn click_by_text(root: &web_sys::Element, text: &str) -> bool {
    if by_role_name(root, "button", text).is_none() {
        return false;
    }
    click_button(root, text);
    true
}

fn versions(runtime: &Rc<crate::runtime::Runtime>, board_id: &str) -> usize {
    runtime
        .model()
        .accepted
        .expect("accepted")
        .document
        .board_outlines
        .iter()
        .find(|state| state.board_id == board_id)
        .map_or(0, |state| state.versions.len())
}

fn alert_text(root: &web_sys::Element) -> Option<String> {
    root.query_selector("[role='alert']")
        .ok()
        .flatten()
        .and_then(|node| node.text_content())
}

#[wasm_bindgen_test]
async fn mounted_refine_outline_creates_a_linked_refinement_with_one_undo() {
    let (probe, root, _mounted) = mounted_polygon_outline_inspector(false).await;
    settle_dimension().await;
    assert!(
        click_by_text(&root, "Refine outline"),
        "the Refine outline control is reachable in the generated context; body: {}",
        root.inner_html()
    );
    accept(&probe).await;
    settle_dimension().await;
    let accepted = accepted_document(&probe);
    let state = accepted
        .board_outlines
        .iter()
        .find(|state| state.board_id == probe.scope.board_id)
        .expect("the refinement creates board outline state");
    assert_eq!(state.versions.len(), 1, "one action creates one refinement");
    let created = &state.versions[0];
    assert_eq!(
        created.ownership,
        boardstudio_model::model::OutlineOwnership::Linked,
        "accepting the alternative creates a linked refinement"
    );
    assert_eq!(
        state.active_version_id.as_deref(),
        Some(created.id.as_str()),
        "the created refinement is selected"
    );
    let labels = active_outline_option_labels(&root).await;
    assert!(
        labels
            .iter()
            .any(|label| label.starts_with("Linked refinement")),
        "the refinement is presented as linked: {labels:?}"
    );
    let name_input = by_role_name(&root, "textbox", "Outline version name")
        .expect("the version name control is mounted for the selected refinement")
        .dyn_into::<web_sys::HtmlInputElement>()
        .unwrap();
    assert!(
        !name_input.disabled(),
        "its settings stay editable while linked"
    );

    // One Undo removes the refinement and restores the Generated selection.
    undo(&probe.runtime).await;
    accept(&probe).await;
    settle_dimension().await;
    let undone = accepted_document(&probe);
    let state = undone
        .board_outlines
        .iter()
        .find(|state| state.board_id == probe.scope.board_id);
    assert!(
        state.is_none_or(|state| state.versions.is_empty()),
        "one Undo removes the refinement"
    );
    assert!(
        undone
            .board_outlines
            .iter()
            .all(|state| state.active_version_id.is_none()),
        "one Undo restores the Generated selection"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_copy_outline_on_a_generated_outline_creates_one_version() {
    let (probe, root, _mounted) = mounted_polygon_outline_inspector(false).await;
    settle_dimension().await;
    assert!(
        click_by_text(&root, "Copy outline"),
        "the Copy outline control is reachable"
    );
    accept(&probe).await;
    settle_dimension().await;
    let count = versions(&probe.runtime, &probe.scope.board_id);
    assert!(
        count == 1,
        "Copy outline lands one version (got {count}); alert={:?}",
        alert_text(&root)
    );
    root.remove();
}
