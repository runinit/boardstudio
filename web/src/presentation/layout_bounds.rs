use super::*;

pub(crate) fn accepted_board_contours(
    snapshot: &AcceptedSnapshot,
    board_id: &str,
) -> Rc<[Contour]> {
    let contours = snapshot
        .scene
        .board_contours
        .iter()
        .find(|board| board.board_id == board_id)
        .map(|board| board.contours.as_slice())
        .unwrap_or_else(|| {
            if snapshot.document.boards.len() == 1 {
                snapshot.scene.contours.as_slice()
            } else {
                &[]
            }
        });
    Rc::<[Contour]>::from(contours.to_vec())
}

pub(crate) fn keymap_bounds(
    view: &keymap::KeymapView,
    contours: &[Contour],
) -> Option<(f64, f64, f64, f64)> {
    layout_camera::key_and_contour_bounds(
        view.keys.iter().map(|key| (key.pose, key.size)),
        contours,
    )
}

pub(crate) fn keymap_toolbar_height() -> f64 {
    web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| {
            document
                .query_selector("[role='group'][aria-label='Keymap commands']")
                .ok()
                .flatten()
        })
        .map(|toolbar| toolbar.get_bounding_client_rect().height())
        .filter(|height| height.is_finite() && *height > 0.0)
        .unwrap_or(42.0)
}

pub(crate) fn keycaps_bounds(
    view: &keycaps_scene::KeycapsView,
    contours: &[Contour],
) -> Option<(f64, f64, f64, f64)> {
    layout_camera::key_and_contour_bounds(
        view.keys.iter().map(|key| (key.pose, key.size)),
        contours,
    )
}

pub(crate) fn pcb_bounds(
    snapshot: &AcceptedSnapshot,
    scope: &Scope,
) -> Option<(f64, f64, f64, f64)> {
    if snapshot.session_epoch != scope.session_epoch || snapshot.document.id != scope.document_id {
        return None;
    }
    let board = snapshot
        .document
        .boards
        .iter()
        .find(|board| board.id == scope.board_id)?;
    let member_ids: BTreeSet<&str> = board.part_ids.iter().map(String::as_str).collect();
    let mut bounds: Option<(f64, f64, f64, f64)> = None;
    let mut include = |x: f64, y: f64| {
        bounds = Some(bounds.map_or((x, x, y, y), |(min_x, max_x, min_y, max_y)| {
            (min_x.min(x), max_x.max(x), min_y.min(y), max_y.max(y))
        }));
    };
    for contour in accepted_board_contours(snapshot, &scope.board_id).iter() {
        for point in &contour.points {
            include(point.x, point.y);
        }
    }
    for part in snapshot
        .document
        .parts
        .iter()
        .filter(|part| member_ids.contains(part.id.as_str()))
    {
        let pose = snapshot
            .scene
            .transforms
            .iter()
            .find(|transform| transform.id == part.id)
            .map(|transform| transform.pose)
            .unwrap_or(part.pose);
        include(pose.at.x, pose.at.y);
    }
    bounds
}
