// Stage and counter hooks kept as no-ops at the existing call sites. The JavaScript metrics
// object they used to feed was retired with the CAD TypeScript adapters (nothing consumed it).

pub(super) struct Stage;

impl Stage {
    pub(super) fn new(_name: &'static str) -> Self {
        Self
    }
}

pub(super) fn count(_name: &'static str, _value: usize) {}
