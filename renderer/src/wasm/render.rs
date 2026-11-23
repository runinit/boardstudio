use super::*;

impl Renderer {
    pub(super) fn accept_scene(&mut self, items: BuiltScene) -> Result<bool, JsValue> {
        self.focused_ids.clear();
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
    pub(super) fn section_equation(&self) -> (Vec4, Vec3, f32) {
        let (mut low, mut high) = bounds_of(self.objects.iter().flat_map(|object| {
            object
                .triangles
                .iter()
                .flatten()
                .chain(&object.line_points)
                .map(move |point| (object.pose * point.extend(1.)).truncate())
        }));
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
    pub(super) fn apply_state(&mut self) {
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
                .and_then(|value| hex_rgb(value))
                .map(|rgb| Srgba::new(rgb[0], rgb[1], rgb[2], item.base_color.a))
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
                exploded_view_offset(&self.state.view, item.explode, self.state.explode_amount),
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
            let line_emissive = if selected {
                Srgba::new(255, 186, 58, 255)
            } else if self.state.mode == "wireframe" && self.state.theme == "dark" {
                Srgba::new(180, 205, 215, 255)
            } else {
                Srgba::new(30, 42, 48, 255)
            };
            item.edges.material.physical.albedo = Srgba::BLACK;
            item.edges.material.physical.emissive = line_emissive;
            if let Some(lines) = &mut item.lines {
                lines.geometry.transformation = transform;
                lines.material.section = clip;
                lines.material.physical.render_states.depth_test =
                    item.edges.material.physical.render_states.depth_test;
                lines.material.physical.render_states.write_mask = three_d::WriteMask::COLOR;
                lines.material.physical.albedo = Srgba::BLACK;
                lines.material.physical.emissive = line_emissive;
            }
        }
    }
    pub(super) fn ray(&self, x: f32, y: f32) -> (Vec3, Vec3) {
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
    pub(super) fn update_camera(&mut self) {
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
            three_d::radians(FOV_Y_DEGREES.to_radians()),
            near,
            far,
        );
    }

    /// Dark line material shared by edge and wire overlays.
    fn line_material(context: &three_d::core::Context) -> PhysicalMaterial {
        let mut material = PhysicalMaterial::new_opaque(
            context,
            &CpuMaterial {
                albedo: Srgba::new(30, 42, 48, 255),
                roughness: 1.0,
                ..CpuMaterial::default()
            },
        );
        material.render_states.cull = Cull::None;
        material
    }

    pub(super) fn upload(&self, item: SceneObject) -> RenderObject {
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
        let edge_material = Self::line_material(&self.context);
        let edge_mesh = Mesh::new(&self.context, &edge_cpu);
        let line_shader = edge_mesh.vertex_shader_source();
        let edges = Gm::new(
            edge_mesh,
            SectionMaterial {
                physical: edge_material,
                section: None,
                depth_only: false,
            },
        );
        let line_points = item.lines;
        let lines = if line_points.is_empty() {
            None
        } else {
            let normals = vec![Vec3::new(0.0, 0.0, 1.0); line_points.len()];
            let line_material = Self::line_material(&self.context);
            Some(Gm::new(
                WorldLines {
                    context: self.context.clone(),
                    gl: self.gl.clone(),
                    positions: VertexBuffer::new_with_data(&self.context, &line_points),
                    normals: VertexBuffer::new_with_data(&self.context, &normals),
                    vertex_count: line_points.len() as u32,
                    aabb: AxisAlignedBoundingBox::new_with_positions(&line_points),
                    transformation: Mat4::identity(),
                    vertex_shader: line_shader,
                },
                SectionMaterial {
                    physical: line_material,
                    section: None,
                    depth_only: false,
                },
            ))
        };
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
            line_points,
            lines,
            edges,
            object: Gm::new(Mesh::new(&self.context, &cpu_mesh), material),
            pose: Mat4::identity(),
            handle_length: None,
            base_color: cpu_material.albedo,
        }
    }
}
