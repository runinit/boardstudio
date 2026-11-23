mod render;
mod scene;
mod text;

use scene::{BuiltScene, build_scene, push_data};

use crate::{
    ModelMesh, decode_stl, decode_wrl,
    geometry::{
        BoardContour, MeshData, SurfaceClipper, board_mesh, box_corners, feature_edges,
        polygon_area, stroke_mesh, surface_mesh,
    },
    is_stale_scene_revision,
    math::{component_model_transform, pcb_model_transform, physical_pcb_transform},
};
// Keep stack helpers private while exercising the same source in native tests.
include!("mechanical.rs");
use js_sys::{Float32Array, Object, Reflect};
use serde::Deserialize;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use three_d::{
    AmbientLight, AxisAlignedBoundingBox, Camera, ClearState, ColorTexture, CpuMaterial, CpuMesh,
    Cull, DepthTexture, DirectionalLight, Effect, EffectMaterialId, Geometry, GeometryId, Gm,
    Indices, InnerSpace, Light, Mat4, Material, MaterialType, Matrix, Mesh, PhysicalMaterial,
    Positions, Program, RenderStates, RenderTarget, SquareMatrix, Srgba, Vec3, Vec4, VertexBuffer,
    Viewer, Viewport,
};
use wasm_bindgen::{JsCast, prelude::*};
use web_sys::{HtmlCanvasElement, WebGl2RenderingContext};

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct SceneInput {
    revision: u64,
    kind: String,
    theme: String,
    view: String,
    selected_layer: String,
    keep_camera: bool,
    hidden: Vec<String>,
    case_mesh: Option<MeshInput>,
    component_previews: Vec<ComponentInput>,
    board_thickness: f32,
    board: Option<BoardInput>,
    models: Vec<LoadedModelInput>,
    bodies: Vec<BodyInput>,
    module_bodies: Vec<ModuleBodyInput>,
    mechanical_stack: Vec<StackLayerInput>,
    battery: Option<BatteryInput>,
    reference: Option<ReferenceInput>,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct MeshInput {
    #[serde(deserialize_with = "float_buffer")]
    positions: Vec<f32>,
    #[serde(deserialize_with = "float_buffer")]
    normals: Vec<f32>,
    #[serde(deserialize_with = "optional_float_buffer")]
    colors: Option<Vec<f32>>,
}

// Typed buffers cross the JS/WASM boundary in one copy. Serde's generic sequence
// visitor otherwise calls into JavaScript separately for every mesh coordinate.
fn read_float_buffer(value: JsValue) -> Result<Vec<f32>, String> {
    if value.is_instance_of::<Float32Array>() {
        Ok(value.unchecked_into::<Float32Array>().to_vec())
    } else {
        serde_wasm_bindgen::from_value(value).map_err(|error| error.to_string())
    }
}

fn float_buffer<'de, D: serde::Deserializer<'de>>(de: D) -> Result<Vec<f32>, D::Error> {
    let value: JsValue = serde_wasm_bindgen::preserve::deserialize(de)?;
    read_float_buffer(value).map_err(serde::de::Error::custom)
}

fn optional_float_buffer<'de, D: serde::Deserializer<'de>>(
    de: D,
) -> Result<Option<Vec<f32>>, D::Error> {
    let value: JsValue = serde_wasm_bindgen::preserve::deserialize(de)?;
    if value.is_null() || value.is_undefined() {
        return Ok(None);
    }
    read_float_buffer(value)
        .map(Some)
        .map_err(serde::de::Error::custom)
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct Vec2Input {
    x: f32,
    y: f32,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct Vec3Input {
    x: f32,
    y: f32,
    z: f32,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct PoseInput {
    at: Vec2Input,
    rotation: f32,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct ModelTransformInput {
    offset: Vec3Input,
    rotation: Vec3Input,
    scale: Vec3Input,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct ComponentInput {
    id: String,
    reference: String,
    pose: PoseInput,
    side: String,
    model: ModelTransformInput,
    mesh: MeshInput,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct ContourInput {
    points: Vec<Vec2Input>,
    hole: bool,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct SurfaceInput {
    layer: String,
    points: Vec<Vec2Input>,
    width: f32,
    filled: bool,
    text: String,
    rotation: f32,
    text_size: f32,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct PcbModelInput {
    id: String,
    reference: String,
    path: String,
    pose: PoseInput,
    side: String,
    offset: Vec3Input,
    rotation: Vec3Input,
    scale: Vec3Input,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct BoardInput {
    revision: u64,
    thickness: f32,
    contours: Vec<ContourInput>,
    surfaces: Vec<SurfaceInput>,
    holes: Vec<Vec<Vec2Input>>,
    models: Vec<PcbModelInput>,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct LoadedModelInput {
    id: String,
    mesh: MeshInput,
    #[serde(deserialize_with = "optional_float_buffer")]
    lines: Option<Vec<f32>>,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct BodyInput {
    color: Option<String>,
    id: String,
    name: String,
    mesh: MeshInput,
    #[serde(deserialize_with = "optional_float_buffer")]
    lines: Option<Vec<f32>>,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct ModuleBodyInput {
    id: String,
    name: String,
    z: f32,
    thickness: f32,
    contours: Vec<ContourInput>,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct BatteryInput {
    at: Vec2Input,
    size: Vec3Input,
    cable_exit: Vec2Input,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct ReferenceInput {
    pose: PoseInput,
    elevation: f32,
}

struct SceneObject {
    id: String,
    groups: Vec<String>,
    explode: f32,
    positions: Vec<Vec3>,
    normals: Vec<Vec3>,
    colors: Option<Vec<Srgba>>,
    indices: Vec<u32>,
    color: Srgba,
    roughness: f32,
    metallic: f32,
    edges: Option<MeshData>,
    lines: Vec<Vec3>,
}

struct WorldLines {
    context: three_d::core::Context,
    gl: WebGl2RenderingContext,
    positions: VertexBuffer<Vec3>,
    normals: VertexBuffer<Vec3>,
    vertex_count: u32,
    aabb: AxisAlignedBoundingBox,
    transformation: Mat4,
    vertex_shader: String,
}

impl<'a> IntoIterator for &'a WorldLines {
    type Item = &'a dyn Geometry;
    type IntoIter = std::iter::Once<&'a dyn Geometry>;

    fn into_iter(self) -> Self::IntoIter {
        std::iter::once(self)
    }
}

impl Geometry for WorldLines {
    fn aabb(&self) -> AxisAlignedBoundingBox {
        self.aabb.transformed(self.transformation)
    }

    fn animate(&mut self, _time: f32) {}

    fn draw(&self, viewer: &dyn Viewer, program: &Program, render_states: RenderStates) {
        if self.vertex_count == 0 {
            return;
        }
        program.use_uniform_if_required("normalMatrix", Mat4::identity());
        program.use_uniform("viewProjection", viewer.projection() * viewer.view());
        program.use_uniform("modelMatrix", self.transformation);
        program.use_vertex_attribute("position", &self.positions);
        if program.requires_attribute("normal") {
            program.use_vertex_attribute("normal", &self.normals);
        }
        program.draw_with(render_states, viewer.viewport(), || {
            self.gl
                .draw_arrays(WebGl2RenderingContext::LINES, 0, self.vertex_count as i32);
        });
    }

    fn vertex_shader_source(&self) -> String {
        self.vertex_shader.clone()
    }

    fn id(&self) -> GeometryId {
        GeometryId(1)
    }

    fn render_with_material(
        &self,
        material: &dyn Material,
        viewer: &dyn Viewer,
        lights: &[&dyn Light],
    ) {
        let _ =
            three_d::renderer::render_with_material(&self.context, viewer, &self, material, lights);
    }

    fn render_with_effect(
        &self,
        material: &dyn Effect,
        viewer: &dyn Viewer,
        lights: &[&dyn Light],
        color_texture: Option<ColorTexture>,
        depth_texture: Option<DepthTexture>,
    ) {
        let _ = three_d::renderer::render_with_effect(
            &self.context,
            viewer,
            self,
            material,
            lights,
            color_texture,
            depth_texture,
        );
    }
}

struct RenderObject {
    id: String,
    groups: Vec<String>,
    explode: f32,
    fingerprint: u64,
    visible: bool,
    triangles: Vec<[Vec3; 3]>,
    line_points: Vec<Vec3>,
    lines: Option<Gm<WorldLines, SectionMaterial>>,
    edges: Gm<Mesh, SectionMaterial>,
    object: Gm<Mesh, SectionMaterial>,
    /// Local pose, kept separate from exploded/state transforms so retained
    /// interactive objects can move without rebuilding their mesh.
    pose: Mat4,
    handle_length: Option<f32>,
    base_color: Srgba,
}

struct SectionMaterial {
    physical: PhysicalMaterial,
    section: Option<Vec4>,
    depth_only: bool,
}

impl Material for SectionMaterial {
    fn fragment_shader_source(&self, lights: &[&dyn three_d::Light]) -> String {
        self.physical
            .fragment_shader_source(lights)
            .replace(
                "in vec3 pos;",
                "in vec3 pos;\nuniform vec4 sectionPlane;\nuniform int sectionEnabled;",
            )
            .replace(
                "void main()\n{",
                "void main()\n{\n    if (sectionEnabled == 1 && dot(pos, sectionPlane.xyz) > sectionPlane.w) discard;",
            )
    }

    fn id(&self) -> EffectMaterialId {
        EffectMaterialId::PhysicalMaterialBase
    }

    fn use_uniforms(&self, program: &Program, viewer: &dyn Viewer, lights: &[&dyn three_d::Light]) {
        self.physical.use_uniforms(program, viewer, lights);
        program.use_uniform(
            "sectionPlane",
            self.section.unwrap_or(Vec4::new(0., 0., 0., 0.)),
        );
        program.use_uniform("sectionEnabled", i32::from(self.section.is_some()));
    }

    fn render_states(&self) -> RenderStates {
        let mut states = self.physical.render_states();
        if self.depth_only {
            states.write_mask = three_d::WriteMask::DEPTH;
        }
        states
    }
    fn material_type(&self) -> MaterialType {
        self.physical.material_type()
    }
}

#[wasm_bindgen]
pub struct Renderer {
    context: three_d::core::Context,
    gl: WebGl2RenderingContext,
    camera: Camera,
    light: DirectionalLight,
    fill: DirectionalLight,
    rim: DirectionalLight,
    state: DisplayState,
    ambient: AmbientLight,
    objects: Vec<RenderObject>,
    handles: Vec<RenderObject>,
    section_plane: Gm<Mesh, SectionMaterial>,
    width: u32,
    height: u32,
    target: Vec3,
    yaw: f32,
    pitch: f32,
    distance: f32,
    bounds_center: Vec3,
    bounds_radius: f32,
    focused_ids: Vec<String>,
    revision: u64,
}

#[wasm_bindgen]
impl Renderer {
    #[wasm_bindgen(constructor)]
    pub fn new(canvas: HtmlCanvasElement) -> Result<Renderer, JsValue> {
        let options = Object::new();
        Reflect::set(&options, &JsValue::from_str("alpha"), &JsValue::TRUE)?;
        Reflect::set(&options, &JsValue::from_str("antialias"), &JsValue::TRUE)?;
        Reflect::set(&options, &JsValue::from_str("depth"), &JsValue::TRUE)?;
        let gl = canvas
            .get_context_with_context_options("webgl2", options.as_ref())?
            .ok_or_else(|| JsValue::from_str("WebGL2 is unavailable"))?
            .dyn_into::<WebGl2RenderingContext>()
            .map_err(|_| JsValue::from_str("WebGL2 is unavailable"))?;
        let low_level = three_d::context::Context::from_webgl2_context(gl.clone());
        #[expect(
            clippy::arc_with_non_send_sync,
            reason = "three-d 0.19.0 requires Arc<Context>; WebGL2 context and renderer stay on the page thread"
        )]
        let context = three_d::core::Context::from_gl_context(Arc::new(low_level))
            .map_err(|error| JsValue::from_str(&error.to_string()))?;
        let viewport = Viewport::new_at_origo(1, 1);
        let mut renderer = Renderer {
            light: DirectionalLight::new(
                &context,
                2.4,
                Srgba::new(255, 244, 229, 255),
                Vec3::new(-1.0, 1.0, -2.0),
            ),
            fill: DirectionalLight::new(
                &context,
                0.65,
                Srgba::new(196, 219, 255, 255),
                Vec3::new(1.0, -0.4, -0.3),
            ),
            rim: DirectionalLight::new(&context, 1.1, Srgba::WHITE, Vec3::new(0.2, -1.0, 0.4)),
            ambient: AmbientLight::new(&context, 0.22, Srgba::WHITE),
            state: DisplayState::default(),
            camera: Camera::new_perspective(
                viewport,
                Vec3::new(40.0, 40.0, 30.0),
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 0.0, 1.0),
                three_d::radians(FOV_Y_DEGREES.to_radians()),
                0.01,
                100_000.0,
            ),
            section_plane: {
                let mut material = PhysicalMaterial::new_transparent(
                    &context,
                    &CpuMaterial {
                        albedo: Srgba::new(92, 165, 235, 55),
                        roughness: 1.,
                        ..CpuMaterial::default()
                    },
                );
                material.render_states.cull = Cull::None;
                material.render_states.write_mask = three_d::WriteMask::COLOR;
                Gm::new(
                    Mesh::new(&context, &CpuMesh::square()),
                    SectionMaterial {
                        physical: material,
                        section: None,
                        depth_only: false,
                    },
                )
            },
            context,
            gl,
            objects: Vec::new(),
            handles: Vec::new(),
            width: 1,
            height: 1,
            target: Vec3::new(0.0, 0.0, 0.0),
            yaw: 0.78,
            pitch: 0.55,
            distance: 100.0,
            bounds_center: Vec3::new(0.0, 0.0, 0.0),
            bounds_radius: 1.0,
            focused_ids: Vec::new(),
            revision: 0,
        };
        renderer.update_camera();
        Ok(renderer)
    }

    #[wasm_bindgen(js_name = resize)]
    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width.max(1);
        self.height = height.max(1);
        self.camera
            .set_viewport(Viewport::new_at_origo(self.width, self.height));
    }

    #[wasm_bindgen(js_name = setScene)]
    pub fn set_scene(&mut self, value: JsValue) -> Result<bool, JsValue> {
        let input: SceneInput = serde_wasm_bindgen::from_value(value)
            .map_err(|error| JsValue::from_str(&format!("Invalid 3D scene: {error}")))?;
        if is_stale_scene_revision(self.revision, input.revision) {
            return Ok(false);
        }
        self.accept_scene(build_scene(input)?)
    }

    pub fn set_state(&mut self, value: JsValue) -> Result<(), JsValue> {
        self.state =
            serde_wasm_bindgen::from_value(value).map_err(|e| JsValue::from_str(&e.to_string()))?;
        self.apply_state();
        Ok(())
    }

    fn projected_bounds(&self, predicate: impl Fn(&RenderObject) -> bool) -> (Vec3, Vec3) {
        bounds_of(
            self.objects
                .iter()
                .filter(|object| predicate(object))
                .flat_map(|object| {
                    let offset = exploded_view_offset(
                        &self.state.view,
                        object.explode,
                        self.state.explode_amount,
                    );
                    object
                        .triangles
                        .iter()
                        .flatten()
                        .chain(&object.line_points)
                        .map(move |point| {
                            (object.pose * point.extend(1.)).truncate() + Vec3::new(0., 0., offset)
                        })
                }),
        )
    }

    /// Focus the accepted geometry objects associated with a current finding.
    /// Focused objects remain visible while their saved display layer is hidden.
    #[wasm_bindgen(js_name = focusObjects)]
    pub fn focus_objects(&mut self, value: JsValue) -> Result<bool, JsValue> {
        let ids: Vec<String> = serde_wasm_bindgen::from_value(value)
            .map_err(|error| JsValue::from_str(&error.to_string()))?;
        let ids = ids
            .into_iter()
            .filter(|id| !id.is_empty())
            .collect::<Vec<_>>();
        if ids.is_empty() {
            self.focused_ids.clear();
            self.apply_state();
            return Ok(true);
        }
        let matches = |object: &&RenderObject| {
            ids.iter()
                .any(|id| object.id == *id || object.groups.iter().any(|group| group == id))
        };
        let focused_ids = self
            .objects
            .iter()
            .filter(matches)
            .map(|object| object.id.clone())
            .collect::<Vec<_>>();
        if focused_ids.is_empty() {
            return Ok(false);
        }
        let (low, high) = self.projected_bounds(|object| focused_ids.contains(&object.id));
        if !low.x.is_finite() {
            return Ok(false);
        }
        self.focused_ids = focused_ids;
        self.apply_state();
        let focus_center = (low + high) / 2.;
        let focus_radius = ((high - low).magnitude() / 2.).max(0.1);
        self.target = focus_center;
        self.distance = frame_distance(self.width, self.height, focus_radius, 1.3);
        self.update_camera();
        Ok(true)
    }

    #[wasm_bindgen(js_name = render)]
    pub fn render(&self) -> Result<(), JsValue> {
        let target = RenderTarget::screen(&self.context, self.width, self.height);
        target.clear(ClearState::color_and_depth(0., 0., 0., 0., 1.));
        let lights: [&dyn three_d::Light; 4] = [&self.light, &self.fill, &self.rim, &self.ambient];
        target.render(
            &self.camera,
            self.objects.iter().filter(|o| o.visible).map(|o| &o.object),
            &lights,
        );
        if self.state.mode != "shaded" || self.state.show_hidden {
            target.render(
                &self.camera,
                self.objects
                    .iter()
                    .filter(|o| {
                        o.visible
                            && !o
                                .groups
                                .iter()
                                .any(|g| matches!(g.as_str(), "Copper" | "Mask" | "Silkscreen"))
                    })
                    .map(|o| &o.edges),
                &lights,
            );
        } else {
            target.render(
                &self.camera,
                self.objects
                    .iter()
                    .filter(|o| o.visible && o.id == self.state.selected_layer)
                    .map(|o| &o.edges),
                &lights,
            );
        }
        target.render(
            &self.camera,
            self.objects
                .iter()
                .filter(|o| o.visible)
                .filter_map(|o| o.lines.as_ref()),
            &lights,
        );
        if self.state.view == "section" && self.state.show_section_plane {
            target.render(&self.camera, [&self.section_plane], &lights);
        }
        // Editing handles are an overlay: bottom-case mounts must remain
        // visible and reachable through the PCB and retained solids.
        target.clear(ClearState::depth(1.));
        target.render(
            &self.camera,
            self.handles.iter().filter(|o| o.visible).map(|o| &o.object),
            &lights,
        );
        if self.state.mode != "shaded" || self.state.show_hidden {
            target.render(
                &self.camera,
                self.handles.iter().filter(|o| o.visible).map(|o| &o.edges),
                &lights,
            );
        }
        Ok(())
    }

    #[wasm_bindgen(js_name = fit)]
    pub fn fit(&mut self) {
        // Fit restores the user's display projection before measuring the scene.
        self.focused_ids.clear();
        self.apply_state();
        // Hidden case bodies must not keep a small imported PCB off-centre.
        // Derive fit bounds on demand without rebuilding any geometry.
        let (low, high) = self.projected_bounds(|item| item.visible && item.id != "pcb-selection");
        if low.x.is_finite() {
            self.bounds_center = (low + high) / 2.;
            self.bounds_radius = ((high - low).magnitude() / 2.).max(0.1);
        }
        self.target = self.bounds_center;
        self.distance = frame_distance(self.width, self.height, self.bounds_radius, 1.16);
        self.yaw = 0.78;
        self.pitch = 0.55;
        self.update_camera();
    }

    #[wasm_bindgen(js_name = view)]
    pub fn view(&mut self, preset: &str) {
        match preset {
            "top" => {
                self.yaw = 0.0;
                self.pitch = 1.56;
                self.target = self.bounds_center;
            }
            "bottom" => {
                self.yaw = 0.0;
                self.pitch = -1.56;
                self.target = self.bounds_center;
            }
            "isometric" => {
                self.yaw = 0.78;
                self.pitch = 0.55;
                self.target = self.bounds_center;
            }
            _ => self.fit(),
        }
        self.update_camera();
    }

    #[wasm_bindgen(js_name = orbit)]
    pub fn orbit(&mut self, delta_x: f32, delta_y: f32) {
        self.yaw -= delta_x * 0.006;
        self.pitch = (self.pitch + delta_y * 0.006).clamp(-1.55, 1.55);
        self.update_camera();
    }

    #[wasm_bindgen(js_name = zoom)]
    pub fn zoom(&mut self, factor: f32) {
        if factor.is_finite() && factor > 0.0 {
            self.distance = (self.distance * factor)
                .clamp(self.bounds_radius * 0.08, self.bounds_radius * 100.0);
            self.update_camera();
        }
    }

    #[wasm_bindgen(js_name = setHandles)]
    pub fn set_handles(&mut self, value: JsValue) -> Result<u32, JsValue> {
        let handles: Vec<HandleInput> =
            serde_wasm_bindgen::from_value(value).map_err(|e| JsValue::from_str(&e.to_string()))?;
        let mut uploaded_count = 0;
        let mut previous = std::mem::take(&mut self.handles);
        let mut next = Vec::with_capacity(handles.len());
        for handle in handles {
            if !handle.length.is_finite() || handle.length <= 0.0 {
                continue;
            }
            let existing = previous
                .iter()
                .position(|item| item.id == handle.id)
                .map(|index| previous.swap_remove(index));
            let mut item = if let Some(item) =
                existing.filter(|item| item.handle_length == Some(handle.length))
            {
                item
            } else {
                let points = [
                    [-handle.length / 2., -0.7],
                    [handle.length / 2., -0.7],
                    [handle.length / 2., 0.7],
                    [-handle.length / 2., 0.7],
                ];
                let data = board_mesh(
                    &[BoardContour {
                        points: &points,
                        hole: false,
                    }],
                    0.6,
                )
                .map_err(|error| JsValue::from_str(&error))?;
                let mut built = BuiltScene {
                    objects: vec![],
                    bounds: None,
                    revision: 0,
                    keep_camera: true,
                };
                push_data(&mut built, &handle.id, data, [1., 0.65, 0.15, 1.], 0.6, 0.0);
                let mut object = built.objects.remove(0);
                object.groups = vec!["GasketHandles".into()];
                let mut uploaded = self.upload(object);
                uploaded_count += 1;
                uploaded.handle_length = Some(handle.length);
                uploaded
            };
            item.pose = crate::math::handle_pose(
                [handle.at.x, handle.at.y, handle.z],
                [handle.tangent.x, handle.tangent.y],
                [handle.normal.x, handle.normal.y],
            );
            item.object.material.physical.albedo = if handle.invalid {
                Srgba::new(230, 31, 26, 255)
            } else {
                Srgba::new(255, 166, 38, 255)
            };
            next.push(item);
        }
        self.handles = next;
        self.apply_state();
        Ok(uploaded_count)
    }

    #[wasm_bindgen(js_name = pointOnPlane)]
    pub fn point_on_plane(&self, x: f32, y: f32, z: f32) -> Float32Array {
        let (origin, direction) = self.ray(x, y);
        if direction.z.abs() < 0.00001 {
            return Float32Array::new_with_length(0);
        }
        let p = origin + direction * ((z - origin.z) / direction.z);
        Float32Array::from([p.x, p.y, p.z].as_slice())
    }

    #[wasm_bindgen(js_name = pick)]
    pub fn pick(&self, x: f32, y: f32) -> Option<String> {
        let section = (self.state.view == "section").then(|| self.section_equation().0);
        let (origin, direction) = self.ray(x, y);
        let mut nearest = f32::INFINITY;
        let mut picked = None;
        let mut picked_handle = false;
        for object in self
            .handles
            .iter()
            .chain(&self.objects)
            .filter(|o| o.visible && o.id != "pcb-selection")
        {
            if picked_handle && object.handle_length.is_none() {
                break;
            }
            let offset =
                exploded_view_offset(&self.state.view, object.explode, self.state.explode_amount);
            let Some(inverse) = object.pose.invert() else {
                continue;
            };
            let origin = (inverse * (origin - Vec3::new(0., 0., offset)).extend(1.)).truncate();
            let direction = (inverse * direction.extend(0.)).truncate();
            for triangle in &object.triangles {
                let a = triangle[1] - triangle[0];
                let b = triangle[2] - triangle[0];
                let h = direction.cross(b);
                let det = a.dot(h);
                if det.abs() < 0.000001 {
                    continue;
                }
                let delta = origin - triangle[0];
                let u = delta.dot(h) / det;
                if !(0.0..=1.0).contains(&u) {
                    continue;
                }
                let q = delta.cross(a);
                let v = direction.dot(q) / det;
                if v < 0. || u + v > 1. {
                    continue;
                }
                let distance = b.dot(q) / det;
                if distance < 0. || distance >= nearest {
                    continue;
                }
                let p = origin + direction * distance;
                if section.is_some_and(|plane| {
                    let world = (object.pose * p.extend(1.)).truncate();
                    world.dot(plane.truncate()) > plane.w
                }) {
                    continue;
                }
                nearest = distance;
                picked = Some(object.id.clone());
                picked_handle = object.handle_length.is_some();
            }
        }
        picked
    }

    #[wasm_bindgen(js_name = dispose)]
    pub fn dispose(&mut self) {
        self.objects.clear();
        self.handles.clear();
        // Cached Programs hold cloned Contexts. Empty the cache to break that
        // ownership cycle before the renderer's final Context is dropped.
        self.context.programs.write().unwrap().clear();
    }
}

#[wasm_bindgen(js_name = decodeStl)]
pub fn decode_stl_wasm(bytes: &[u8]) -> Result<JsValue, JsValue> {
    encode_mesh(decode_stl(bytes).map_err(|error| JsValue::from_str(&error))?)
}

#[wasm_bindgen(js_name = decodeWrl)]
pub fn decode_wrl_wasm(bytes: &[u8]) -> Result<JsValue, JsValue> {
    encode_mesh(decode_wrl(bytes).map_err(|error| JsValue::from_str(&error))?)
}

fn encode_mesh(mesh: ModelMesh) -> Result<JsValue, JsValue> {
    let output = Object::new();
    Reflect::set(
        &output,
        &JsValue::from_str("positions"),
        &Float32Array::from(mesh.positions.as_slice()),
    )?;
    Reflect::set(
        &output,
        &JsValue::from_str("normals"),
        &Float32Array::from(mesh.normals.as_slice()),
    )?;
    if let Some(colors) = mesh.colors {
        Reflect::set(
            &output,
            &JsValue::from_str("colors"),
            &Float32Array::from(colors.as_slice()),
        )?;
    }
    Ok(output.into())
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct DisplayState {
    explode_amount: Option<f32>,
    section_plane: String,
    section_position: f32,
    show_section_plane: bool,
    show_hidden: bool,
    colors: std::collections::HashMap<String, String>,
    hidden: Vec<String>,
    selected_layer: String,
    view: String,
    mode: String,
    theme: String,
}

fn mesh_fingerprint(object: &SceneObject) -> u64 {
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    for p in object
        .positions
        .iter()
        .chain(&object.normals)
        .chain(&object.lines)
    {
        for v in [p.x, p.y, p.z] {
            v.to_bits().hash(&mut hash);
        }
    }
    object.indices.hash(&mut hash);
    object.roughness.to_bits().hash(&mut hash);
    object.metallic.to_bits().hash(&mut hash);
    if let Some(colors) = &object.colors {
        for c in colors {
            [c.r, c.g, c.b, c.a].hash(&mut hash);
        }
    }
    [
        object.color.r,
        object.color.g,
        object.color.b,
        object.color.a,
    ]
    .hash(&mut hash);
    hash.finish()
}
fn batch_surfaces(objects: &mut Vec<SceneObject>) {
    let mut result: Vec<SceneObject> = Vec::new();
    for item in objects.drain(..) {
        if item
            .groups
            .iter()
            .any(|g| matches!(g.as_str(), "Copper" | "Mask" | "Silkscreen"))
            && let Some(target) = result.iter_mut().find(|o| {
                o.groups == item.groups
                    && o.color == item.color
                    && o.colors.is_none()
                    && item.colors.is_none()
            })
        {
            let offset = target.positions.len() as u32;
            target.positions.extend(item.positions);
            target.normals.extend(item.normals);
            target
                .indices
                .extend(item.indices.iter().map(|i| i + offset));
            continue;
        }
        result.push(item);
    }
    *objects = result;
}
fn edge_mesh(mesh: &MeshData) -> MeshData {
    let mut result = MeshData::default();
    for [a, b] in feature_edges(mesh) {
        let a = Vec3::from(a);
        let b = Vec3::from(b);
        let direction = (b - a).normalize();
        let axis = if direction.z.abs() < 0.9 {
            Vec3::unit_z()
        } else {
            Vec3::unit_y()
        };
        let perpendicular = direction.cross(axis).normalize() * 0.055;
        let other = direction.cross(perpendicular).normalize() * 0.055;
        for delta in [perpendicular, other] {
            let start = result.positions.len() as u32;
            let normal = direction.cross(delta).normalize();
            for p in [a - delta, b - delta, b + delta, a + delta] {
                result.positions.push([p.x, p.y, p.z]);
                result.normals.push([normal.x, normal.y, normal.z]);
            }
            result
                .indices
                .extend([start, start + 1, start + 2, start, start + 2, start + 3]);
        }
    }
    result
}

#[derive(Deserialize)]
struct HandleInput {
    id: String,
    at: Vec2Input,
    tangent: Vec2Input,
    normal: Vec2Input,
    length: f32,
    z: f32,
    #[serde(default)]
    invalid: bool,
}

/// Full vertical field of view shared by the perspective camera and fit framing.
const FOV_Y_DEGREES: f32 = 34.0;

/// Framing distance for a bounding radius, matching the camera projection.
fn frame_distance(width: u32, height: u32, radius: f32, margin: f32) -> f32 {
    let vertical = (FOV_Y_DEGREES * 0.5).to_radians();
    let horizontal = (vertical.tan() * width as f32 / height.max(1) as f32).atan();
    radius / vertical.min(horizontal).sin() * margin
}

/// Z lift of one object in the exploded view; assembled views stay at zero.
fn exploded_view_offset(view: &str, explode: f32, amount: Option<f32>) -> f32 {
    if view == "exploded" {
        explode * amount.unwrap_or(1.)
    } else {
        0.
    }
}

/// Fold transformed points into an axis-aligned bounding box.
fn bounds_of(points: impl IntoIterator<Item = Vec3>) -> (Vec3, Vec3) {
    let mut low = Vec3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY);
    let mut high = Vec3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY);
    for point in points {
        low.x = low.x.min(point.x);
        low.y = low.y.min(point.y);
        low.z = low.z.min(point.z);
        high.x = high.x.max(point.x);
        high.y = high.y.max(point.y);
        high.z = high.z.max(point.z);
    }
    (low, high)
}

fn hex_rgb(value: &str) -> Option<[u8; 3]> {
    if value.len() != 7 || !value.starts_with('#') {
        return None;
    }
    let rgb = u32::from_str_radix(&value[1..], 16).ok()?;
    Some([(rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8])
}

fn hex_color(value: &str) -> Option<[f32; 4]> {
    let [red, green, blue] = hex_rgb(value)?;
    Some([
        red as f32 / 255.0,
        green as f32 / 255.0,
        blue as f32 / 255.0,
        1.0,
    ])
}
