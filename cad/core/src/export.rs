use crate::ExactModel;
use std::fmt::{self, Display, Write};

mod generated;
mod geometry;

/// One prepared body's identity and exact geometry, already in world coordinates.
pub struct StepBody<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub model: &'a ExactModel,
}

/// Writes every exact solid, in model order, as one millimetre STEP product.
pub fn export_step(model: &ExactModel) -> Result<Vec<u8>, String> {
    export_step_bodies(&[StepBody {
        id: "body",
        name: "Body",
        model,
    }])
}

/// Writes one STEP product per body, retaining every exact solid in body order.
/// Placement is already baked into geometry; no additional transform is applied.
pub fn export_step_bodies(bodies: &[StepBody<'_>]) -> Result<Vec<u8>, String> {
    if bodies.is_empty() || bodies.iter().any(|body| body.model.is_empty()) {
        return Err("STEP export requires at least one exact solid per body".into());
    }

    let mut step = Writer {
        text: PREAMBLE.into(),
        next: 14,
    };
    for body in bodies {
        step.body(body)
            .map_err(|error| format!("STEP body '{}': {error}", body.id))?;
    }
    step.text.push_str("ENDSEC;\nEND-ISO-10303-21;\n");
    Ok(step.text.into_bytes())
}

// A fixed timestamp keeps identical geometry and product identities byte-for-byte reproducible.
const PREAMBLE: &str = "ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('BoardStudio exact geometry'), '2;1');
FILE_NAME('', '1970-01-01T00:00:00', (''), (''), 'BoardStudio', 'BoardStudio', '');
FILE_SCHEMA(('AUTOMOTIVE_DESIGN'));
ENDSEC;
DATA;
#1 = APPLICATION_CONTEXT('core data for automotive mechanical design processes');
#2 = (GEOMETRIC_REPRESENTATION_CONTEXT(3) GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT((#6)) GLOBAL_UNIT_ASSIGNED_CONTEXT((#3, #4, #5)) REPRESENTATION_CONTEXT('', '3D'));
#3 = (LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.MILLI.,.METRE.));
#4 = (NAMED_UNIT(*) PLANE_ANGLE_UNIT() SI_UNIT($,.RADIAN.));
#5 = (NAMED_UNIT(*) SI_UNIT($,.STERADIAN.) SOLID_ANGLE_UNIT());
#6 = UNCERTAINTY_MEASURE_WITH_UNIT(LENGTH_MEASURE(1.E-6), #3, 'distance_accuracy_value', '');
#7 = APPLICATION_PROTOCOL_DEFINITION('international standard', 'automotive_design', 2000, #1);
#8 = PRODUCT_CONTEXT('', #1, 'mechanical');
#9 = AXIS2_PLACEMENT_3D('', #10, #11, #12);
#10 = CARTESIAN_POINT('', (0.0, 0.0, 0.0));
#11 = DIRECTION('', (0.0, 0.0, 1.0));
#12 = DIRECTION('', (1.0, 0.0, 0.0));
#13 = PRODUCT_DEFINITION_CONTEXT('part definition', #1, 'design');
";

type StepResult<T> = Result<T, &'static str>;

struct Writer {
    text: String,
    next: usize,
}

impl Writer {
    fn entity(&mut self, record: fmt::Arguments<'_>) -> Ref {
        let id = self.next;
        self.next += 1;
        writeln!(self.text, "#{id} = {record};").unwrap();
        Ref(id)
    }

    fn body(&mut self, body: &StepBody<'_>) -> StepResult<()> {
        let id = step_string(body.id);
        let name = step_string(body.name);
        let product = self.entity(format_args!("PRODUCT('{id}', '{name}', '', (#8))"));
        let formation = self.entity(format_args!(
            "PRODUCT_DEFINITION_FORMATION('', '', {product})"
        ));
        let definition = self.entity(format_args!(
            "PRODUCT_DEFINITION('design', '', {formation}, #13)"
        ));
        let shape = self.entity(format_args!(
            "PRODUCT_DEFINITION_SHAPE('', '', {definition})"
        ));
        let mut items = vec![Ref(9)];
        for (topology, shell) in body.model.shells() {
            items.push(self.generated_solid(topology, shell));
        }
        let representation = self.entity(format_args!(
            "SHAPE_REPRESENTATION('', {}, #2)",
            List(&items)
        ));
        self.entity(format_args!(
            "SHAPE_DEFINITION_REPRESENTATION({shape}, {representation})"
        ));
        Ok(())
    }
}

fn logical(value: bool) -> &'static str {
    if value { ".T." } else { ".F." }
}

#[derive(Clone, Copy)]
struct Ref(usize);

impl Display for Ref {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{}", self.0)
    }
}

struct List<'a, T>(&'a [T]);

impl<T: Display> Display for List<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("(")?;
        for (index, item) in self.0.iter().enumerate() {
            if index > 0 {
                f.write_str(", ")?;
            }
            item.fmt(f)?;
        }
        f.write_str(")")
    }
}

// Part 21 string escaping preserves caller identity in the product records.
fn step_string(value: &str) -> String {
    let mut encoded = String::new();
    for c in value.chars() {
        match c {
            '\'' => encoded.push_str("''"),
            '\\' => encoded.push_str("\\\\"),
            ' '..='~' => encoded.push(c),
            _ if c.is_ascii() => write!(&mut encoded, "\\X\\{:02X}", c as u32).unwrap(),
            _ => write!(&mut encoded, "\\X4\\{:08X}\\X0\\", c as u32).unwrap(),
        }
    }
    encoded
}
