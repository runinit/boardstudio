use crate::{MAX_MODEL_POSITION_FLOATS, ModelMesh, assets::check_size};
use std::collections::HashMap;
use three_d::{InnerSpace, Mat4, Rad, SquareMatrix, Vec3, Vec4};

const KICAD_VRML_UNIT_MM: f32 = 2.54;

#[derive(Clone, Copy)]
struct Node<'a> {
    name: &'a str,
    open: usize,
    close: usize,
}

pub fn decode_wrl(bytes: &[u8]) -> Result<ModelMesh, String> {
    check_size(bytes)?;
    let source = std::str::from_utf8(bytes).map_err(|_| "WRL preview must be UTF-8".to_owned())?;
    let tokens = tokenize(source);
    if tokens.iter().any(|token| {
        matches!(
            token.as_str(),
            "Inline" | "ImageTexture" | "MovieTexture" | "Script" | "PROTO" | "EXTERNPROTO"
        )
    }) || tokens
        .windows(2)
        .any(|pair| pair[0] == "url" && pair[1].starts_with('"'))
    {
        return Err("WRL preview supports self-contained static geometry and materials; external resources are unsupported".to_owned());
    }

    let definitions = find_definitions(&tokens);
    let mut output = ModelMesh {
        colors: Some(Vec::new()),
        ..ModelMesh::default()
    };
    let mut cursor = 0;
    while cursor < tokens.len() {
        if let Some(node) = resolve_node(&tokens, cursor, &definitions) {
            walk_node(
                &tokens,
                node,
                &definitions,
                Mat4::identity(),
                [0.72, 0.75, 0.78],
                &mut output,
                &mut Vec::new(),
            )?;
            cursor = if tokens[cursor] == "USE" {
                cursor + 2
            } else {
                node.close + 1
            };
        } else {
            cursor += 1;
        }
    }
    if output.colors.as_ref().is_some_and(Vec::is_empty) {
        output.colors = None;
    }
    output.finish("WRL")
}

fn tokenize(source: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut chars = source.chars().peekable();
    while let Some(character) = chars.next() {
        match character {
            '#' => while chars.next().is_some_and(|next| next != '\n') {},
            '{' | '}' | '[' | ']' | ',' => result.push(character.to_string()),
            '"' => {
                let mut value = String::from("\"");
                for next in chars.by_ref() {
                    value.push(next);
                    if next == '"' && !value.ends_with("\\\"") {
                        break;
                    }
                }
                result.push(value);
            }
            value if value.is_whitespace() => {}
            value => {
                let mut token = String::from(value);
                while chars.peek().is_some_and(|next| {
                    !next.is_whitespace() && !matches!(next, '{' | '}' | '[' | ']' | ',' | '#')
                }) {
                    token.push(chars.next().unwrap());
                }
                result.push(token);
            }
        }
    }
    result
}

fn node_at<'a>(tokens: &'a [String], start: usize) -> Option<Node<'a>> {
    let name_index = if tokens.get(start).is_some_and(|token| token == "DEF") {
        start + 2
    } else {
        start
    };
    let name = tokens.get(name_index)?.as_str();
    if !matches!(
        name,
        "Transform"
            | "Group"
            | "Shape"
            | "Appearance"
            | "Material"
            | "IndexedFaceSet"
            | "Coordinate"
            | "Color"
            | "Normal"
            | "Box"
            | "Sphere"
            | "Cylinder"
            | "Cone"
    ) {
        return None;
    }
    let open = name_index + 1;
    if tokens.get(open)? != "{" {
        return None;
    }
    let mut depth = 1;
    let mut cursor = open + 1;
    while cursor < tokens.len() {
        match tokens[cursor].as_str() {
            "{" => depth += 1,
            "}" => {
                depth -= 1;
                if depth == 0 {
                    return Some(Node {
                        name,
                        open,
                        close: cursor,
                    });
                }
            }
            _ => {}
        }
        cursor += 1;
    }
    None
}

fn find_definitions<'a>(tokens: &'a [String]) -> HashMap<&'a str, Node<'a>> {
    let mut definitions = HashMap::new();
    for start in 0..tokens.len() {
        if tokens[start] == "DEF"
            && let (Some(name), Some(node)) = (tokens.get(start + 1), node_at(tokens, start))
        {
            definitions.insert(name.as_str(), node);
        }
    }
    definitions
}

fn resolve_node<'a>(
    tokens: &'a [String],
    start: usize,
    definitions: &HashMap<&'a str, Node<'a>>,
) -> Option<Node<'a>> {
    if tokens.get(start).is_some_and(|token| token == "USE") {
        return tokens
            .get(start + 1)
            .and_then(|name| definitions.get(name.as_str()))
            .copied();
    }
    node_at(tokens, start)
}

fn field(tokens: &[String], node: Node<'_>, key: &str) -> Option<usize> {
    let mut brace_depth = 0;
    let mut bracket_depth = 0;
    let mut cursor = node.open + 1;
    while cursor < node.close {
        match tokens[cursor].as_str() {
            "{" => brace_depth += 1,
            "}" => brace_depth -= 1,
            "[" => bracket_depth += 1,
            "]" => bracket_depth -= 1,
            token if brace_depth == 0 && bracket_depth == 0 && token == key => {
                return Some(cursor + 1);
            }
            _ => {}
        }
        cursor += 1;
    }
    None
}

fn scalar(tokens: &[String], node: Node<'_>, key: &str, defaults: &[f32]) -> Vec<f32> {
    let Some(start) = field(tokens, node, key) else {
        return defaults.to_vec();
    };
    let count = if defaults.len() == 1 {
        1
    } else {
        defaults.len()
    };
    (0..count)
        .map(|offset| {
            tokens
                .get(start + offset)
                .and_then(|token| token.parse().ok())
                .unwrap_or(defaults[offset])
        })
        .collect()
}

fn list_values(tokens: &[String], start: usize) -> Vec<&str> {
    if tokens.get(start).is_none_or(|token| token != "[") {
        return Vec::new();
    }
    let mut result = Vec::new();
    let mut cursor = start + 1;
    while cursor < tokens.len() && tokens[cursor] != "]" {
        if !matches!(tokens[cursor].as_str(), "[" | "]" | ",") {
            result.push(tokens[cursor].as_str());
        }
        cursor += 1;
    }
    result
}

fn node_field<'a>(
    tokens: &'a [String],
    parent: Node<'a>,
    key: &str,
    definitions: &HashMap<&'a str, Node<'a>>,
) -> Option<Node<'a>> {
    let value = field(tokens, parent, key)?;
    resolve_node(tokens, value, definitions)
}

fn child_nodes<'a>(
    tokens: &'a [String],
    parent: Node<'a>,
    key: &str,
    definitions: &HashMap<&'a str, Node<'a>>,
) -> Vec<Node<'a>> {
    let Some(value) = field(tokens, parent, key) else {
        return Vec::new();
    };
    let mut result = Vec::new();
    if tokens.get(value).is_some_and(|token| token == "[") {
        let mut cursor = value + 1;
        while cursor < parent.close && tokens[cursor] != "]" {
            if let Some(child) = resolve_node(tokens, cursor, definitions) {
                cursor = if tokens[cursor] == "USE" {
                    cursor + 2
                } else {
                    child.close + 1
                };
                result.push(child);
            } else {
                cursor += 1;
            }
        }
    } else if let Some(child) = resolve_node(tokens, value, definitions) {
        result.push(child);
    }
    result
}

fn walk_node<'a>(
    tokens: &'a [String],
    node: Node<'a>,
    definitions: &HashMap<&'a str, Node<'a>>,
    transform: Mat4,
    inherited_color: [f32; 3],
    output: &mut ModelMesh,
    active_nodes: &mut Vec<usize>,
) -> Result<(), String> {
    if active_nodes.contains(&node.open) {
        return Err("WRL contains a cyclic USE reference".to_owned());
    }
    active_nodes.push(node.open);
    let result = (|| -> Result<(), String> {
        match node.name {
            "Transform" => {
                let translation = scalar(tokens, node, "translation", &[0.0, 0.0, 0.0]);
                let rotation = scalar(tokens, node, "rotation", &[0.0, 0.0, 1.0, 0.0]);
                let scale = scalar(tokens, node, "scale", &[1.0, 1.0, 1.0]);
                let center = scalar(tokens, node, "center", &[0.0, 0.0, 0.0]);
                let axis = Vec3::new(rotation[0], rotation[1], rotation[2]);
                let rotation_matrix = if axis.magnitude2() > 0.0 {
                    Mat4::from_axis_angle(axis.normalize(), Rad(rotation[3]))
                } else {
                    Mat4::identity()
                };
                let center = Vec3::new(center[0], center[1], center[2]);
                let local = Mat4::from_translation(Vec3::new(
                    translation[0],
                    translation[1],
                    translation[2],
                )) * Mat4::from_translation(center)
                    * rotation_matrix
                    * Mat4::from_nonuniform_scale(scale[0], scale[1], scale[2])
                    * Mat4::from_translation(-center);
                for child in child_nodes(tokens, node, "children", definitions) {
                    walk_node(
                        tokens,
                        child,
                        definitions,
                        transform * local,
                        inherited_color,
                        output,
                        active_nodes,
                    )?;
                }
            }
            "Group" => {
                for child in child_nodes(tokens, node, "children", definitions) {
                    walk_node(
                        tokens,
                        child,
                        definitions,
                        transform,
                        inherited_color,
                        output,
                        active_nodes,
                    )?;
                }
            }
            "Shape" => {
                let color = node_field(tokens, node, "appearance", definitions)
                    .and_then(|appearance| node_field(tokens, appearance, "material", definitions))
                    .map(|material| scalar(tokens, material, "diffuseColor", &[0.72, 0.75, 0.78]))
                    .map(|values| [values[0], values[1], values[2]])
                    .unwrap_or(inherited_color);
                if let Some(geometry) = node_field(tokens, node, "geometry", definitions) {
                    append_geometry(tokens, geometry, definitions, transform, color, output)?;
                }
            }
            _ => {}
        }
        Ok(())
    })();
    active_nodes.pop();
    result
}

fn append_geometry<'a>(
    tokens: &'a [String],
    node: Node<'a>,
    definitions: &HashMap<&'a str, Node<'a>>,
    transform: Mat4,
    material: [f32; 3],
    output: &mut ModelMesh,
) -> Result<(), String> {
    match node.name {
        "IndexedFaceSet" => {
            let Some(coordinate) = node_field(tokens, node, "coord", definitions) else {
                return Ok(());
            };
            let point_values = field(tokens, coordinate, "point")
                .map(|index| list_values(tokens, index))
                .unwrap_or_default();
            let points = point_values
                .as_chunks::<3>()
                .0
                .iter()
                .filter_map(|chunk| {
                    Some([
                        chunk[0].parse().ok()?,
                        chunk[1].parse().ok()?,
                        chunk[2].parse().ok()?,
                    ])
                })
                .collect::<Vec<_>>();
            let index_values = field(tokens, node, "coordIndex")
                .map(|index| list_values(tokens, index))
                .unwrap_or_default();
            let polygons = split_faces(&index_values);
            let color_values = node_field(tokens, node, "color", definitions)
                .and_then(|color| {
                    field(tokens, color, "color").map(|index| list_values(tokens, index))
                })
                .unwrap_or_default();
            let colors = color_values
                .as_chunks::<3>()
                .0
                .iter()
                .filter_map(|chunk| {
                    Some([
                        chunk[0].parse().ok()?,
                        chunk[1].parse().ok()?,
                        chunk[2].parse().ok()?,
                    ])
                })
                .collect::<Vec<_>>();
            let color_indices = field(tokens, node, "colorIndex")
                .map(|index| list_values(tokens, index))
                .unwrap_or_default();
            let color_polygons = split_faces(&color_indices);
            let per_vertex = !field(tokens, node, "colorPerVertex")
                .is_some_and(|index| tokens.get(index).is_some_and(|value| value == "FALSE"));
            for (face_index, polygon) in polygons.iter().enumerate() {
                if polygon.len() < 3 {
                    continue;
                }
                for corner in 1..polygon.len() - 1 {
                    let indexes = [polygon[0], polygon[corner], polygon[corner + 1]];
                    let mut triangle = [[0.0; 3]; 3];
                    let mut triangle_colors = [material; 3];
                    for vertex in 0..3 {
                        let index = indexes[vertex];
                        if index >= points.len() {
                            return Err("WRL contains an invalid coordinate index".to_owned());
                        }
                        triangle[vertex] = points[index];
                        let color_index = if per_vertex {
                            color_polygons
                                .get(face_index)
                                .and_then(|face| {
                                    face.get(if vertex == 0 {
                                        0
                                    } else if vertex == 1 {
                                        corner
                                    } else {
                                        corner + 1
                                    })
                                })
                                .copied()
                                .unwrap_or(index)
                        } else {
                            color_polygons
                                .get(face_index)
                                .and_then(|face| face.first())
                                .copied()
                                .or_else(|| colors.len().checked_sub(1))
                                .unwrap_or(0)
                        };
                        if let Some(color) = colors.get(color_index) {
                            triangle_colors[vertex] = *color;
                        }
                    }
                    append_triangle(triangle, triangle_colors, transform, output)?;
                }
            }
        }
        "Box" => {
            let size = scalar(tokens, node, "size", &[2.0, 2.0, 2.0]);
            append_box([size[0], size[1], size[2]], transform, material, output)?;
        }
        "Sphere" => {
            let radius = scalar(tokens, node, "radius", &[1.0])[0];
            append_sphere(radius, transform, material, output)?;
        }
        "Cylinder" | "Cone" => append_cylinder_or_cone(tokens, node, transform, material, output)?,
        _ => {}
    }
    Ok(())
}

fn split_faces(values: &[&str]) -> Vec<Vec<usize>> {
    let mut faces = Vec::new();
    let mut face = Vec::new();
    for value in values {
        if let Ok(index) = value.parse::<i32>() {
            if index < 0 {
                if !face.is_empty() {
                    faces.push(std::mem::take(&mut face));
                }
            } else {
                face.push(index as usize);
            }
        }
    }
    if !face.is_empty() {
        faces.push(face);
    }
    faces
}

fn append_triangle(
    points: [[f32; 3]; 3],
    colors: [[f32; 3]; 3],
    transform: Mat4,
    output: &mut ModelMesh,
) -> Result<(), String> {
    if output.positions.len() + 9 > MAX_MODEL_POSITION_FLOATS {
        return Err("Model exceeds preview triangle limits".to_owned());
    }
    let transformed = points.map(|point| {
        let result = transform * Vec4::new(point[0], point[1], point[2], 1.0);
        [
            result.x * KICAD_VRML_UNIT_MM,
            result.y * KICAD_VRML_UNIT_MM,
            result.z * KICAD_VRML_UNIT_MM,
        ]
    });
    let a = Vec3::new(transformed[0][0], transformed[0][1], transformed[0][2]);
    let b = Vec3::new(transformed[1][0], transformed[1][1], transformed[1][2]);
    let c = Vec3::new(transformed[2][0], transformed[2][1], transformed[2][2]);
    let normal = (b - a).cross(c - a).normalize();
    for index in 0..3 {
        output.positions.extend_from_slice(&transformed[index]);
        output
            .normals
            .extend_from_slice(&[normal.x, normal.y, normal.z]);
        output
            .colors
            .get_or_insert_with(Vec::new)
            .extend_from_slice(&colors[index]);
    }
    Ok(())
}

fn append_box(
    size: [f32; 3],
    transform: Mat4,
    color: [f32; 3],
    output: &mut ModelMesh,
) -> Result<(), String> {
    let half = size.map(|value| value / 2.0);
    let corners = [
        [-half[0], -half[1], -half[2]],
        [half[0], -half[1], -half[2]],
        [half[0], half[1], -half[2]],
        [-half[0], half[1], -half[2]],
        [-half[0], -half[1], half[2]],
        [half[0], -half[1], half[2]],
        [half[0], half[1], half[2]],
        [-half[0], half[1], half[2]],
    ];
    for face in [
        [0, 1, 2, 3],
        [4, 7, 6, 5],
        [0, 4, 5, 1],
        [1, 5, 6, 2],
        [2, 6, 7, 3],
        [4, 0, 3, 7],
    ] {
        for triangle in [[face[0], face[1], face[2]], [face[0], face[2], face[3]]] {
            append_triangle(
                triangle.map(|index| corners[index]),
                [color; 3],
                transform,
                output,
            )?;
        }
    }
    Ok(())
}

fn append_sphere(
    radius: f32,
    transform: Mat4,
    color: [f32; 3],
    output: &mut ModelMesh,
) -> Result<(), String> {
    const LONGITUDES: usize = 16;
    const LATITUDES: usize = 10;
    for latitude in 0..LATITUDES {
        let phi0 = std::f32::consts::PI * latitude as f32 / LATITUDES as f32;
        let phi1 = std::f32::consts::PI * (latitude + 1) as f32 / LATITUDES as f32;
        for longitude in 0..LONGITUDES {
            let theta0 = std::f32::consts::TAU * longitude as f32 / LONGITUDES as f32;
            let theta1 = std::f32::consts::TAU * (longitude + 1) as f32 / LONGITUDES as f32;
            let point = |phi: f32, theta: f32| {
                [
                    radius * phi.sin() * theta.cos(),
                    radius * phi.cos(),
                    radius * phi.sin() * theta.sin(),
                ]
            };
            let quad = [
                point(phi0, theta0),
                point(phi1, theta0),
                point(phi1, theta1),
                point(phi0, theta1),
            ];
            append_triangle([quad[0], quad[1], quad[2]], [color; 3], transform, output)?;
            append_triangle([quad[0], quad[2], quad[3]], [color; 3], transform, output)?;
        }
    }
    Ok(())
}

fn append_cylinder_or_cone(
    tokens: &[String],
    node: Node<'_>,
    transform: Mat4,
    color: [f32; 3],
    output: &mut ModelMesh,
) -> Result<(), String> {
    const SIDES: usize = 20;
    let height = scalar(tokens, node, "height", &[2.0])[0];
    let is_cone = node.name == "Cone";
    let radius = scalar(
        tokens,
        node,
        if is_cone { "bottomRadius" } else { "radius" },
        &[1.0],
    )[0];
    let bottom = -height / 2.0;
    let top = height / 2.0;
    for side in 0..SIDES {
        let a = std::f32::consts::TAU * side as f32 / SIDES as f32;
        let b = std::f32::consts::TAU * (side + 1) as f32 / SIDES as f32;
        let p0 = [radius * a.cos(), bottom, radius * a.sin()];
        let p1 = [radius * b.cos(), bottom, radius * b.sin()];
        let p2 = if is_cone {
            [0.0, top, 0.0]
        } else {
            [radius * b.cos(), top, radius * b.sin()]
        };
        let p3 = if is_cone {
            p2
        } else {
            [radius * a.cos(), top, radius * a.sin()]
        };
        append_triangle([p0, p1, p2], [color; 3], transform, output)?;
        if !is_cone {
            append_triangle([p0, p2, p3], [color; 3], transform, output)?;
        }
        append_triangle([[0.0, bottom, 0.0], p1, p0], [color; 3], transform, output)?;
        if !is_cone {
            append_triangle([[0.0, top, 0.0], p3, p2], [color; 3], transform, output)?;
        }
    }
    Ok(())
}
