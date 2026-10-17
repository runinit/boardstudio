use crate::{MeshBuffers, mesh::mesh_buffers};
use boardstudio_contracts::Vec2;
use monstertruck_meshing::tessellation::{TessellationOptions, shell_to_polygon_strict_with};
use monstertruck_modeling::{Curve, Point3, Solid, Surface, Vector3, Wire, builder};

const PROFILE_MESH_TOLERANCE_MM: f64 = 0.1;
const PROFILE_EDGE_ANGULAR_TOLERANCE_RAD: f64 = 0.5;

/// Which prepared case body is being extruded.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProfileKind {
    Plate,
    Tray,
    Lid,
}

/// Already-prepared planar region with an exterior and optional through-holes.
#[derive(Clone, Debug, PartialEq)]
pub struct ProfileRegion {
    pub outer: Vec<Vec2>,
    pub holes: Vec<Vec<Vec2>>,
}

/// Inputs needed to create the upstream plate/tray/lid extrusion.
#[derive(Clone, Debug, PartialEq)]
pub struct ProfileInput {
    pub kind: ProfileKind,
    pub base_z: f64,
    pub thickness: f64,
    pub wall_height: f64,
    pub regions: Vec<ProfileRegion>,
}

/// Opaque exact solids created from a prepared case profile.
pub struct ExactModel {
    pub(crate) solids: Vec<Solid>,
}

impl ExactModel {
    /// Returns physical-solid and conservative face/edge-occurrence cache costs.
    pub fn cache_complexity(&self) -> (usize, usize) {
        let mut topology = 0usize;
        for solid in &self.solids {
            for face in solid.face_iter() {
                topology = topology
                    .saturating_add(1)
                    .saturating_add(face.edge_iter().count());
            }
        }
        (self.solids.len(), topology)
    }
}

/// Extrudes each prepared region into its own exact solid.
pub fn build_profile(input: ProfileInput) -> Result<ExactModel, String> {
    if input.regions.is_empty() {
        return Err("Case profile requires at least one prepared region".into());
    }
    let wall_height = if input.kind == ProfileKind::Plate {
        0.0
    } else {
        input.wall_height
    };
    let total_height = input.thickness + wall_height;
    let top_z = input.base_z + total_height;
    if !input.base_z.is_finite()
        || !total_height.is_finite()
        || total_height <= 0.0
        || !top_z.is_finite()
    {
        return Err("Case body has invalid height or elevation".into());
    }

    let mut solids = Vec::with_capacity(input.regions.len());
    for region in input.regions {
        let (outer, holes) = validate_region(region)?;
        let mut rings = Vec::with_capacity(holes.len() + 1);
        rings.push(&outer);
        rings.extend(holes.iter());
        let wires = rings
            .into_iter()
            .map(|ring| wire(ring, input.base_z))
            .collect::<Result<Vec<_>, _>>()?;
        let solid = monstertruck_modeling::profile::solid_from_planar_profile::<Curve, Surface>(
            wires,
            Vector3::unit_z() * total_height,
        )
        .map_err(|error| format!("Monstertruck could not extrude case profile: {error:?}"))?;
        solids.push(solid);
    }
    Ok(ExactModel { solids })
}

/// Tessellates exact profile solids into complete, oriented triangle buffers.
pub fn mesh_profile(model: &ExactModel) -> Result<MeshBuffers, String> {
    let mut meshes = Vec::with_capacity(model.solids.len());
    for solid in &model.solids {
        for shell in solid.boundaries() {
            meshes.push(
                shell_to_polygon_strict_with(
                    shell,
                    TessellationOptions {
                        tolerance: PROFILE_MESH_TOLERANCE_MM,
                        edge_angular_tolerance: Some(PROFILE_EDGE_ANGULAR_TOLERANCE_RAD),
                        ..Default::default()
                    },
                )
                .map_err(|error| format!("Monstertruck could not mesh case profile: {error:?}"))?,
            );
        }
    }
    mesh_buffers(meshes)
}

fn wire(points: &[Vec2], z: f64) -> Result<Wire, String> {
    if points.len() < 3
        || !z.is_finite()
        || points
            .iter()
            .any(|point| !point.x.is_finite() || !point.y.is_finite())
    {
        return Err("Case profile contains an invalid contour".into());
    }
    let vertices = points
        .iter()
        .map(|point| builder::vertex(Point3::new(point.x, point.y, z)))
        .collect::<Vec<_>>();
    let edges = (0..vertices.len())
        .map(|index| builder::line(&vertices[index], &vertices[(index + 1) % vertices.len()]))
        .collect::<Vec<_>>();
    Ok(edges.into())
}

fn validate_region(region: ProfileRegion) -> Result<(Vec<Vec2>, Vec<Vec<Vec2>>), String> {
    let exterior = region
        .outer
        .iter()
        .map(|point| [point.x, point.y])
        .collect::<Vec<_>>();
    let holes = region
        .holes
        .iter()
        .map(|hole| hole.iter().map(|point| [point.x, point.y]).collect())
        .collect::<Vec<Vec<_>>>();
    let (outer, holes) = boardstudio_contracts::normalize_case_profile_region(&exterior, &holes)
        .map_err(|error| format!("Case profile {error}"))?;
    let convert = |ring: Vec<[f64; 2]>| {
        ring.into_iter()
            .map(|point| Vec2 {
                x: point[0],
                y: point[1],
            })
            .collect()
    };
    Ok((convert(outer), holes.into_iter().map(convert).collect()))
}
