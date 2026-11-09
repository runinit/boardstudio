use monstertruck_io::step::load::Table;
use monstertruck_meshing::prelude::*;

#[test]
fn successful_default_alignment_retains_original_endpoint_representatives() {
    let table = Table::from_step(include_str!("fixtures/choc-face27676.step")).unwrap();
    table.entity_report.require_empty().unwrap();
    let (source, report) = table
        .to_compressed_shell_reported(table.shell.values().next().unwrap())
        .unwrap();
    report.require_lossless().unwrap();
    let mesh = source.robust_triangulation(0.0005);
    let face = mesh.faces[0].surface.as_ref().unwrap();
    let referenced = face
        .faces()
        .triangle_iter()
        .flat_map(|triangle| triangle.map(|vertex| face.positions()[vertex.pos]))
        .collect::<Vec<_>>();
    let mut missing = Vec::new();
    for edge in mesh.faces[0].boundaries.iter().flatten() {
        for (sample, point) in mesh.edges[edge.index].curve.iter().enumerate() {
            if !referenced.contains(point) {
                missing.push((edge.index, sample, *point));
            }
        }
    }
    assert!(
        missing.is_empty(),
        "existing aligned source representatives were lost: {missing:?}"
    );
}
