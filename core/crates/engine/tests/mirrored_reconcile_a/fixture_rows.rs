//! By-ID access to materialized fixture rows. Missing rows fail at the assertion site.
use boardstudio_model::model::*;

pub trait FixtureId {
    fn fixture_id(&self) -> &str;
}

macro_rules! fixture_ids {
    ($($record:ty),+ $(,)?) => {$(
        impl FixtureId for $record {
            fn fixture_id(&self) -> &str { &self.id }
        }
    )+};
}
fixture_ids!(
    Part,
    Matrix,
    Board,
    Net,
    MountedModule,
    EmbeddedCircuit,
    PartDefinition,
    CaseBody,
    MatrixAssembly,
    MechanicalMirrorPair,
    Layout
);

pub trait FixtureRows<T> {
    fn by_id(&self, id: &str) -> &T;
    fn by_id_mut(&mut self, id: &str) -> &mut T;
}
impl<T: FixtureId> FixtureRows<T> for [T] {
    #[track_caller]
    fn by_id(&self, id: &str) -> &T {
        self.iter()
            .find(|row| row.fixture_id() == id)
            .unwrap_or_else(|| panic!("missing fixture row {id}"))
    }

    #[track_caller]
    fn by_id_mut(&mut self, id: &str) -> &mut T {
        self.iter_mut()
            .find(|row| row.fixture_id() == id)
            .unwrap_or_else(|| panic!("missing fixture row {id}"))
    }
}

pub trait FixturePair {
    fn first_halves(&self) -> &MirroredHalves;
    fn first_halves_mut(&mut self) -> &mut MirroredHalves;
}
impl FixturePair for ProjectDoc {
    fn first_halves(&self) -> &MirroredHalves {
        self.hardware.as_ref().unwrap().pairs[0]
            .halves
            .as_ref()
            .unwrap()
    }
    fn first_halves_mut(&mut self) -> &mut MirroredHalves {
        self.hardware.as_mut().unwrap().pairs[0]
            .halves
            .as_mut()
            .unwrap()
    }
}
