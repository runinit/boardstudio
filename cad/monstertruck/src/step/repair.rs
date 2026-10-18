//! Internal operation accounting. A support translation is not whole-asset movement.
mod cuboid;
use super::*;
pub(super) use cuboid::Cuboid;
use monstertruck_io::step::load::step_geometry::Curve3D;
use monstertruck_io::step::load::step_p21::tables::{IntoOwned, PlaceHolder};

struct NativeLineSource {
    points: [Point3; 2],
    origin: Point3,
    direction: Vec<f64>,
    magnitude: f64,
}

/// Captured before checked conversion, from a table bound to the original AST.
pub(super) struct OriginalGeometry {
    lines: HashMap<u64, NativeLineSource>,
    boundaries: HashMap<u64, Vec<SourceBoundary>>,
}

struct SourceBoundary {
    orientation: bool,
    edges: Vec<(u64, bool)>,
}

fn entity_id<T>(holder: &PlaceHolder<T>) -> Option<u64> {
    match holder {
        PlaceHolder::Ref(Name::Entity(id)) => Some(*id),
        _ => None,
    }
}

fn source_boundaries(table: &Table, face: &FaceSurfaceHolder) -> Option<Vec<SourceBoundary>> {
    face.bounds
        .iter()
        .map(|holder| {
            let bound = table.face_bound.get(&entity_id(holder)?)?;
            let edge_loop = table.edge_loop.get(&entity_id(&bound.bound)?)?;
            let edges = edge_loop
                .edge_list
                .iter()
                .map(|holder| {
                    let id = entity_id(holder)?;
                    match table.oriented_edge.get(&id) {
                        Some(edge) => Some((entity_id(&edge.edge_element)?, edge.orientation)),
                        None if table.edge_curve.contains_key(&id) => Some((id, true)),
                        _ => None,
                    }
                })
                .collect::<Option<Vec<_>>>()?;
            Some(SourceBoundary {
                orientation: bound.orientation,
                edges,
            })
        })
        .collect()
}

impl OriginalGeometry {
    pub(super) fn capture(table: &Table) -> Self {
        let mut lines = HashMap::new();
        for (&id, holder) in &table.edge_curve {
            let PlaceHolder::Ref(Name::Entity(curve_id)) = &holder.edge_geometry else {
                continue;
            };
            if !table.line.contains_key(curve_id) {
                continue;
            }
            let (PlaceHolder::Ref(Name::Entity(_)), PlaceHolder::Ref(Name::Entity(_))) =
                (&holder.edge_start, &holder.edge_end)
            else {
                continue;
            };
            let Ok(edge) = holder.clone().into_owned(table) else {
                continue;
            };
            let CurveAny::Line(line) = &edge.edge_geometry else {
                continue;
            };
            if line.pnt.coordinates.len() != 3
                || edge.edge_start.vertex_geometry.coordinates.len() != 3
                || edge.edge_end.vertex_geometry.coordinates.len() != 3
            {
                continue;
            }
            lines.insert(
                id,
                NativeLineSource {
                    points: [
                        Point3::from(&edge.edge_start.vertex_geometry),
                        Point3::from(&edge.edge_end.vertex_geometry),
                    ],
                    origin: Point3::from(&line.pnt),
                    direction: line.dir.orientation.direction_ratios.clone(),
                    magnitude: line.dir.magnitude,
                },
            );
        }
        let boundaries = table
            .face_surface
            .iter()
            .filter_map(|(&id, face)| Some((id, source_boundaries(table, face)?)))
            .collect();
        Self { lines, boundaries }
    }

    fn bind_edges(&self, source: &SourceBoundStepShell) -> Option<Vec<u64>> {
        let shell = source.shell();
        let mut ids = vec![None; shell.edges.len()];
        if shell.faces.len() != source.source_face_ids().len() {
            return None;
        }
        for (&id, face) in source.source_face_ids().iter().zip(&shell.faces) {
            let boundaries = self.boundaries.get(&id)?;
            if boundaries.len() != face.boundaries.len() {
                return None;
            }
            for (raw, emitted) in boundaries.iter().zip(&face.boundaries) {
                if raw.edges.len() != emitted.len() {
                    return None;
                }
                let forward = raw.orientation == face.orientation;
                for (index, edge_use) in emitted.iter().enumerate() {
                    let index = if forward {
                        index
                    } else {
                        raw.edges.len() - 1 - index
                    };
                    let (id, orientation) = raw.edges[index];
                    if edge_use.orientation != (orientation == forward) {
                        return None;
                    }
                    let slot = ids.get_mut(edge_use.index)?;
                    if slot.is_some_and(|previous| previous != id) {
                        return None;
                    }
                    *slot = Some(id);
                }
            }
        }
        let ids = ids.into_iter().collect::<Option<Vec<_>>>()?;
        (ids.iter().copied().collect::<HashSet<_>>().len() == ids.len()).then_some(ids)
    }
}

#[derive(Debug)]
pub(super) struct Operation {
    pub occurrence: usize,
    pub source_edge: Option<u64>,
    pub stage: &'static str,
    pub bound_mm: Result<f64, &'static str>,
}

#[derive(Default)]
pub(super) struct Ledger(Vec<Operation>);

impl Ledger {
    pub(super) fn conversion(
        &mut self,
        original: &OriginalGeometry,
        source: &SourceBoundStepShell,
        factor: f64,
        placement: Matrix4,
        occurrence: usize,
    ) {
        let ids = original.bind_edges(source);
        for (index, edge) in source.shell().edges.iter().enumerate() {
            let id = ids
                .as_ref()
                .and_then(|ids| ids.as_slice().get(index))
                .copied();
            self.0.push(Operation {
                occurrence,
                source_edge: id,
                stage: "axis-aligned native LINE support substitution",
                bound_mm: id
                    .ok_or("original face/loop/edge binding is unproved")
                    .and_then(|id| {
                        line_support_bound(original, id, source, edge, factor, placement)
                    }),
            });
        }
        self.unknown(
            occurrence,
            "curve trims and surface conversion",
            "source domains and whole supports are not traced",
        );
    }

    pub(super) fn unknown(&mut self, occurrence: usize, stage: &'static str, reason: &'static str) {
        self.0.push(Operation {
            occurrence,
            source_edge: None,
            stage,
            bound_mm: Err(reason),
        });
    }

    /// Sum each operation once. Any missing proof poisons the complete bound.
    pub(super) fn complete_bound_mm(&self) -> Option<f64> {
        if self.0.is_empty() {
            return None;
        }
        self.0.iter().try_fold(0.0_f64, |sum, op| {
            let value = *op.bound_mm.as_ref().ok()?;
            if !value.is_finite() || value < 0.0 {
                return None;
            }
            let bound = add_upper(sum, value);
            bound.is_finite().then_some(bound)
        })
    }

    pub(super) fn trace(&self) {
        if std::env::var_os("MT_DEBUG_REPAIR_MOVEMENT").is_some() {
            for op in &self.0 {
                eprintln!(
                    "STEP repair occurrence={} edge={:?} stage={} bound_mm={:?}",
                    op.occurrence, op.source_edge, op.stage, op.bound_mm
                );
            }
        }
    }
}

/// This proves only the whole parsed LINE support translation. Decimal decoding and
/// finite trim selection stay unknown, even when all diagnostics happen to be zero.
fn line_support_bound(
    original: &OriginalGeometry,
    id: u64,
    source: &SourceBoundStepShell,
    edge: &monstertruck_topology::compress::CompressedEdge<Curve3D>,
    factor: f64,
    placement: Matrix4,
) -> Result<f64, &'static str> {
    let raw = original
        .lines
        .get(&id)
        .ok_or("non-native LINE: master selection, seams and nonlinear changes are untraced")?;
    let Curve3D::Line(line) = &edge.curve else {
        return Err("native LINE was replaced by another curve family");
    };
    if source.shell().vertices.as_slice().get(edge.vertices.0) != Some(&raw.points[0])
        || source.shell().vertices.as_slice().get(edge.vertices.1) != Some(&raw.points[1])
    {
        return Err("original EDGE_CURVE vertex attachments changed");
    }
    // Restrict the proof to exact coordinate-axis supports; no tolerance tests,
    // inverse searches or endpoint samples can establish parallelism here.
    let direction = &raw.direction;
    if direction.len() != 3
        || !direction.iter().all(|v| v.is_finite())
        || !raw.magnitude.is_finite()
        || raw.magnitude == 0.0
    {
        return Err("invalid original LINE direction");
    }
    let axes: Vec<_> = (0..3).filter(|&axis| direction[axis] != 0.0).collect();
    let [axis] = axes.as_slice() else {
        return Err("native LINE support is not axis-aligned");
    };
    if line.0[*axis] == line.1[*axis] || (0..3).any(|i| i != *axis && line.0[i] != line.1[i]) {
        return Err("emitted LINE support does not preserve the native axis");
    }
    // Both entire supports are parallel. Their transverse coordinate difference is
    // one uniform translation of EVERY point; longitudinal trim changes stay unknown.
    let origin = raw.origin;
    if [origin, line.0, line.1]
        .iter()
        .any(|p| !(0..3).all(|i| p[i].is_finite()))
    {
        return Err("nonfinite LINE support");
    }
    let mut delta = [0.0; 3];
    for (i, d) in delta.iter_mut().enumerate() {
        if i != *axis {
            *d = upper((line.0[i] - origin[i]).abs());
        }
    }
    placed_translation_bound(delta, factor, placement)
}

fn upper(value: f64) -> f64 {
    if value == 0.0 { 0.0 } else { value.next_up() }
}

fn add_upper(a: f64, b: f64) -> f64 {
    if a == 0.0 {
        b
    } else if b == 0.0 {
        a
    } else {
        (a + b).next_up()
    }
}

fn mul_upper(a: f64, b: f64) -> f64 {
    if a == 0.0 || b == 0.0 {
        0.0
    } else {
        (a * b).next_up()
    }
}

/// An outward-rounded L1 bound on A*(factor*delta) covers rotations, reflections,
/// uniform scale and shear. Common occurrence translation is not a repair.
fn placed_translation_bound(
    delta: [f64; 3],
    factor: f64,
    placement: Matrix4,
) -> Result<f64, &'static str> {
    if delta.iter().any(|v| !v.is_finite() || *v < 0.0)
        || !factor.is_finite()
        || factor <= 0.0
        || !(0..4).all(|col| (0..4).all(|row| placement[col][row].is_finite()))
        || placement.x.w != 0.0
        || placement.y.w != 0.0
        || placement.z.w != 0.0
        || placement.w.w != 1.0
    {
        return Err("nonfinite or non-affine world transform");
    }
    let mut bound = 0.0;
    for row in 0..3 {
        for col in 0..3 {
            let term = mul_upper(mul_upper(delta[col], factor), placement[col][row].abs());
            bound = add_upper(bound, term);
        }
    }
    if bound.is_finite() && bound >= 0.0 {
        Ok(bound)
    } else {
        Err("nonfinite world movement bound")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shifted_line_tetra() -> String {
        include_str!("../../tests/fixtures/tetra-mm.step").replace(
            "#22=LINE('',#1,#21);",
            "#310=CARTESIAN_POINT('',(0.,0.001953125,0.));#22=LINE('',#310,#21);",
        )
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn native_line_support_translation_is_measured_on_the_real_import_route() {
        let text = shifted_line_tetra();
        let model = import_step(text.clone().into_bytes()).unwrap();
        let operation = model
            ._repair_ledger
            .0
            .iter()
            .find(|op| op.source_edge == Some(23))
            .unwrap();
        let bound = operation
            .bound_mm
            .expect("the whole native LINE support translation is bounded");
        assert!((0.001953125..0.001953126).contains(&bound), "{bound}");
        assert_eq!(model.original_bytes(), text.as_bytes());
        assert_eq!(model.accuracy().max_repair_displacement_mm, None);
        assert_eq!(model.accuracy().analysis_error_bound_mm, None);
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn support_measurement_uses_world_units_and_keeps_occurrences_separate() {
        let assembly = include_str!("../../tests/fixtures/two-occurrences-mm.step");
        let text = assembly
            .replace(
                "#22=LINE('',#1,#21);",
                "#310=CARTESIAN_POINT('',(0.,0.001953125,0.));#22=LINE('',#310,#21);",
            )
            .replace("SI_UNIT(.MILLI.,.METRE.)", "SI_UNIT($,.METRE.)");
        let model = import_step(text.into_bytes()).unwrap();
        let operations: Vec<_> = model
            ._repair_ledger
            .0
            .iter()
            .filter(|op| op.source_edge == Some(23))
            .collect();
        assert_eq!(
            operations.len(),
            2,
            "one correction per occurrence, not per face use"
        );
        assert_ne!(operations[0].occurrence, operations[1].occurrence);
        for op in operations {
            let bound = op.bound_mm.unwrap();
            assert!((1.953125..1.953126).contains(&bound), "{op:?}");
        }
        assert_eq!(model.accuracy().max_repair_displacement_mm, None);
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn tilted_native_support_and_selected_pcurves_cannot_borrow_the_line_proof() {
        let tilted = shifted_line_tetra().replace(
            "#20=DIRECTION('',(2.,0.,0.));",
            "#20=DIRECTION('',(2.,0.000001,0.));",
        );
        let model = import_step(tilted.into_bytes()).unwrap();
        let operation = model
            ._repair_ledger
            .0
            .iter()
            .find(|op| op.source_edge == Some(23))
            .unwrap();
        assert_eq!(
            operation.bound_mm,
            Err("native LINE support is not axis-aligned")
        );
        let model =
            import_step(include_bytes!("../../tests/fixtures/source-pcurve-cube.step").to_vec())
                .unwrap();
        assert!(model._repair_ledger.0.iter().all(|op| op.bound_mm.is_err()));
        assert_eq!(model.accuracy().max_repair_displacement_mm, None);
        assert_eq!(model.accuracy().analysis_error_bound_mm, None);
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn unchanged_parsed_line_support_does_not_certify_the_asset_or_source_trim() {
        let model =
            import_step(include_bytes!("../../tests/fixtures/tetra-mm.step").to_vec()).unwrap();
        let operation = model
            ._repair_ledger
            .0
            .iter()
            .find(|op| op.source_edge == Some(23))
            .unwrap();
        assert_eq!(
            operation.bound_mm,
            Ok(0.0),
            "only this support is unchanged"
        );
        assert_eq!(model.accuracy().sampled_support_checks, 0);
        assert_eq!(model.accuracy().max_repair_displacement_mm, None);
        assert!(
            model
                ._repair_ledger
                .0
                .iter()
                .any(|op| op.stage == "curve trims and surface conversion" && op.bound_mm.is_err())
        );
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn original_topology_identity_mismatch_refuses_a_support_proof() {
        let text = shifted_line_tetra();
        let model = import_step(text.clone().into_bytes()).unwrap();
        let altered = text.replace(
            "#50=ORIENTED_EDGE('',*,*,#27,.T.);",
            "#50=ORIENTED_EDGE('',*,*,#23,.T.);",
        );
        let exchange = parser::parse(&altered).unwrap();
        let raw = OriginalGeometry::capture(&Table::from_data_section(&exchange.data[0]));
        assert!(
            raw.bind_edges(&model.solids[&101].source[&104].shells()[0])
                .is_none()
        );
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn world_translation_bound_covers_reflection_scale_and_underflow() {
        let placement =
            Matrix4::from_translation(monstertruck_modeling::Vector3::new(1e10, 0., 0.))
                * Matrix4::from_scale(-3.0);
        let bound = placed_translation_bound([0., 0.125, 0.], 2.0, placement).unwrap();
        assert!((0.75..0.750001).contains(&bound));
        assert!(
            placed_translation_bound([f64::from_bits(1), 0., 0.], 0.5, Matrix4::identity())
                .unwrap()
                > 0.0
        );
        assert!(placed_translation_bound([f64::MAX, 0., 0.], 2.0, Matrix4::identity()).is_err());
    }
}
