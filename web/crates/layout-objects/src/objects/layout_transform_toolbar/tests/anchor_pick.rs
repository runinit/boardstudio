//! Ticket 08: the canvas Anchor pick mode for anchored matrices and independent parts.
use super::*;
use fixture::undo;

fn pick_token(root: &web_sys::Element) -> Option<web_sys::Element> {
    query(root, "[data-pick=\"anchor\"]")
}

fn pick_ring_values(root: &web_sys::Element) -> Vec<String> {
    let mut values = query_all(root, "[data-anchor]")
        .iter()
        .map(|ring| ring.get_attribute("data-anchor").unwrap_or_default())
        .collect::<Vec<_>>();
    values.sort();
    values
}

fn pick_ring(root: &web_sys::Element, anchor: &str) -> web_sys::Element {
    query(root, &format!("[data-anchor=\"{anchor}\"]"))
        .unwrap_or_else(|| panic!("the {anchor} ring draws"))
}

fn ring_circle_radii(ring: &web_sys::Element) -> Vec<String> {
    query_all(ring, "circle")
        .iter()
        .map(|circle| circle.get_attribute("r").unwrap_or_default())
        .collect()
}

fn amber_previews(root: &web_sys::Element) -> usize {
    query_all(root, "[data-anchor-pick-preview]").len()
}

const LAYOUT_KEYS: [&str; 3] = ["matrix:0:0", "matrix:0:1", "matrix:0:2"];

/// The current anchor fills a white dot inside the 4.5 px ring.
const CURRENT_RING: [&str; 3] = ["12", "2.25", "1.6"];

async fn arm_pick_via_token(root: &web_sys::Element) {
    let token = pick_token(root).expect("the Anchor token draws");
    dispatch_pointer(&token, "pointerdown", false, false, 1, (50, 50));
    gloo_timers::future::TimeoutFuture::new(80).await;
}

/// Click the `anchor` ring and drive the runtime until the pick lands as one revision.
async fn pick(runtime: &Rc<Runtime>, root: &web_sys::Element, anchor: &str) {
    let before = revision(runtime);
    dispatch_pointer(
        &pick_ring(root, anchor),
        "pointerdown",
        false,
        false,
        1,
        (60, 60),
    );
    pump_past(runtime, before).await;
    assert_eq!(
        revision(runtime),
        before + 1,
        "the pick lands as one revision"
    );
}

fn matrix_anchor_target(runtime: &Rc<Runtime>, matrix_id: &str) -> (String, u32, u32) {
    let anchor = fixture::matrix_named(runtime, matrix_id)
        .anchor
        .expect("still anchored");
    (anchor.matrix_id, anchor.row, anchor.column)
}

/// Ticket 08: arming the anchored matrix lists its committable keys in variant-D ring sizes,
/// marks the current anchor, and clicking a ring commits one Undo entry with the carried
/// toast; Undo restores.
#[wasm_bindgen_test]
async fn matrix_pick_commits_one_undo_entry_with_the_carried_toast() {
    let runtime = open_anchored_drag_runtime(full_follow()).await;
    let (root, _mounted) = mount_pick(&runtime, matrix_pick_owner(&runtime, "dep")).await;
    assert!(
        pick_token(&root).is_some(),
        "the anchored matrix draws the Anchor token"
    );
    assert!(
        pick_ring_values(&root).is_empty(),
        "rings wait for the token press"
    );
    arm_pick_via_token(&root).await;
    assert_eq!(
        pick_ring_values(&root),
        LAYOUT_KEYS,
        "the dep lists the main matrix's keys"
    );
    assert!(
        query(&root, "[data-anchor=\"dep:0:0\"]").is_none(),
        "the dep's own key draws no ring"
    );
    assert_eq!(
        ring_circle_radii(&pick_ring(&root, "matrix:0:0")),
        CURRENT_RING
    );
    assert!(
        query(&root, "[data-feedback-popup]").is_some(),
        "the anchor popup pins while armed"
    );
    // Hovering previews the Core-evaluated post-commit pose in amber, on a ring grown to 6 px.
    dispatch_pointer(
        &pick_ring(&root, "matrix:0:2"),
        "pointerenter",
        false,
        false,
        0,
        (60, 60),
    );
    wait_until(
        "hover shows the enlarged anchor ring and amber preview",
        || {
            ring_circle_radii(&pick_ring(&root, "matrix:0:2"))[1] == "3"
                && amber_previews(&root) > 0
        },
    )
    .await;
    pick(&runtime, &root, "matrix:0:2").await;
    assert_eq!(
        matrix_anchor_target(&runtime, "dep"),
        ("matrix".into(), 0, 2),
        "the pick retargets the anchor"
    );
    assert!(
        pick_ring_values(&root).is_empty(),
        "committing leaves pick mode"
    );
    let text = toast_text(&runtime, &root).await;
    assert!(
        text.contains("Dependent anchored to matrix R1C3")
            && text.contains("(was matrix R1C1)")
            && text.contains("1 Undo step"),
        "the toast carries the item and both anchors, got {text:?}"
    );
    undo(&runtime).await;
    assert_eq!(
        matrix_anchor_target(&runtime, "dep"),
        ("matrix".into(), 0, 0),
        "Undo restores the anchor"
    );
    root.remove();
}

/// Ticket 08: a matrix whose anchor chain reaches the owner draws no ring: re-anchoring there
/// would close a loop.
#[wasm_bindgen_test]
async fn matrix_pick_hides_loop_forming_targets() {
    let mut document = anchored_drag_document(full_follow());
    document
        .layouts
        .iter_mut()
        .find(|layout| layout.id == "layout")
        .expect("the pick layout")
        .matrix_ids
        .push("looped".into());
    add_single_key_matrix(&mut document, "looped", "K9", Vec2 { x: 0.0, y: -38.1 });
    let looped = document.matrices.last_mut().expect("the looped matrix");
    looped.name = Some("Looped".into());
    looped.column_layout = vec![Default::default()];
    looped.anchor = Some(fixture::key_anchor("dep", true));
    let runtime = support::new_runtime();
    support::open_document(&runtime, document).await;
    wait_until_editable(&runtime).await;
    let (root, _mounted) = mount_pick(&runtime, matrix_pick_owner(&runtime, "dep")).await;
    arm_pick_via_token(&root).await;
    assert_eq!(
        pick_ring_values(&root),
        LAYOUT_KEYS,
        "only the loop-free keys draw rings"
    );
    assert!(
        query(&root, "[data-anchor=\"looped:0:0\"]").is_none(),
        "the loop-forming matrix draws no ring"
    );
    root.remove();
}

/// Ticket 08: Escape leaves pick mode with no revision and no toast.
#[wasm_bindgen_test]
async fn matrix_pick_escape_leaves_no_revision() {
    let runtime = open_anchored_drag_runtime(full_follow()).await;
    let (root, _mounted) = mount_pick(&runtime, matrix_pick_owner(&runtime, "dep")).await;
    let before = revision(&runtime);
    arm_pick_via_token(&root).await;
    assert!(
        !pick_ring_values(&root).is_empty(),
        "the token press arms pick mode"
    );
    fixture::press_key(&root, "Escape");
    gloo_timers::future::TimeoutFuture::new(80).await;
    assert!(
        pick_ring_values(&root).is_empty(),
        "Escape leaves pick mode"
    );
    assert_eq!(revision(&runtime), before, "Escape commits nothing");
    assert!(
        query(&root, "[data-feedback-toast]").is_none(),
        "Escape raises no toast"
    );
    root.remove();
}

/// Ticket 08: an unanchored matrix draws no Anchor token: there is no anchor to retarget.
#[wasm_bindgen_test]
async fn unanchored_matrix_draws_no_anchor_token() {
    let runtime = fixture::open_matrix_runtime().await;
    wait_until_editable(&runtime).await;
    let (root, _mounted) =
        mount_pick(&runtime, matrix_pick_owner(&runtime, fixture::MATRIX_ID)).await;
    assert!(
        pick_token(&root).is_none(),
        "an unanchored matrix offers no Anchor token"
    );
    root.remove();
}

fn part_pick_owner() -> AnchorPickOwner {
    AnchorPickOwner::Part {
        part_id: "mcu".into(),
    }
}

/// Ticket 08: the anchored part's rail ends in the Anchor token; picking a key moves the part
/// with its carried values and Undo restores.
#[wasm_bindgen_test]
async fn part_pick_commits_with_carried_values_and_undo_restores() {
    let runtime = fixture::open_part_runtime((5.0, 2.0)).await;
    wait_until_editable(&runtime).await;
    let (root, _mounted) = mount_pick(&runtime, part_pick_owner()).await;
    assert!(
        pick_token(&root).is_some(),
        "the part rail ends in the Anchor token"
    );
    arm_pick_via_token(&root).await;
    assert_eq!(
        pick_ring_values(&root),
        LAYOUT_KEYS,
        "the mcu lists its layout keys"
    );
    assert_eq!(
        ring_circle_radii(&pick_ring(&root, "matrix:0:1")),
        CURRENT_RING,
        "the part's current anchor fills a white dot"
    );
    dispatch_pointer(
        &pick_ring(&root, "matrix:0:0"),
        "pointerenter",
        false,
        false,
        0,
        (60, 60),
    );
    wait_until("hover previews where the part would move", || {
        amber_previews(&root) > 0
    })
    .await;
    pick(&runtime, &root, "matrix:0:0").await;
    let anchor = fixture::part_named(&runtime, "mcu")
        .anchor
        .expect("anchored");
    assert_eq!(
        (anchor.matrix_id.as_str(), anchor.row, anchor.column),
        ("matrix", 0, 0),
        "the pick retargets the part anchor"
    );
    assert_eq!(
        (anchor.spread, anchor.stagger),
        (5.0, 2.0),
        "the carried values ride along"
    );
    let text = toast_text(&runtime, &root).await;
    assert!(
        text.contains("U1 anchored to matrix R1C1")
            && text.contains("(was matrix R1C2)")
            && text.contains("1 Undo step"),
        "the toast carries the part and both anchors, got {text:?}"
    );
    undo(&runtime).await;
    let anchor = fixture::part_named(&runtime, "mcu")
        .anchor
        .expect("anchored");
    assert_eq!(anchor.column, 1, "Undo restores the part anchor");
    root.remove();
}

/// Ticket 08: a detached part arms pick with the half origin current and anchors from the
/// first ring click.
#[wasm_bindgen_test]
async fn detached_part_pick_anchors_from_the_half_origin() {
    let runtime = fixture::open_detached_part_runtime().await;
    wait_until_editable(&runtime).await;
    let (root, _mounted) = mount_pick(&runtime, part_pick_owner()).await;
    arm_pick_via_token(&root).await;
    assert_eq!(
        pick_ring_values(&root),
        LAYOUT_KEYS,
        "a detached part can anchor to any key"
    );
    assert!(
        query(&root, "[data-anchor-pick-major]").is_some(),
        "the half origin marks the current placement"
    );
    pick(&runtime, &root, "matrix:0:0").await;
    let anchor = fixture::part_named(&runtime, "mcu")
        .anchor
        .expect("the detached part anchors");
    assert_eq!(
        (anchor.matrix_id.as_str(), anchor.row, anchor.column),
        ("matrix", 0, 0)
    );
    // Drain the anchor-commit task before unmount: it writes the toast feedback after the
    // revision lands, and must finish while mounted.
    toast_text(&runtime, &root).await;
    root.remove();
}
