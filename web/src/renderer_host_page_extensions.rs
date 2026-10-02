use js_sys::{Float32Array, Object, Uint8Array};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RendererModelFormat {
    Stl,
    Wrl,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RendererModelSource {
    pub(crate) id: String,
    pub(crate) format: RendererModelFormat,
    pub(crate) bytes: Vec<u8>,
}

#[derive(Clone)]
pub(crate) enum RendererSceneUpdate {
    Full(JsValue),
    Prepare(JsValue),
    Prepared(JsValue),
    PreparedPatch(JsValue),
}

/// Binary-only renderer handle. The reusable library host stays unchanged;
/// this wrapper owns the cached ES module namespace needed by page-only calls.
pub(crate) struct RendererPageHost {
    host: RendererHost,
    module: JsValue,
    last_sequence: Cell<u64>,
}

impl RendererPageHost {
    pub(crate) async fn mount(
        canvas: HtmlCanvasElement,
        initial_input: JsValue,
        update: RendererSceneUpdate,
        models: &[RendererModelSource],
        sequence: u64,
        status: Rc<dyn Fn(String)>,
        is_current: Rc<dyn Fn() -> bool>,
    ) -> Result<(Self, bool), String> {
        let initial_sequence = sequence
            .checked_sub(1)
            .ok_or_else(|| "Renderer sequence must be positive".to_owned())?;
        let initial_input = with_scene_sequence(&initial_input, initial_sequence, true)?;
        let host = RendererHost::mount(canvas, initial_input, status, is_current.clone()).await?;
        let module = import_renderer_module().await;
        let module = match module {
            Ok(module) if is_current() => module,
            Ok(_) => {
                let _ = host.dispose();
                return Err("Renderer mount scope was cancelled".to_owned());
            }
            Err(error) => {
                return match host.dispose() {
                    Ok(()) => Err(error),
                    Err(cleanup) => {
                        Err(format!("{error}; renderer cleanup also failed: {cleanup}"))
                    }
                };
            }
        };
        let page_host = Self {
            host,
            module,
            last_sequence: Cell::new(initial_sequence),
        };
        let accepted = page_host.submit_scene(update, models, sequence)?;
        Ok((page_host, accepted))
    }

    pub(crate) fn dispose(&self) -> Result<(), String> {
        self.host.dispose()
    }

    pub(crate) fn submit_scene(
        &self,
        update: RendererSceneUpdate,
        models: &[RendererModelSource],
        sequence: u64,
    ) -> Result<bool, String> {
        self.ensure_active()?;
        if sequence <= self.last_sequence.get() {
            return Err("Renderer scene sequence must increase".to_owned());
        }
        self.last_sequence.set(sequence);
        let (method, input) = match update {
            RendererSceneUpdate::Full(input) => {
                let input = self.input_with_models(input, models)?;
                ("setScene", with_scene_sequence(&input, sequence, true)?)
            }
            RendererSceneUpdate::Prepare(input) => {
                let input = self.input_with_models(input, models)?;
                let input = with_scene_sequence(&input, sequence, true)?;
                let prepared = self.prepare_scene(input)?;
                (
                    "setPreparedScene",
                    with_scene_sequence(&prepared, sequence, false)?,
                )
            }
            RendererSceneUpdate::Prepared(input) => (
                "setPreparedScene",
                with_scene_sequence(&input, sequence, false)?,
            ),
            RendererSceneUpdate::PreparedPatch(input) => (
                "setPreparedScenePatch",
                with_scene_sequence(&input, sequence, false)?,
            ),
        };
        let accepted = call_method(&self.host.inner.renderer, method, &[input])
            .map_err(js_error)?
            .as_bool()
            .ok_or_else(|| format!("Renderer {method} returned no scene acceptance result"))?;
        if accepted {
            schedule_frame(&self.host.inner).map_err(js_error)?;
        }
        Ok(accepted)
    }

    pub(crate) fn set_display_state(&self, state: JsValue) -> Result<(), String> {
        self.ensure_active()?;
        call_method(&self.host.inner.renderer, "setState", &[state]).map_err(js_error)?;
        schedule_frame(&self.host.inner).map_err(js_error)
    }

    pub(crate) fn set_handles(&self, handles: JsValue) -> Result<u32, String> {
        self.ensure_active()?;
        let count = call_method(&self.host.inner.renderer, "setHandles", &[handles])
            .map_err(js_error)?
            .as_f64()
            .ok_or_else(|| "Renderer returned no handle count".to_owned())?;
        if !count.is_finite() || !(0.0..=f64::from(u32::MAX)).contains(&count) {
            return Err("Renderer returned an invalid handle count".to_owned());
        }
        schedule_frame(&self.host.inner).map_err(js_error)?;
        Ok(count as u32)
    }

    pub(crate) fn pick_at_client(
        &self,
        client_x: f64,
        client_y: f64,
    ) -> Result<Option<String>, String> {
        self.ensure_active()?;
        let (x, y) = self.renderer_coordinates(client_x, client_y)?;
        let result = call_method(&self.host.inner.renderer, "pick", &[x.into(), y.into()])
            .map_err(js_error)?;
        if result.is_null() || result.is_undefined() {
            return Ok(None);
        }
        result
            .as_string()
            .map(Some)
            .ok_or_else(|| "Renderer returned a non-string pick result".to_owned())
    }

    pub(crate) fn point_on_plane_at_client(
        &self,
        client_x: f64,
        client_y: f64,
        z: f32,
    ) -> Result<Option<[f32; 3]>, String> {
        self.ensure_active()?;
        let (x, y) = self.renderer_coordinates(client_x, client_y)?;
        let value = call_method(
            &self.host.inner.renderer,
            "pointOnPlane",
            &[x.into(), y.into(), z.into()],
        )
        .map_err(js_error)?;
        let points = value
            .dyn_into::<Float32Array>()
            .map_err(|_| "Renderer returned an invalid world point buffer".to_owned())?;
        match points.length() {
            0 => Ok(None),
            3 => {
                let point = [
                    points.get_index(0),
                    points.get_index(1),
                    points.get_index(2),
                ];
                point
                    .iter()
                    .all(|coordinate| coordinate.is_finite())
                    .then_some(Some(point))
                    .ok_or_else(|| "Renderer returned non-finite world coordinates".to_owned())
            }
            _ => Err("Renderer returned an invalid world point length".to_owned()),
        }
    }

    pub(crate) fn orbit(&self, delta_x: f64, delta_y: f64) -> Result<(), String> {
        self.host.orbit(delta_x, delta_y)
    }

    pub(crate) fn zoom(&self, factor: f64) -> Result<(), String> {
        self.host.zoom(factor)
    }

    pub(crate) fn view(&self, preset: &str) -> Result<(), String> {
        self.host.view(preset)
    }

    pub(crate) fn fit(&self) -> Result<(), String> {
        self.host.fit()
    }

    fn input_with_models(
        &self,
        input: JsValue,
        models: &[RendererModelSource],
    ) -> Result<JsValue, String> {
        if models.is_empty() {
            return Ok(input);
        }
        let loaded_models = js_sys::Array::new();
        for model in models {
            let mesh = match model.format {
                RendererModelFormat::Stl => self.decode_stl(&model.bytes)?,
                RendererModelFormat::Wrl => self.decode_wrl(&model.bytes)?,
            };
            let loaded = Object::new();
            Reflect::set(&loaded, &"id".into(), &model.id.clone().into()).map_err(js_error)?;
            Reflect::set(&loaded, &"mesh".into(), &mesh).map_err(js_error)?;
            loaded_models.push(&loaded);
        }
        Reflect::set(&input, &"models".into(), &loaded_models).map_err(js_error)?;
        Ok(input)
    }

    fn ensure_active(&self) -> Result<(), String> {
        if self.host.inner.disposed.get() || self.host.inner.context_lost.get() {
            Err("Renderer is no longer active".to_owned())
        } else {
            Ok(())
        }
    }

    fn renderer_coordinates(&self, client_x: f64, client_y: f64) -> Result<(f64, f64), String> {
        self.ensure_active()?;
        if !client_x.is_finite() || !client_y.is_finite() {
            return Err("Pointer coordinates are not finite".to_owned());
        }
        let rect = self.host.inner.canvas.get_bounding_client_rect();
        if rect.width() <= 0.0 || rect.height() <= 0.0 {
            return Err("Renderer canvas has no visible area".to_owned());
        }
        let x = (client_x - rect.left()) * f64::from(self.host.inner.canvas.width()) / rect.width();
        let y =
            (client_y - rect.top()) * f64::from(self.host.inner.canvas.height()) / rect.height();
        if !x.is_finite() || !y.is_finite() {
            return Err("Renderer pointer coordinates are invalid".to_owned());
        }
        Ok((x, y))
    }

    pub(crate) fn decode_stl(&self, bytes: &[u8]) -> Result<JsValue, String> {
        self.call_module_function("decodeStl", &[Uint8Array::from(bytes).into()])
    }

    pub(crate) fn decode_wrl(&self, bytes: &[u8]) -> Result<JsValue, String> {
        self.call_module_function("decodeWrl", &[Uint8Array::from(bytes).into()])
    }

    fn prepare_scene(&self, input: JsValue) -> Result<JsValue, String> {
        self.call_module_function("prepareScene", &[input])
    }

    fn call_module_function(&self, name: &str, arguments: &[JsValue]) -> Result<JsValue, String> {
        self.ensure_active()?;
        let function = Reflect::get(&self.module, &JsValue::from_str(name))
            .map_err(js_error)?
            .dyn_into::<Function>()
            .map_err(js_error)?;
        let values = Array::new();
        for argument in arguments {
            values.push(argument);
        }
        function.apply(&self.module, &values).map_err(js_error)
    }
}

fn with_scene_sequence(
    input: &JsValue,
    sequence: u64,
    include_board: bool,
) -> Result<JsValue, String> {
    if !input.is_object() || input.is_null() {
        return Err("Renderer scene update must be an object".to_owned());
    }
    let output = Object::new();
    let keys = Object::keys(input);
    for index in 0..keys.length() {
        let key = keys.get(index);
        let value = Reflect::get(input, &key).map_err(js_error)?;
        Reflect::set(&output, &key, &value).map_err(js_error)?;
    }
    Reflect::set(
        &output,
        &"revision".into(),
        &JsValue::from_f64(sequence as f64),
    )
    .map_err(js_error)?;
    if include_board {
        let board = Reflect::get(input, &"board".into()).map_err(js_error)?;
        if !board.is_null() && !board.is_undefined() {
            let board_copy = Object::new();
            let board_keys = Object::keys(&board);
            for index in 0..board_keys.length() {
                let key = board_keys.get(index);
                let value = Reflect::get(&board, &key).map_err(js_error)?;
                Reflect::set(&board_copy, &key, &value).map_err(js_error)?;
            }
            Reflect::set(
                &board_copy,
                &"revision".into(),
                &JsValue::from_f64(sequence as f64),
            )
            .map_err(js_error)?;
            Reflect::set(&output, &"board".into(), &board_copy).map_err(js_error)?;
        }
    }
    Ok(output.into())
}

async fn import_renderer_module() -> Result<JsValue, String> {
    let module_url = resource_url("assets/renderer/boardstudio_renderer_wasm.js")?;
    let import = Function::new_with_args("url", "return import(url)");
    let module = import
        .call1(&JsValue::NULL, &JsValue::from_str(&module_url))
        .map_err(js_error)?
        .dyn_into::<Promise>()
        .map_err(js_error)?;
    JsFuture::from(module).await.map_err(js_error)
}
