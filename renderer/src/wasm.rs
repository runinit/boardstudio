use crate::{
    ModelMesh, decode_stl, decode_wrl,
    geometry::{
        BoardContour, MeshData, SurfaceClipper, board_mesh, feature_edges, polygon_area,
        stroke_mesh, surface_mesh,
    },
    is_stale_scene_revision,
    math::{component_model_transform, pcb_model_transform},
};
// Keep stack helpers private while exercising the same source in native tests.
include!("mechanical.rs");
use js_sys::{Float32Array, Object, Reflect};
use serde::Deserialize;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use three_d::{
    AmbientLight, Camera, ClearState, CpuMaterial, CpuMesh, Cull, DirectionalLight,
    EffectMaterialId, Gm, Indices, InnerSpace, Mat4, Material, MaterialType, Matrix, Mesh,
    PhysicalMaterial, Positions, Program, RenderStates, RenderTarget, SquareMatrix, Srgba, Vec3,
    Vec4, Viewer, Viewport,
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
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct BodyInput {
    color: Option<String>,
    id: String,
    name: String,
    mesh: MeshInput,
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
}

struct RenderObject {
    id: String,
    groups: Vec<String>,
    explode: f32,
    fingerprint: u64,
    visible: bool,
    triangles: Vec<[Vec3; 3]>,
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
    section_x: Option<f32>,
}

#[wasm_bindgen]
impl Renderer {
    #[wasm_bindgen(constructor)]
    pub fn new(canvas: HtmlCanvasElement) -> Result<Renderer, JsValue> {
        let options = Object::new();
        Reflect::set(&options, &JsValue::from_str("alpha"), &JsValue::TRUE)?;
        Reflect::set(&options, &JsValue::from_str("antialias"), &JsValue::TRUE)?;
        Reflect::set(&options, &JsValue::from_str("depth"), &JsValue::TRUE)?;
        let context = canvas
            .get_context_with_context_options("webgl2", options.as_ref())?
            .ok_or_else(|| JsValue::from_str("WebGL2 is unavailable"))?
            .dyn_into::<WebGl2RenderingContext>()
            .map_err(|_| JsValue::from_str("WebGL2 is unavailable"))?;
        let low_level = three_d::context::Context::from_webgl2_context(context);
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
                three_d::radians(34.0_f32.to_radians()),
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
            section_x: None,
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

    #[wasm_bindgen(js_name = setPreparedScene)]
    pub fn set_prepared_scene(&mut self, value: JsValue) -> Result<bool, JsValue> {
        let input: PreparedScene =
            serde_wasm_bindgen::from_value(value).map_err(|e| JsValue::from_str(&e.to_string()))?;
        if is_stale_scene_revision(self.revision, input.revision) {
            return Ok(false);
        }
        let mut built = BuiltScene {
            objects: vec![],
            bounds: input.bounds.map(|b| (Vec3::new(b[0], b[1], b[2]), b[3])),
            section_x: input.section_x,
            revision: input.revision,
            keep_camera: input.keep_camera,
        };
        for object in input.objects {
            push_mesh(
                &mut built,
                &object.id,
                object.mesh,
                Mat4::identity(),
                object.color,
                object.roughness,
                object.metallic,
            )?;
            let item = built.objects.last_mut().unwrap();
            item.groups = object.groups;
            item.explode = object.explode;
            item.edges = Some(MeshData {
                positions: object
                    .edges
                    .positions
                    .as_chunks::<3>()
                    .0
                    .iter()
                    .map(|p| [p[0], p[1], p[2]])
                    .collect(),
                normals: object
                    .edges
                    .normals
                    .as_chunks::<3>()
                    .0
                    .iter()
                    .map(|p| [p[0], p[1], p[2]])
                    .collect(),
                indices: (0..object.edges.positions.len() as u32 / 3).collect(),
                colors: None,
            });
        }
        self.accept_scene(built)
    }

    /// Applies only the changed scene objects.  The client uses stable object
    /// ids, so unchanged PCB and model meshes remain resident in the GPU.
    #[wasm_bindgen(js_name = setPreparedScenePatch)]
    pub fn set_prepared_scene_patch(&mut self, value: JsValue) -> Result<bool, JsValue> {
        let input: PreparedScenePatch =
            serde_wasm_bindgen::from_value(value).map_err(|e| JsValue::from_str(&e.to_string()))?;
        if is_stale_scene_revision(self.revision, input.revision) {
            return Ok(false);
        }
        let mut changed = BuiltScene {
            objects: vec![],
            bounds: input.bounds.map(|b| (Vec3::new(b[0], b[1], b[2]), b[3])),
            section_x: input.section_x,
            revision: input.revision,
            keep_camera: true,
        };
        for object in input.objects {
            push_mesh(
                &mut changed,
                &object.id,
                object.mesh,
                Mat4::identity(),
                object.color,
                object.roughness,
                object.metallic,
            )?;
            let item = changed.objects.last_mut().unwrap();
            item.groups = object.groups;
            item.explode = object.explode;
            item.edges = Some(MeshData {
                positions: object
                    .edges
                    .positions
                    .as_chunks::<3>()
                    .0
                    .iter()
                    .map(|p| [p[0], p[1], p[2]])
                    .collect(),
                normals: object
                    .edges
                    .normals
                    .as_chunks::<3>()
                    .0
                    .iter()
                    .map(|p| [p[0], p[1], p[2]])
                    .collect(),
                indices: (0..object.edges.positions.len() as u32 / 3).collect(),
                colors: None,
            });
        }
        self.accept_scene_patch(changed, input.removed)
    }

    #[wasm_bindgen(js_name = setState)]
    pub fn set_state(&mut self, value: JsValue) -> Result<(), JsValue> {
        self.state =
            serde_wasm_bindgen::from_value(value).map_err(|e| JsValue::from_str(&e.to_string()))?;
        self.apply_state();
        Ok(())
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
        let mut low = Vec3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY);
        let mut high = Vec3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY);
        for object in self
            .objects
            .iter()
            .filter(|object| focused_ids.contains(&object.id))
        {
            let offset = if self.state.view == "exploded" {
                object.explode * self.state.explode_amount.unwrap_or(1.)
            } else {
                0.
            };
            for point in object.triangles.iter().flatten() {
                let point = (object.pose * point.extend(1.)).truncate() + Vec3::new(0., 0., offset);
                low.x = low.x.min(point.x);
                low.y = low.y.min(point.y);
                low.z = low.z.min(point.z);
                high.x = high.x.max(point.x);
                high.y = high.y.max(point.y);
                high.z = high.z.max(point.z);
            }
        }
        if !low.x.is_finite() {
            return Ok(false);
        }
        self.focused_ids = focused_ids;
        self.apply_state();
        let focus_center = (low + high) / 2.;
        let focus_radius = ((high - low).magnitude() / 2.).max(0.1);
        self.target = focus_center;
        let vertical = 17.0_f32.to_radians();
        let horizontal = (vertical.tan() * self.width as f32 / self.height.max(1) as f32).atan();
        self.distance = focus_radius / vertical.min(horizontal).sin() * 1.3;
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
        let mut low = Vec3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY);
        let mut high = Vec3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY);
        for object in self
            .objects
            .iter()
            .filter(|item| item.visible && item.id != "pcb-selection")
        {
            let offset = if self.state.view == "exploded" {
                object.explode * self.state.explode_amount.unwrap_or(1.)
            } else {
                0.
            };
            for point in object.triangles.iter().flatten() {
                let point = (object.pose * point.extend(1.)).truncate() + Vec3::new(0., 0., offset);
                low.x = low.x.min(point.x);
                low.y = low.y.min(point.y);
                low.z = low.z.min(point.z);
                high.x = high.x.max(point.x);
                high.y = high.y.max(point.y);
                high.z = high.z.max(point.z);
            }
        }
        if low.x.is_finite() {
            self.bounds_center = (low + high) / 2.;
            self.bounds_radius = ((high - low).magnitude() / 2.).max(0.1);
        }
        self.target = self.bounds_center;
        let vertical = 17.0_f32.to_radians();
        let horizontal = (vertical.tan() * self.width as f32 / self.height.max(1) as f32).atan();
        self.distance = self.bounds_radius / vertical.min(horizontal).sin() * 1.16;
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
                    section_x: None,
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
            let offset = if self.state.view == "exploded" {
                object.explode * self.state.explode_amount.unwrap_or(1.)
            } else {
                0.
            };
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

impl Renderer {
    fn accept_scene_patch(
        &mut self,
        items: BuiltScene,
        removed: Vec<String>,
    ) -> Result<bool, JsValue> {
        let removed: std::collections::HashSet<_> = removed.into_iter().collect();
        let mut previous = std::mem::take(&mut self.objects);
        previous.retain(|item| !removed.contains(&item.id));
        let mut next = Vec::with_capacity(previous.len() + items.objects.len());
        for item in items.objects {
            let fingerprint = mesh_fingerprint(&item);
            if let Some(index) = previous.iter().position(|old| old.id == item.id) {
                let mut old = previous.swap_remove(index);
                if old.groups == item.groups && old.fingerprint == fingerprint {
                    old.explode = item.explode;
                    next.push(old);
                } else {
                    next.push(self.upload(item));
                }
            } else {
                next.push(self.upload(item));
            }
        }
        next.extend(previous);
        self.objects = next;
        self.revision = items.revision;
        // A body patch does not describe complete scene bounds.
        self.apply_state();
        self.update_camera();
        Ok(true)
    }

    fn accept_scene(&mut self, items: BuiltScene) -> Result<bool, JsValue> {
        self.focused_ids.clear();
        self.section_x = items.section_x;
        let mut previous = std::mem::take(&mut self.objects);
        self.objects = items
            .objects
            .into_iter()
            .map(|item| {
                let fingerprint = mesh_fingerprint(&item);
                if let Some(index) = previous.iter().position(|old| {
                    old.id == item.id && old.groups == item.groups && old.fingerprint == fingerprint
                }) {
                    let mut old = previous.swap_remove(index);
                    old.explode = item.explode;
                    old
                } else {
                    self.upload(item)
                }
            })
            .collect();
        self.revision = items.revision;
        if let Some((center, radius)) = items.bounds {
            self.bounds_center = center;
            self.bounds_radius = radius.max(0.1);
        }
        self.apply_state();
        if !items.keep_camera {
            self.fit();
        } else {
            self.update_camera();
        }
        Ok(true)
    }
    fn section_equation(&self) -> (Vec4, Vec3, f32) {
        let mut low = Vec3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY);
        let mut high = Vec3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY);
        for object in &self.objects {
            for point in object.triangles.iter().flatten() {
                let p = (object.pose * point.extend(1.)).truncate();
                low.x = low.x.min(p.x);
                low.y = low.y.min(p.y);
                low.z = low.z.min(p.z);
                high.x = high.x.max(p.x);
                high.y = high.y.max(p.y);
                high.z = high.z.max(p.z);
            }
        }
        if !low.x.is_finite() {
            low = Vec3::new(-1., -1., -1.);
            high = -low;
        }
        let (equation, center) = section_location(
            [low.x, low.y, low.z],
            [high.x, high.y, high.z],
            &self.state.section_plane,
            self.state.section_position,
        );
        (
            Vec4::new(equation[0], equation[1], equation[2], equation[3]),
            Vec3::new(center[0], center[1], center[2]),
            (high - low).magnitude() * 0.6,
        )
    }
    fn apply_state(&mut self) {
        let (equation, center, radius) = self.section_equation();
        let rotation = match self.state.section_plane.as_str() {
            "XY" => Mat4::identity(),
            "XZ" => Mat4::from_angle_x(three_d::degrees(90.)),
            _ => Mat4::from_angle_y(three_d::degrees(90.)),
        };
        self.section_plane.geometry.set_transformation(
            Mat4::from_translation(center) * rotation * Mat4::from_scale(radius),
        );
        for item in self.objects.iter_mut().chain(&mut self.handles) {
            item.visible = self.focused_ids.contains(&item.id)
                || !self.state.hidden.iter().any(|g| {
                    g == "Assembly"
                        || g == &item.id
                        || item.groups.contains(g)
                        || (g == "Gaskets"
                            && (item.id.starts_with("gasket:")
                                || item.id.starts_with("gasket-handle:")))
                        || (item.id.starts_with("gasket-handle:")
                            && g == &format!(
                                "{}:lower",
                                item.id.replacen("gasket-handle:", "gasket:", 1)
                            ))
                });
            let color = self
                .state
                .colors
                .get(&item.id)
                .or_else(|| {
                    item.groups
                        .iter()
                        .find_map(|group| self.state.colors.get(group))
                })
                .or_else(|| {
                    item.id
                        .starts_with("gasket:")
                        .then(|| self.state.colors.get("Gaskets"))
                        .flatten()
                });
            item.object.material.physical.albedo = color
                .and_then(|value| u32::from_str_radix(value.trim_start_matches('#'), 16).ok())
                .map(|rgb| {
                    Srgba::new(
                        (rgb >> 16) as u8,
                        (rgb >> 8) as u8,
                        rgb as u8,
                        item.base_color.a,
                    )
                })
                .unwrap_or(item.base_color);
            item.edges.material.physical.render_states.depth_test = if self.state.show_hidden {
                three_d::DepthTest::Always
            } else {
                three_d::DepthTest::LessOrEqual
            };
            item.edges.material.physical.render_states.write_mask = three_d::WriteMask::COLOR;
            if item.id == "pcb-selection" {
                item.visible &= self.state.selected_layer == "pcb";
            }
            let transform = Mat4::from_translation(Vec3::new(
                0.,
                0.,
                if self.state.view == "exploded" {
                    item.explode * self.state.explode_amount.unwrap_or(1.)
                } else {
                    0.
                },
            )) * item.pose;
            item.object.geometry.set_transformation(transform);
            item.edges.geometry.set_transformation(transform);
            let clip = if self.state.view == "section" {
                Some(equation)
            } else {
                None
            };
            item.object.material.section = clip;
            item.edges.material.section = clip;
            item.object.material.depth_only = self.state.mode == "wireframe";
            let selected =
                item.id == self.state.selected_layer || self.focused_ids.contains(&item.id);
            item.edges.material.physical.albedo = Srgba::BLACK;
            item.edges.material.physical.emissive = if selected {
                Srgba::new(255, 186, 58, 255)
            } else if self.state.mode == "wireframe" && self.state.theme == "dark" {
                Srgba::new(180, 205, 215, 255)
            } else {
                Srgba::new(30, 42, 48, 255)
            };
        }
    }
    fn ray(&self, x: f32, y: f32) -> (Vec3, Vec3) {
        let inverse = (self.camera.projection() * self.camera.view())
            .invert()
            .unwrap_or_else(Mat4::identity);
        let point = |z| {
            let p = inverse
                * Vec4::new(
                    2. * x / self.width as f32 - 1.,
                    1. - 2. * y / self.height as f32,
                    z,
                    1.,
                );
            p.truncate() / p.w
        };
        let start = point(-1.);
        (start, (point(1.) - start).normalize())
    }
    fn update_camera(&mut self) {
        let horizontal = self.pitch.cos();
        let direction = Vec3::new(
            self.yaw.cos() * horizontal,
            self.yaw.sin() * horizontal,
            self.pitch.sin(),
        );
        let up = if direction.z.abs() > 0.98 {
            Vec3::new(0.0, 1.0, 0.0)
        } else {
            Vec3::new(0.0, 0.0, 1.0)
        };
        let radius = self.bounds_radius.max(0.1);
        let near = (self.distance - radius * 1.5).max(radius * 0.001).max(0.01);
        let far = (self.distance + radius * 1.5).max(near + radius);
        self.light.direction = (-direction + Vec3::new(-0.5, 0.2, -0.7)).normalize();
        self.fill.direction = (-direction + Vec3::new(0.8, -0.6, 0.1)).normalize();
        self.camera = Camera::new_perspective(
            Viewport::new_at_origo(self.width, self.height),
            self.target + direction * self.distance,
            self.target,
            up,
            three_d::radians(34.0_f32.to_radians()),
            near,
            far,
        );
    }

    fn upload(&self, item: SceneObject) -> RenderObject {
        let fingerprint = mesh_fingerprint(&item);
        let triangles = item
            .indices
            .as_chunks::<3>()
            .0
            .iter()
            .map(|t| {
                [
                    item.positions[t[0] as usize],
                    item.positions[t[1] as usize],
                    item.positions[t[2] as usize],
                ]
            })
            .collect();
        let data = MeshData {
            positions: item.positions.iter().map(|p| [p.x, p.y, p.z]).collect(),
            normals: vec![],
            indices: item.indices.clone(),
            colors: None,
        };
        let edge_data = item.edges.unwrap_or_else(|| edge_mesh(&data));
        let edge_cpu = CpuMesh {
            positions: Positions::F32(
                edge_data
                    .positions
                    .iter()
                    .map(|p| Vec3::new(p[0], p[1], p[2]))
                    .collect(),
            ),
            indices: Indices::U32(edge_data.indices),
            normals: Some(
                edge_data
                    .normals
                    .iter()
                    .map(|p| Vec3::new(p[0], p[1], p[2]))
                    .collect(),
            ),
            ..CpuMesh::default()
        };
        let mut edge_material = PhysicalMaterial::new_opaque(
            &self.context,
            &CpuMaterial {
                albedo: Srgba::new(30, 42, 48, 255),
                roughness: 1.0,
                ..CpuMaterial::default()
            },
        );
        edge_material.render_states.cull = Cull::None;
        let edges = Gm::new(
            Mesh::new(&self.context, &edge_cpu),
            SectionMaterial {
                physical: edge_material,
                section: None,
                depth_only: false,
            },
        );
        let cpu_mesh = CpuMesh {
            positions: Positions::F32(item.positions),
            indices: Indices::U32(item.indices),
            normals: Some(item.normals),
            colors: item.colors,
            ..CpuMesh::default()
        };
        let cpu_material = CpuMaterial {
            albedo: if cpu_mesh.colors.is_some() {
                Srgba::WHITE
            } else {
                item.color
            },
            roughness: item.roughness,
            metallic: item.metallic,
            ..CpuMaterial::default()
        };
        let physical = if item.color.a < 255 {
            PhysicalMaterial::new_transparent(&self.context, &cpu_material)
        } else {
            PhysicalMaterial::new_opaque(&self.context, &cpu_material)
        };
        let mut material = SectionMaterial {
            physical,
            section: None,
            depth_only: false,
        };
        material.physical.render_states.cull = Cull::Back;
        RenderObject {
            id: item.id,
            groups: item.groups,
            explode: item.explode,
            fingerprint,
            visible: true,
            triangles,
            edges,
            object: Gm::new(Mesh::new(&self.context, &cpu_mesh), material),
            pose: Mat4::identity(),
            handle_length: None,
            base_color: cpu_material.albedo,
        }
    }
}

struct BuiltScene {
    objects: Vec<SceneObject>,
    bounds: Option<(Vec3, f32)>,
    section_x: Option<f32>,
    revision: u64,
    keep_camera: bool,
}

fn build_scene(input: SceneInput) -> Result<BuiltScene, JsValue> {
    let mut output = BuiltScene {
        objects: Vec::new(),
        bounds: None,
        section_x: None,
        revision: input.revision,
        keep_camera: input.keep_camera,
    };
    if input.kind == "case" {
        if let Some(mesh) = input.case_mesh {
            push_mesh(
                &mut output,
                "case",
                mesh,
                Mat4::identity(),
                [0.72, 0.70, 0.65, 1.0],
                0.74,
                0.04,
            )?;
        }
        for (index, component) in input.component_previews.into_iter().enumerate() {
            let back = component.side == "back";
            let pose = Mat4::from_translation(Vec3::new(
                component.pose.at.x,
                component.pose.at.y,
                if back { 0.0 } else { input.board_thickness },
            )) * Mat4::from_angle_z(three_d::Rad(component.pose.rotation.to_radians()));
            let model = component_model_transform(
                [
                    component.model.offset.x,
                    component.model.offset.y,
                    component.model.offset.z,
                ],
                [
                    component.model.rotation.x,
                    component.model.rotation.y,
                    component.model.rotation.z,
                ],
                [
                    component.model.scale.x,
                    component.model.scale.y,
                    component.model.scale.z,
                ],
                back,
            );
            push_mesh(
                &mut output,
                if component.reference.is_empty() {
                    &component.id
                } else {
                    &component.reference
                },
                component.mesh,
                pose * model,
                if index % 2 == 0 {
                    [0.62, 0.67, 0.74, 1.0]
                } else {
                    [0.73, 0.72, 0.68, 1.0]
                },
                0.62,
                0.08,
            )?;
        }
    } else if input.kind == "bodyPatch" {
        for body in input.bodies {
            let color = if body.id.to_ascii_lowercase().contains("foam")
                || body.id.to_ascii_lowercase().contains("gasket")
            {
                [0.20, 0.18, 0.24, 1.0]
            } else if body.id.to_ascii_lowercase().contains("plate") {
                [0.34, 0.57, 0.44, 1.0]
            } else if input.theme == "dark" {
                [0.62, 0.66, 0.72, 1.0]
            } else {
                [0.68, 0.71, 0.75, 1.0]
            };
            let color = body.color.as_deref().and_then(hex_color).unwrap_or(color);
            push_mesh(
                &mut output,
                &body.id,
                body.mesh,
                Mat4::identity(),
                color,
                0.68,
                0.03,
            )?;
        }
    } else if let Some(board) = input.board {
        let hidden = |id: &str| input.hidden.iter().any(|entry| entry == id);
        let contours = board
            .contours
            .iter()
            .map(|contour| {
                (
                    contour
                        .points
                        .iter()
                        .map(|point| [point.x, point.y])
                        .collect::<Vec<_>>(),
                    contour.hole,
                )
            })
            .collect::<Vec<_>>();
        let holes = board
            .holes
            .iter()
            .map(|hole| {
                hole.iter()
                    .map(|point| [point.x, point.y])
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let mut contour_refs = contours
            .iter()
            .map(|(points, hole)| BoardContour {
                points,
                hole: *hole,
            })
            .collect::<Vec<_>>();
        contour_refs.extend(
            holes
                .iter()
                .map(|points| BoardContour { points, hole: true }),
        );
        let board_area = contours
            .iter()
            .map(|(points, hole)| polygon_area(points) * if *hole { -1.0 } else { 1.0 })
            .sum::<f32>()
            - holes.iter().map(|hole| polygon_area(hole)).sum::<f32>();
        let board_area = board_area.max(1.0);
        let board_bounds =
            contours
                .iter()
                .flat_map(|(points, _)| points)
                .fold(None, |bounds, point| {
                    Some(bounds.map_or(
                        (point[0], point[0], point[1], point[1]),
                        |(low_x, high_x, low_y, high_y): (f32, f32, f32, f32)| {
                            (
                                low_x.min(point[0]),
                                high_x.max(point[0]),
                                low_y.min(point[1]),
                                high_y.max(point[1]),
                            )
                        },
                    ))
                });
        let center_x = board_bounds.map_or(0.0, |(low_x, high_x, low_y, high_y)| {
            let local_x = (low_x + high_x) / 2.0;
            let local_y = (low_y + high_y) / 2.0;
            input.reference.as_ref().map_or(local_x, |reference| {
                let radians = reference.pose.rotation.to_radians();
                reference.pose.at.x + local_x * radians.cos() - local_y * radians.sin()
            })
        });
        let board_base_z = input
            .reference
            .as_ref()
            .map_or(0.0, |reference| reference.elevation)
            - if !input.mechanical_stack.is_empty() {
                board.thickness
            } else {
                0.0
            }
            + if input.view == "exploded" {
                explode_offset(stack_index(&input.mechanical_stack, "pcb"))
            } else {
                0.0
            };
        let reference_transform = input
            .reference
            .as_ref()
            .map(|reference| {
                Mat4::from_translation(Vec3::new(
                    reference.pose.at.x,
                    reference.pose.at.y,
                    reference.elevation,
                )) * Mat4::from_angle_z(three_d::Rad(reference.pose.rotation.to_radians()))
            })
            .unwrap_or_else(Mat4::identity);
        let pcb_transform = reference_transform
            * Mat4::from_translation(Vec3::new(
                0.0,
                0.0,
                board_base_z
                    - input
                        .reference
                        .as_ref()
                        .map_or(0.0, |reference| reference.elevation),
            ));
        if !hidden("PCB") {
            let mut board_mesh = board_mesh(&contour_refs, board.thickness)
                .map_err(|error| JsValue::from_str(&error))?;
            board_mesh.colors = Some(
                board_mesh
                    .normals
                    .iter()
                    .map(|normal| {
                        if normal[2].abs() > 0.5 {
                            [0.12, 0.36, 0.22]
                        } else {
                            [0.49, 0.39, 0.24]
                        }
                    })
                    .collect(),
            );
            translate_mesh(&mut board_mesh, pcb_transform);
            push_data(
                &mut output,
                "pcb",
                board_mesh,
                [1.0, 1.0, 1.0, 1.0],
                0.82,
                0.0,
            );
        }
        let clipper = SurfaceClipper::new(&contour_refs, &holes);
        for surface in &board.surfaces {
            let kind = if surface.layer.ends_with(".Cu") {
                "Copper"
            } else if surface.layer.ends_with(".Mask") {
                "Mask"
            } else if surface.layer.ends_with(".SilkS") {
                "Silkscreen"
            } else {
                continue;
            };
            if hidden("PCB") || hidden(kind) {
                continue;
            }
            let back = surface.layer.starts_with("B.");
            let points = surface
                .points
                .iter()
                .map(|point| [point.x, point.y])
                .collect::<Vec<_>>();
            // Large filled copper polygons are normally covered by solder mask.
            // Keep them under the green board color so they do not paint a whole
            // copper zone gold over traces, pads, and silkscreen.
            let covered_copper =
                kind == "Copper" && surface.filled && polygon_area(&points) >= board_area * 0.08;
            let layer_offset = match kind {
                "Copper" if covered_copper => 0.02,
                "Copper" => 0.04,
                "Mask" => 0.06,
                _ => 0.08,
            };
            let z = if back {
                -layer_offset
            } else {
                board.thickness + layer_offset
            };
            let normal = if back {
                [0.0, 0.0, -1.0]
            } else {
                [0.0, 0.0, 1.0]
            };
            let geometry = if !surface.text.is_empty() {
                text_mesh(
                    &surface.text,
                    surface
                        .points
                        .first()
                        .map_or([0.0, 0.0], |point| [point.x, point.y]),
                    surface.text_size.max(0.1),
                    surface.rotation,
                    back,
                    z,
                )
            } else if surface.filled {
                surface_mesh(&points, z, normal)
            } else {
                stroke_mesh(&points, surface.width, z, normal)
            }
            .map_err(|error| JsValue::from_str(&error))?;
            let color = if covered_copper {
                [0.12, 0.36, 0.22, 1.0]
            } else {
                match kind {
                    "Silkscreen" => [0.93, 0.92, 0.85, 1.0],
                    "Mask" | "Copper" => [0.76, 0.58, 0.22, 1.0],
                    _ => [0.10, 0.30, 0.21, 1.0],
                }
            };
            let mut geometry = geometry;
            clipper
                .clip(&mut geometry)
                .map_err(|error| JsValue::from_str(&error))?;
            translate_mesh(&mut geometry, pcb_transform);
            push_data(
                &mut output,
                "pcb",
                geometry,
                color,
                if kind == "Copper" || kind == "Mask" {
                    0.35
                } else {
                    0.8
                },
                if kind == "Copper" || kind == "Mask" {
                    0.65
                } else {
                    0.0
                },
            );
            output.objects.last_mut().unwrap().groups =
                vec!["PCB".into(), kind.into(), surface.layer.clone()];
        }
        if !hidden("PCB") {
            for (points, _) in contours.iter().filter(|(_, hole)| !hole) {
                if points.len() < 3 {
                    continue;
                }
                let mut boundary = points.clone();
                boundary.push(points[0]);
                let mut outline =
                    stroke_mesh(&boundary, 0.3, board.thickness + 0.1, [0.0, 0.0, 1.0])
                        .map_err(|error| JsValue::from_str(&error))?;
                translate_mesh(&mut outline, pcb_transform);
                push_data(
                    &mut output,
                    "pcb-selection",
                    outline,
                    [0.97, 0.69, 0.20, 1.0],
                    0.45,
                    0.15,
                );
            }
        }
        for loaded in input.models {
            if hidden("Models") || hidden(&loaded.id) {
                continue;
            }
            if let Some(model) = board.models.iter().find(|model| model.id == loaded.id) {
                if model.path.to_ascii_lowercase().contains("keycap") && hidden("Keycaps") {
                    continue;
                }
                let back = model.side == "back";
                let model_transform = pcb_model_transform(
                    [
                        model.pose.at.x,
                        model.pose.at.y,
                        if back { 0.0 } else { board.thickness },
                    ],
                    model.pose.rotation,
                    back,
                    [model.offset.x, model.offset.y, model.offset.z],
                    [model.rotation.x, model.rotation.y, model.rotation.z],
                    [model.scale.x, model.scale.y, model.scale.z],
                );
                let color = if input.selected_layer == model.reference {
                    [0.84, 0.63, 0.23, 1.0]
                } else {
                    [0.70, 0.73, 0.77, 1.0]
                };
                push_mesh(
                    &mut output,
                    &model.reference,
                    loaded.mesh,
                    pcb_transform * model_transform,
                    color,
                    0.55,
                    0.12,
                )?;
                let object = output.objects.last_mut().unwrap();
                object.groups = vec!["Models".into(), loaded.id.clone()];
                if model.path.to_ascii_lowercase().contains("keycap") {
                    object.groups.push("Keycaps".into());
                }
            }
        }
        for body in input.bodies {
            if hidden(&body.id) || (body.id.starts_with("keycap") && hidden("Keycaps")) {
                continue;
            }
            let layer_index = stack_index(&input.mechanical_stack, &body.id);
            let offset = if input.view == "exploded" {
                explode_offset(layer_index)
            } else {
                0.0
            };
            let color = if input.selected_layer == body.id {
                [0.84, 0.63, 0.23, 1.0]
            } else if body.id.to_ascii_lowercase().contains("foam")
                || body.id.to_ascii_lowercase().contains("gasket")
            {
                [0.20, 0.18, 0.24, 1.0]
            } else if body.id.to_ascii_lowercase().contains("plate") {
                [0.34, 0.57, 0.44, 1.0]
            } else if input.theme == "dark" {
                [0.62, 0.66, 0.72, 1.0]
            } else {
                [0.68, 0.71, 0.75, 1.0]
            };
            let color = body.color.as_deref().and_then(hex_color).unwrap_or(color);
            push_mesh(
                &mut output,
                &body.id,
                body.mesh,
                Mat4::from_translation(Vec3::new(0.0, 0.0, offset)),
                color,
                0.68,
                0.03,
            )?;
        }
        if let Some(battery) = input.battery
            && !hidden("battery")
        {
            let offset = if input.view == "exploded" {
                explode_offset(stack_index(&input.mechanical_stack, "battery"))
            } else {
                0.0
            };
            let center_z = input
                .mechanical_stack
                .iter()
                .find(|layer| layer.id == "battery")
                .map_or(battery.size.z / 2.0, |layer| {
                    layer.z + layer.thickness / 2.0
                })
                + offset;
            let mut box_geometry = box_data([battery.size.x, battery.size.y, battery.size.z]);
            translate_data(
                &mut box_geometry,
                Mat4::from_translation(Vec3::new(battery.at.x, battery.at.y, center_z)),
            );
            push_data(
                &mut output,
                "battery",
                box_geometry,
                [0.76, 0.43, 0.24, 1.0],
                0.7,
                0.02,
            );
            let cable = stroke_mesh(
                &[
                    [battery.at.x, battery.at.y],
                    [battery.cable_exit.x, battery.cable_exit.y],
                ],
                0.45,
                center_z,
                [0.0, 0.0, 1.0],
            )
            .map_err(|error| JsValue::from_str(&error))?;
            push_data(
                &mut output,
                "battery",
                cable,
                [0.81, 0.25, 0.22, 1.0],
                0.72,
                0.0,
            );
        }
        output.section_x = Some(center_x);
    }
    for object in &mut output.objects {
        if object.id.starts_with("keycap:") || object.id.starts_with("keycap-legend:") {
            object.groups.extend(["Keycaps".into(), "Models".into()]);
        }
        if (object.id == "pcb" || object.id == "pcb-selection")
            && !object.groups.iter().any(|g| g == "PCB")
        {
            object.groups.push("PCB".into());
        }
        let layer = if object.groups.iter().any(|g| g == "Models") || object.id.starts_with("pcb") {
            "pcb"
        } else {
            &object.id
        };
        object.explode = explode_offset(stack_index(&input.mechanical_stack, layer));
    }
    batch_surfaces(&mut output.objects);
    let mut min = Vec3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY);
    let mut max = Vec3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY);
    for object in &output.objects {
        for point in &object.positions {
            min.x = min.x.min(point.x);
            min.y = min.y.min(point.y);
            min.z = min.z.min(point.z);
            max.x = max.x.max(point.x);
            max.y = max.y.max(point.y);
            max.z = max.z.max(point.z);
        }
    }
    if min.x.is_finite() {
        let center = (min + max) / 2.0;
        let radius = (max - min).magnitude() / 2.0;
        output.bounds = Some((center, radius));
    }
    Ok(output)
}

fn push_mesh(
    output: &mut BuiltScene,
    id: &str,
    mesh: MeshInput,
    transform: Mat4,
    color: [f32; 4],
    roughness: f32,
    metallic: f32,
) -> Result<(), JsValue> {
    let mut data = MeshData::default();
    if !mesh.positions.len().is_multiple_of(3) {
        return Err(JsValue::from_str("Mesh position buffer is incomplete"));
    }
    for chunk in mesh.positions.as_chunks::<3>().0.iter() {
        let transformed = transform * Vec4::new(chunk[0], chunk[1], chunk[2], 1.0);
        data.positions
            .push([transformed.x, transformed.y, transformed.z]);
    }
    if mesh.normals.len() == mesh.positions.len() {
        let normal_matrix = transform
            .invert()
            .unwrap_or_else(Mat4::identity)
            .transpose();
        for normal in mesh.normals.as_chunks::<3>().0.iter() {
            let value = normal_matrix * Vec4::new(normal[0], normal[1], normal[2], 0.0);
            let value = value.truncate().normalize();
            data.normals.push([value.x, value.y, value.z]);
        }
    } else {
        for tri in data.positions.as_chunks::<3>().0.iter() {
            let normal = (Vec3::new(
                tri[1][0] - tri[0][0],
                tri[1][1] - tri[0][1],
                tri[1][2] - tri[0][2],
            )
            .cross(Vec3::new(
                tri[2][0] - tri[0][0],
                tri[2][1] - tri[0][1],
                tri[2][2] - tri[0][2],
            )))
            .normalize();
            data.normals
                .extend_from_slice(&[[normal.x, normal.y, normal.z]; 3]);
        }
    }
    if !mesh.positions.len().is_multiple_of(9) {
        return Err(JsValue::from_str(
            "Mesh position buffer must contain complete triangles",
        ));
    }
    data.indices = (0..data.positions.len() as u32).collect();
    if let Some(colors) = mesh.colors {
        if colors.len() != data.positions.len() * 3 {
            return Err(JsValue::from_str(
                "Mesh color buffer does not match positions",
            ));
        }
        data.colors = Some(
            colors
                .as_chunks::<3>()
                .0
                .iter()
                .map(|value| [value[0], value[1], value[2]])
                .collect(),
        );
    }
    push_data(output, id, data, color, roughness, metallic);
    Ok(())
}

fn push_data(
    output: &mut BuiltScene,
    id: &str,
    data: MeshData,
    color: [f32; 4],
    roughness: f32,
    metallic: f32,
) {
    output.objects.push(SceneObject {
        id: id.to_owned(),
        groups: vec![id.to_owned()],
        explode: 0.,
        edges: None,
        positions: data
            .positions
            .iter()
            .map(|p| Vec3::new(p[0], p[1], p[2]))
            .collect(),
        normals: data
            .normals
            .iter()
            .map(|n| Vec3::new(n[0], n[1], n[2]))
            .collect(),
        colors: data.colors.as_ref().map(|items| {
            items
                .iter()
                .map(|c| {
                    Srgba::new(
                        (c[0].clamp(0.0, 1.0) * 255.0) as u8,
                        (c[1].clamp(0.0, 1.0) * 255.0) as u8,
                        (c[2].clamp(0.0, 1.0) * 255.0) as u8,
                        255,
                    )
                })
                .collect()
        }),
        indices: data.indices,
        color: Srgba::new(
            (color[0].clamp(0.0, 1.0) * 255.0) as u8,
            (color[1].clamp(0.0, 1.0) * 255.0) as u8,
            (color[2].clamp(0.0, 1.0) * 255.0) as u8,
            (color[3].clamp(0.0, 1.0) * 255.0) as u8,
        ),
        roughness,
        metallic,
    });
}

fn translate_mesh(mesh: &mut MeshData, transform: Mat4) {
    for point in &mut mesh.positions {
        let output = transform * Vec4::new(point[0], point[1], point[2], 1.0);
        *point = [output.x, output.y, output.z];
    }
}

fn translate_data(mesh: &mut MeshData, transform: Mat4) {
    translate_mesh(mesh, transform);
}

fn box_data(size: [f32; 3]) -> MeshData {
    let h = size.map(|value| value / 2.0);
    let vertices = [
        [-h[0], -h[1], -h[2]],
        [h[0], -h[1], -h[2]],
        [h[0], h[1], -h[2]],
        [-h[0], h[1], -h[2]],
        [-h[0], -h[1], h[2]],
        [h[0], -h[1], h[2]],
        [h[0], h[1], h[2]],
        [-h[0], h[1], h[2]],
    ];
    let faces = [
        [0, 1, 2, 3],
        [4, 7, 6, 5],
        [0, 4, 5, 1],
        [1, 5, 6, 2],
        [2, 6, 7, 3],
        [4, 0, 3, 7],
    ];
    let mut data = MeshData::default();
    for face in faces {
        for triangle in [[face[0], face[1], face[2]], [face[0], face[2], face[3]]] {
            let start = data.positions.len() as u32;
            for index in triangle {
                data.positions.push(vertices[index]);
            }
            let a = Vec3::new(
                data.positions[start as usize][0],
                data.positions[start as usize][1],
                data.positions[start as usize][2],
            );
            let b = Vec3::new(
                data.positions[start as usize + 1][0],
                data.positions[start as usize + 1][1],
                data.positions[start as usize + 1][2],
            );
            let c = Vec3::new(
                data.positions[start as usize + 2][0],
                data.positions[start as usize + 2][1],
                data.positions[start as usize + 2][2],
            );
            let n = (b - a).cross(c - a).normalize();
            data.normals.extend_from_slice(&[[n.x, n.y, n.z]; 3]);
            data.indices
                .extend_from_slice(&[start, start + 1, start + 2]);
        }
    }
    data
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

fn text_mesh(
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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PreparedObject {
    id: String,
    groups: Vec<String>,
    explode: f32,
    mesh: MeshInput,
    edges: MeshInput,
    color: [f32; 4],
    roughness: f32,
    metallic: f32,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PreparedScene {
    revision: u64,
    keep_camera: bool,
    section_x: Option<f32>,
    bounds: Option<[f32; 4]>,
    objects: Vec<PreparedObject>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PreparedScenePatch {
    revision: u64,
    #[serde(default)]
    section_x: Option<f32>,
    #[serde(default)]
    bounds: Option<[f32; 4]>,
    #[serde(default)]
    objects: Vec<PreparedObject>,
    #[serde(default)]
    removed: Vec<String>,
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

#[wasm_bindgen(js_name=prepareScene)]
pub fn prepare_scene(value: JsValue) -> Result<JsValue, JsValue> {
    let mut input: SceneInput =
        serde_wasm_bindgen::from_value(value).map_err(|e| JsValue::from_str(&e.to_string()))?;
    input.view = "assembled".into();
    input.hidden.clear();
    input.selected_layer.clear();
    let scene = build_scene(input)?;
    let result = Object::new();
    let set = |name: &str, value: &JsValue| Reflect::set(&result, &JsValue::from_str(name), value);
    set("revision", &JsValue::from_f64(scene.revision as f64))?;
    set("keepCamera", &JsValue::from_bool(scene.keep_camera))?;
    set(
        "sectionX",
        &scene
            .section_x
            .map_or(JsValue::NULL, |v| JsValue::from_f64(v as f64)),
    )?;
    set(
        "bounds",
        &serde_wasm_bindgen::to_value(&scene.bounds.map(|(c, r)| [c.x, c.y, c.z, r])).unwrap(),
    )?;
    let objects = js_sys::Array::new();
    for object in scene.objects {
        let item = Object::new();
        let data = MeshData {
            positions: object.positions.iter().map(|p| [p.x, p.y, p.z]).collect(),
            normals: vec![],
            indices: object.indices.clone(),
            colors: None,
        };
        let edges = if object
            .groups
            .iter()
            .any(|g| matches!(g.as_str(), "Copper" | "Mask" | "Silkscreen"))
        {
            MeshData::default()
        } else {
            edge_mesh(&data)
        };
        let edges = ModelMesh {
            positions: edges
                .indices
                .iter()
                .flat_map(|i| edges.positions[*i as usize])
                .collect(),
            normals: edges
                .indices
                .iter()
                .flat_map(|i| edges.normals[*i as usize])
                .collect(),
            colors: None,
        };
        Reflect::set(&item, &"edges".into(), &encode_mesh(edges)?)?;
        let mesh = ModelMesh {
            positions: object
                .indices
                .iter()
                .flat_map(|i| {
                    let p = object.positions[*i as usize];
                    [p.x, p.y, p.z]
                })
                .collect(),
            normals: object
                .indices
                .iter()
                .flat_map(|i| {
                    let n = object.normals[*i as usize];
                    [n.x, n.y, n.z]
                })
                .collect(),
            colors: object.colors.as_ref().map(|colors| {
                object
                    .indices
                    .iter()
                    .flat_map(|i| {
                        let c = colors[*i as usize];
                        [c.r as f32 / 255., c.g as f32 / 255., c.b as f32 / 255.]
                    })
                    .collect()
            }),
        };
        Reflect::set(&item, &"mesh".into(), &encode_mesh(mesh)?)?;
        Reflect::set(&item, &"id".into(), &object.id.into())?;
        Reflect::set(
            &item,
            &"groups".into(),
            &serde_wasm_bindgen::to_value(&object.groups).unwrap(),
        )?;
        Reflect::set(
            &item,
            &"explode".into(),
            &JsValue::from_f64(object.explode as f64),
        )?;
        let c = object.color;
        Reflect::set(
            &item,
            &"color".into(),
            &serde_wasm_bindgen::to_value(&[
                c.r as f32 / 255.,
                c.g as f32 / 255.,
                c.b as f32 / 255.,
                c.a as f32 / 255.,
            ])
            .unwrap(),
        )?;
        Reflect::set(
            &item,
            &"roughness".into(),
            &JsValue::from_f64(object.roughness as f64),
        )?;
        Reflect::set(
            &item,
            &"metallic".into(),
            &JsValue::from_f64(object.metallic as f64),
        )?;
        objects.push(&item);
    }
    set("objects", &objects)?;
    Ok(result.into())
}
fn mesh_fingerprint(object: &SceneObject) -> u64 {
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    for p in object.positions.iter().chain(&object.normals) {
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

fn hex_color(value: &str) -> Option<[f32; 4]> {
    if value.len() != 7 || !value.starts_with('#') {
        return None;
    }
    let rgb = u32::from_str_radix(&value[1..], 16).ok()?;
    Some([
        ((rgb >> 16) & 255) as f32 / 255.0,
        ((rgb >> 8) & 255) as f32 / 255.0,
        (rgb & 255) as f32 / 255.0,
        1.0,
    ])
}
