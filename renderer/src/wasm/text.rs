use crate::geometry::MeshData;

pub(super) fn text_mesh(
    text: &str,
    origin: [f32; 2],
    size: f32,
    rotation: f32,
    back: bool,
    z: f32,
) -> Result<MeshData, String> {
    let cell_h = size / 7.0;
    let cell_w = cell_h * 0.72;
    let advance = cell_w * 6.0;
    let start_x = origin[0] - advance * text.chars().count() as f32 / 2.0;
    let mut mesh = MeshData::default();
    let normal = if back {
        [0.0, 0.0, -1.0]
    } else {
        [0.0, 0.0, 1.0]
    };
    let angle = rotation.to_radians();
    let cos = angle.cos();
    let sin = angle.sin();
    for (character_index, character) in text.chars().enumerate() {
        let rows = glyph(character);
        for (row, bits) in rows.iter().enumerate() {
            for column in 0..5 {
                if bits & (1 << (4 - column)) == 0 {
                    continue;
                }
                let x = start_x + (character_index as f32 * 6.0 + column as f32) * cell_w;
                let y = origin[1] + (3.0 - row as f32) * cell_h;
                let corners = [
                    [x, y],
                    [x + cell_w * 0.72, y],
                    [x + cell_w * 0.72, y + cell_h * 0.8],
                    [x, y + cell_h * 0.8],
                ];
                let transform_point = |point: [f32; 2]| {
                    let local_x = (point[0] - origin[0]) * if back { -1.0 } else { 1.0 };
                    let local_y = point[1] - origin[1];
                    [
                        origin[0] + local_x * cos - local_y * sin,
                        origin[1] + local_x * sin + local_y * cos,
                    ]
                };
                let base = mesh.positions.len() as u32;
                for index in [0, 1, 2, 0, 2, 3] {
                    let point = transform_point(corners[index]);
                    mesh.positions.push([point[0], point[1], z]);
                    mesh.normals.push(normal);
                    mesh.indices.push(base + mesh.indices.len() as u32 - base);
                }
            }
        }
    }
    for triangle in mesh.indices.as_chunks_mut::<3>().0.iter_mut() {
        let a = mesh.positions[triangle[0] as usize];
        let b = mesh.positions[triangle[1] as usize];
        let c = mesh.positions[triangle[2] as usize];
        let cross = [
            (b[1] - a[1]) * (c[2] - a[2]) - (b[2] - a[2]) * (c[1] - a[1]),
            (b[2] - a[2]) * (c[0] - a[0]) - (b[0] - a[0]) * (c[2] - a[2]),
            (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]),
        ];
        if cross[0] * normal[0] + cross[1] * normal[1] + cross[2] * normal[2] < 0.0 {
            triangle.swap(1, 2);
        }
    }
    Ok(mesh)
}

fn glyph(character: char) -> [u8; 7] {
    match character.to_ascii_uppercase() {
        'A' => [14, 17, 17, 31, 17, 17, 17],
        'B' => [30, 17, 17, 30, 17, 17, 30],
        'C' => [14, 17, 16, 16, 16, 17, 14],
        'D' => [30, 17, 17, 17, 17, 17, 30],
        'E' => [31, 16, 16, 30, 16, 16, 31],
        'F' => [31, 16, 16, 30, 16, 16, 16],
        'G' => [14, 17, 16, 23, 17, 17, 15],
        'H' => [17, 17, 17, 31, 17, 17, 17],
        'I' => [14, 4, 4, 4, 4, 4, 14],
        'J' => [7, 2, 2, 2, 2, 18, 12],
        'K' => [17, 18, 20, 24, 20, 18, 17],
        'L' => [16, 16, 16, 16, 16, 16, 31],
        'M' => [17, 27, 21, 21, 17, 17, 17],
        'N' => [17, 25, 21, 19, 17, 17, 17],
        'O' => [14, 17, 17, 17, 17, 17, 14],
        'P' => [30, 17, 17, 30, 16, 16, 16],
        'Q' => [14, 17, 17, 17, 21, 18, 13],
        'R' => [30, 17, 17, 30, 20, 18, 17],
        'S' => [15, 16, 16, 14, 1, 1, 30],
        'T' => [31, 4, 4, 4, 4, 4, 4],
        'U' => [17, 17, 17, 17, 17, 17, 14],
        'V' => [17, 17, 17, 17, 17, 10, 4],
        'W' => [17, 17, 17, 21, 21, 21, 10],
        'X' => [17, 17, 10, 4, 10, 17, 17],
        'Y' => [17, 17, 10, 4, 4, 4, 4],
        'Z' => [31, 1, 2, 4, 8, 16, 31],
        '0' => [14, 17, 19, 21, 25, 17, 14],
        '1' => [4, 12, 4, 4, 4, 4, 14],
        '2' => [14, 17, 1, 2, 4, 8, 31],
        '3' => [30, 1, 1, 14, 1, 1, 30],
        '4' => [2, 6, 10, 18, 31, 2, 2],
        '5' => [31, 16, 16, 30, 1, 1, 30],
        '6' => [14, 16, 16, 30, 17, 17, 14],
        '7' => [31, 1, 2, 4, 8, 8, 8],
        '8' => [14, 17, 17, 14, 17, 17, 14],
        '9' => [14, 17, 17, 15, 1, 1, 14],
        '-' => [0, 0, 0, 31, 0, 0, 0],
        '_' => [0, 0, 0, 0, 0, 0, 31],
        '.' => [0, 0, 0, 0, 0, 12, 12],
        ':' => [0, 12, 12, 0, 12, 12, 0],
        '/' => [1, 2, 2, 4, 8, 8, 16],
        '+' => [0, 4, 4, 31, 4, 4, 0],
        _ => [0; 7],
    }
}
