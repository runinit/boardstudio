use super::*;
use crate::presentation::test_mount_helpers::canvas::{
    column_at_origin, one_key_matrix, one_key_scene, scene, switch_document,
};
use crate::presentation::test_mount_helpers::{MountedRoot, mount};
use boardstudio_core::model::SceneDelta;
use std::sync::Arc;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::wasm_bindgen_test;

fn spacing_fixture() -> (Arc<ProjectDoc>, Arc<SceneDelta>, Arc<SceneDelta>) {
    let mut document = switch_document(
        "canvas",
        "Canvas",
        &["matrix/left/r0c0", "matrix/right/r0c0"],
    );
    document.matrices.extend([
        one_key_matrix("left", "matrix/left/r0c0", 0.0),
        one_key_matrix("right", "matrix/right/r0c0", 0.0),
    ]);
    let scene_with = |half_gap: f64| {
        scene(&[
            one_key_scene("left", -half_gap, 0.0, &[column_at_origin()]),
            one_key_scene("right", half_gap, 0.0, &[column_at_origin()]),
        ])
    };
    // Narrow spacing (keys at ±30) and the widened spacing (keys at ±35).
    (Arc::new(document), scene_with(30.0), scene_with(35.0))
}

#[derive(Props, Clone, PartialEq)]
struct SpacingCaseProps {
    document: Arc<ProjectDoc>,
    displayed: Arc<SceneDelta>,
    accepted: Option<Arc<SceneDelta>>,
    ghost: Option<SpacingGhost>,
    preview: bool,
}

#[component]
fn RenderSpacingCase(props: SpacingCaseProps) -> Element {
    let highlight = use_signal(|| None::<Vec<String>>);
    use_context_provider(|| super::super::RelationHighlight(highlight));
    rsx! {
        svg {
            Scene {
                source: SceneSource { document: props.document, scene: props.displayed, board_id: "board".into() },
                preview_active: props.preview,
                accepted_scene: props.accepted,
                spacing_ghost: props.ghost,
                selected_ids: Vec::new(), selected_context: None,
                view_width_world: 200.0, surface_width_px: 800.0,
                keys_visible: true, components_visible: true, keycaps_visible: true, footprints_visible: false,
                on_matrix_pointer_down: |_| {}, on_part_pointer_down: |_| {},
            }
        }
    }
}

async fn mount_case(
    displayed: Arc<SceneDelta>,
    accepted: Option<Arc<SceneDelta>>,
    ghost: Option<SpacingGhost>,
    preview: bool,
) -> MountedRoot {
    let (document, _, _) = spacing_fixture();
    let root = mount(
        VirtualDom::new_with_props(
            RenderSpacingCase,
            SpacingCaseProps {
                document,
                displayed,
                accepted,
                ghost,
                preview,
            },
        ),
        None,
    );
    gloo_timers::future::TimeoutFuture::new(5).await;
    root
}

fn ghost_transforms(root: &web_sys::Element) -> Vec<String> {
    let nodes = root.query_selector_all("[data-ghost='spacing']").unwrap();
    (0..nodes.length())
        .filter_map(|index| {
            nodes
                .item(index)?
                .dyn_into::<web_sys::SvgElement>()
                .ok()?
                .get_attribute("transform")
        })
        .collect()
}

/// RED: widening draws the previous (narrow) keys dashed with a +N note.
#[wasm_bindgen_test]
async fn widening_ghosts_the_narrow_keys_with_a_plus_note() {
    let (_, narrow, wide) = spacing_fixture();
    let root = mount_case(
        wide,
        None,
        Some(SpacingGhost {
            scene: narrow,
            delta_mm: 10.0,
            axis_x: 0.0,
        }),
        false,
    )
    .await;
    let transforms = ghost_transforms(&root);
    assert_eq!(
        transforms.len(),
        2,
        "both halves ghost at the previous spacing"
    );
    assert!(
        transforms
            .iter()
            .any(|transform| transform.contains("translate(-30 0)")),
        "the left ghost sits on its narrow pose, got {transforms:?}"
    );
    assert!(
        transforms
            .iter()
            .any(|transform| transform.contains("translate(30 0)")),
        "the right ghost sits on its narrow pose, got {transforms:?}"
    );
    let note = root
        .query_selector("[data-ghost-note='spacing']")
        .unwrap()
        .expect("the axis carries the widening delta");
    assert_eq!(note.text_content().as_deref(), Some("+10.0 mm"));
    // Ticket 02: the note sits a fixed 14 px screen gap above the top
    // key edge: 9.025 half key plus 3.5 world units at this zoom.
    assert!(
        note.get_attribute("transform")
            .as_deref()
            .unwrap_or_default()
            .contains("translate(0 12.5"),
        "the note sits on the axis above the keys"
    );
}

/// RED: narrowing draws the previous (wide) keys dashed with a −N note.
#[wasm_bindgen_test]
async fn narrowing_ghosts_the_wide_keys_with_a_minus_note() {
    let (_, narrow, wide) = spacing_fixture();
    let root = mount_case(
        narrow,
        None,
        Some(SpacingGhost {
            scene: wide,
            delta_mm: -6.0,
            axis_x: 0.0,
        }),
        false,
    )
    .await;
    let transforms = ghost_transforms(&root);
    assert_eq!(transforms.len(), 2, "both halves ghost at the wide poses");
    assert!(
        transforms
            .iter()
            .any(|transform| transform.contains("translate(-35 0)")),
        "the left ghost sits on its wide pose, got {transforms:?}"
    );
    let note = root
        .query_selector("[data-ghost-note='spacing']")
        .unwrap()
        .expect("the axis carries the narrowing delta");
    assert_eq!(note.text_content().as_deref(), Some("−6.0 mm"));
}

/// RED: a settled scene with no ghost prop draws no ghost elements.
#[wasm_bindgen_test]
async fn settled_scene_without_ghost_draws_nothing() {
    let (_, _, wide) = spacing_fixture();
    let root = mount_case(wide, None, None, false).await;
    assert!(
        root.query_selector_all("[data-ghost='spacing']")
            .unwrap()
            .length()
            == 0,
        "no ghost prop means no ghost rects"
    );
    assert!(
        root.query_selector("[data-ghost-note='spacing']")
            .unwrap()
            .is_none(),
        "no ghost prop means no axis note"
    );
}

/// RED: a drag preview keeps the amber preview ghosts and hides the
/// spacing ghost and its note.
#[wasm_bindgen_test]
async fn drag_preview_preempts_the_spacing_ghost() {
    let (_, narrow, wide) = spacing_fixture();
    let root = mount_case(
        wide,
        Some(narrow.clone()),
        Some(SpacingGhost {
            scene: narrow,
            delta_mm: 10.0,
            axis_x: 0.0,
        }),
        true,
    )
    .await;
    assert!(
        root.query_selector("[data-ghost='scene']")
            .unwrap()
            .is_some(),
        "the preview still ghosts the pre-drag pose"
    );
    assert!(
        root.query_selector_all("[data-ghost='spacing']")
            .unwrap()
            .length()
            == 0,
        "the spacing ghost hides while a preview is up"
    );
    assert!(
        root.query_selector("[data-ghost-note='spacing']")
            .unwrap()
            .is_none(),
        "the axis note hides while a preview is up"
    );
}
