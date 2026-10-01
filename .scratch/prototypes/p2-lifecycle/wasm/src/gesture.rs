#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum GestureEffect {
    Preview {
        id: String,
        at: Point,
        base_revision: u64,
        transaction_id: String,
        origin: Point,
    },
    Commit {
        id: String,
        at: Point,
        base_revision: u64,
        transaction_id: String,
        origin: Point,
    },
    Cancel {
        id: String,
        base_revision: u64,
        transaction_id: String,
        origin: Point,
    },
    Release {
        pointer_id: i32,
    },
}

#[derive(Clone, Debug)]
struct ActiveGesture {
    pointer_id: i32,
    id: String,
    start: Point,
    base_revision: u64,
    transaction_id: String,
    origin: Point,
}

#[derive(Default)]
pub struct GestureCoordinator {
    active: Option<ActiveGesture>,
}

impl GestureCoordinator {
    pub fn pointer_down(&mut self, pointer_id: i32, id: &str, at: Point) -> Vec<GestureEffect> {
        self.pointer_down_with_base(
            pointer_id,
            id,
            at,
            0,
            String::new(),
            Point { x: 0.0, y: 0.0 },
        )
    }

    pub fn pointer_down_with_base(
        &mut self,
        pointer_id: i32,
        id: &str,
        at: Point,
        base_revision: u64,
        transaction_id: String,
        origin: Point,
    ) -> Vec<GestureEffect> {
        if self.active.is_some() || id.is_empty() {
            return Vec::new();
        }
        self.active = Some(ActiveGesture {
            pointer_id,
            id: id.to_owned(),
            start: at,
            base_revision,
            transaction_id,
            origin,
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
            base_revision: active.base_revision,
            transaction_id: active.transaction_id.clone(),
            origin: active.origin,
        }]
    }

    pub fn pointer_up(&mut self, pointer_id: i32, at: Point) -> Vec<GestureEffect> {
        if !self.is_active_for(pointer_id) {
            return Vec::new();
        }
        let Some(active) = self.active.take() else {
            return Vec::new();
        };
        let delta = Point {
            x: at.x - active.start.x,
            y: at.y - active.start.y,
        };
        let mut effects = Vec::with_capacity(2);
        if delta.x != 0.0 || delta.y != 0.0 {
            effects.push(GestureEffect::Commit {
                id: active.id,
                at: delta,
                base_revision: active.base_revision,
                transaction_id: active.transaction_id,
                origin: active.origin,
            });
        }
        effects.push(GestureEffect::Release { pointer_id });
        effects
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
            GestureEffect::Cancel {
                id: active.id,
                base_revision: active.base_revision,
                transaction_id: active.transaction_id,
                origin: active.origin,
            },
            GestureEffect::Release {
                pointer_id: active.pointer_id,
            },
        ]
    }
}
