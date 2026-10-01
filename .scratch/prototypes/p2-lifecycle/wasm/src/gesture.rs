#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum GestureEffect {
    Preview { id: String, at: Point },
    Commit { id: String, at: Point },
    Cancel { id: String },
    Release { pointer_id: i32 },
}

#[derive(Clone, Debug)]
struct ActiveGesture {
    pointer_id: i32,
    id: String,
    start: Point,
}

#[derive(Default)]
pub struct GestureCoordinator {
    active: Option<ActiveGesture>,
}

impl GestureCoordinator {
    pub fn pointer_down(&mut self, pointer_id: i32, id: &str, at: Point) -> Vec<GestureEffect> {
        if self.active.is_some() || id.is_empty() {
            return Vec::new();
        }
        self.active = Some(ActiveGesture {
            pointer_id,
            id: id.to_owned(),
            start: at,
        });
        Vec::new()
    }

    pub fn pointer_move(&mut self, pointer_id: i32, at: Point) -> Vec<GestureEffect> {
        let Some(active) = self
            .active
            .as_ref()
            .filter(|active| active.pointer_id == pointer_id)
        else {
            return Vec::new();
        };
        vec![GestureEffect::Preview {
            id: active.id.clone(),
            at: Point {
                x: at.x - active.start.x,
                y: at.y - active.start.y,
            },
        }]
    }

    pub fn pointer_up(&mut self, pointer_id: i32, at: Point) -> Vec<GestureEffect> {
        if !self.is_active_for(pointer_id) {
            return Vec::new();
        }
        let Some(active) = self.active.take() else {
            return Vec::new();
        };
        vec![
            GestureEffect::Commit {
                id: active.id,
                at: Point {
                    x: at.x - active.start.x,
                    y: at.y - active.start.y,
                },
            },
            GestureEffect::Release { pointer_id },
        ]
    }

    pub fn pointer_cancel(&mut self, pointer_id: i32) -> Vec<GestureEffect> {
        self.cancel_active(Some(pointer_id))
    }

    pub fn escape(&mut self) -> Vec<GestureEffect> {
        self.cancel_active(None)
    }

    pub fn is_active_for(&self, pointer_id: i32) -> bool {
        self.active
            .as_ref()
            .is_some_and(|active| active.pointer_id == pointer_id)
    }

    fn cancel_active(&mut self, pointer_id: Option<i32>) -> Vec<GestureEffect> {
        let Some(active) = self.active.as_ref() else {
            return Vec::new();
        };
        if pointer_id.is_some_and(|pointer_id| pointer_id != active.pointer_id) {
            return Vec::new();
        }
        let active = self.active.take().expect("active gesture was checked");
        vec![
            GestureEffect::Cancel { id: active.id },
            GestureEffect::Release {
                pointer_id: active.pointer_id,
            },
        ]
    }
}
