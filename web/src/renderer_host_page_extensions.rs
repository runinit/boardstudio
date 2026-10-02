use js_sys::{Float32Array, Object};

/// Binary-only renderer handle. The reusable library host stays unchanged;
/// this wrapper submits the current Case full-scene input with checked sequence identity.
pub(crate) struct RendererPageHost {
    host: RendererHost,
    last_sequence: Cell<u64>,
}

impl RendererPageHost {
    pub(crate) async fn mount(
        canvas: HtmlCanvasElement,
        input: JsValue,
        sequence: u64,
        status: Rc<dyn Fn(String)>,
        is_current: Rc<dyn Fn() -> bool>,
    ) -> Result<(Self, bool), String> {
        let initial_sequence = sequence
            .checked_sub(1)
            .ok_or_else(|| "Renderer sequence must be positive".to_owned())?;
        let initial_input = with_scene_sequence(&input, initial_sequence)?;
        let host = RendererHost::mount(canvas, initial_input, status, is_current).await?;
        let page_host = Self {
            host,
            last_sequence: Cell::new(initial_sequence),
        };
        let accepted = page_host.submit_scene(input, sequence)?;
        Ok((page_host, accepted))
    }

    pub(crate) fn dispose(&self) -> Result<(), String> {
        self.host.dispose()
    }

    pub(crate) fn submit_scene(&self, input: JsValue, sequence: u64) -> Result<bool, String> {
        self.ensure_active()?;
        if sequence <= self.last_sequence.get() {
            return Err("Renderer scene sequence must increase".to_owned());
        }
        self.last_sequence.set(sequence);
        let input = with_scene_sequence(&input, sequence)?;
        let accepted = call_method(&self.host.inner.renderer, "setScene", &[input])
            .map_err(js_error)?
            .as_bool()
            .ok_or_else(|| "Renderer setScene returned no scene acceptance result".to_owned())?;
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
}

fn with_scene_sequence(input: &JsValue, sequence: u64) -> Result<JsValue, String> {
    if !input.is_object() || input.is_null() {
        return Err("Renderer scene update must be an object".to_owned());
    }
    let output = Object::new();
    let keys = Object::keys(input.unchecked_ref::<Object>());
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
    let board = Reflect::get(input, &"board".into()).map_err(js_error)?;
    if !board.is_null() && !board.is_undefined() {
        let board_copy = Object::new();
        let board_keys = Object::keys(board.unchecked_ref::<Object>());
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
    Ok(output.into())
}
