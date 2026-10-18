//! Private capture of the pinned native geometry's serde stream. This is neither
//! JSON nor a public geometry representation. Maps, bytes and nonfinite values
//! refuse; retained temporary tokens are dropped before sealing evidence.
use serde::ser::{self, Serialize, SerializeSeq, SerializeStruct, Serializer};
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
enum Token {
    Struct(&'static str, usize),
    Field(&'static str),
    Seq(Option<usize>),
    Variant(&'static str, &'static str),
    Float(u64),
    Unsigned(u64),
    Signed(i64),
    Bool(bool),
    End,
}
#[derive(Debug)]
struct CaptureError;
impl fmt::Display for CaptureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("unsupported retained geometry serialization")
    }
}
impl std::error::Error for CaptureError {}
impl ser::Error for CaptureError {
    fn custom<T: fmt::Display>(_: T) -> Self {
        Self
    }
}
#[derive(Default)]
struct Capture(Vec<Token>);
macro_rules! integer {
    ($name:ident, $ty:ty, $token:ident) => {
        fn $name(self, value: $ty) -> Result<(), CaptureError> {
            self.0.push(Token::$token(value.into()));
            Ok(())
        }
    };
}
impl Serializer for &mut Capture {
    type Ok = ();
    type Error = CaptureError;
    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = ser::Impossible<(), CaptureError>;
    type SerializeMap = ser::Impossible<(), CaptureError>;
    type SerializeStruct = Self;
    type SerializeStructVariant = ser::Impossible<(), CaptureError>;
    fn serialize_bool(self, value: bool) -> Result<(), CaptureError> {
        self.0.push(Token::Bool(value));
        Ok(())
    }
    integer!(serialize_i8, i8, Signed);
    integer!(serialize_i16, i16, Signed);
    integer!(serialize_i32, i32, Signed);
    integer!(serialize_i64, i64, Signed);
    integer!(serialize_u8, u8, Unsigned);
    integer!(serialize_u16, u16, Unsigned);
    integer!(serialize_u32, u32, Unsigned);
    integer!(serialize_u64, u64, Unsigned);
    fn serialize_f32(self, _: f32) -> Result<(), CaptureError> {
        Err(CaptureError)
    }
    fn serialize_f64(self, value: f64) -> Result<(), CaptureError> {
        if !value.is_finite() {
            return Err(CaptureError);
        }
        self.0.push(Token::Float(value.to_bits()));
        Ok(())
    }
    fn serialize_char(self, _: char) -> Result<(), CaptureError> {
        Err(CaptureError)
    }
    fn serialize_str(self, _: &str) -> Result<(), CaptureError> {
        Err(CaptureError)
    }
    fn serialize_bytes(self, _: &[u8]) -> Result<(), CaptureError> {
        Err(CaptureError)
    }
    fn serialize_none(self) -> Result<(), CaptureError> {
        Err(CaptureError)
    }
    fn serialize_some<T: ?Sized + Serialize>(self, _: &T) -> Result<(), CaptureError> {
        Err(CaptureError)
    }
    fn serialize_unit(self) -> Result<(), CaptureError> {
        Err(CaptureError)
    }
    fn serialize_unit_struct(self, _: &'static str) -> Result<(), CaptureError> {
        Err(CaptureError)
    }
    fn serialize_unit_variant(
        self,
        name: &'static str,
        _: u32,
        variant: &'static str,
    ) -> Result<(), CaptureError> {
        self.0.push(Token::Variant(name, variant));
        self.0.push(Token::End);
        Ok(())
    }
    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        name: &'static str,
        value: &T,
    ) -> Result<(), CaptureError> {
        self.0.push(Token::Struct(name, 1));
        value.serialize(&mut *self)?;
        self.0.push(Token::End);
        Ok(())
    }
    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        name: &'static str,
        _: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<(), CaptureError> {
        self.0.push(Token::Variant(name, variant));
        value.serialize(&mut *self)?;
        self.0.push(Token::End);
        Ok(())
    }
    fn serialize_seq(self, len: Option<usize>) -> Result<Self, CaptureError> {
        self.0.push(Token::Seq(len));
        Ok(self)
    }
    fn serialize_tuple(self, len: usize) -> Result<Self, CaptureError> {
        self.serialize_seq(Some(len))
    }
    fn serialize_tuple_struct(self, name: &'static str, len: usize) -> Result<Self, CaptureError> {
        self.serialize_struct(name, len)
    }
    fn serialize_tuple_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeTupleVariant, CaptureError> {
        Err(CaptureError)
    }
    fn serialize_map(self, _: Option<usize>) -> Result<Self::SerializeMap, CaptureError> {
        Err(CaptureError)
    }
    fn serialize_struct(self, name: &'static str, len: usize) -> Result<Self, CaptureError> {
        self.0.push(Token::Struct(name, len));
        Ok(self)
    }
    fn serialize_struct_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeStructVariant, CaptureError> {
        Err(CaptureError)
    }
}
impl SerializeSeq for &mut Capture {
    type Ok = ();
    type Error = CaptureError;
    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), CaptureError> {
        value.serialize(&mut **self)
    }
    fn end(self) -> Result<(), CaptureError> {
        self.0.push(Token::End);
        Ok(())
    }
}
impl ser::SerializeTuple for &mut Capture {
    type Ok = ();
    type Error = CaptureError;
    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), CaptureError> {
        SerializeSeq::serialize_element(self, value)
    }
    fn end(self) -> Result<(), CaptureError> {
        SerializeSeq::end(self)
    }
}
impl ser::SerializeTupleStruct for &mut Capture {
    type Ok = ();
    type Error = CaptureError;
    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), CaptureError> {
        SerializeSeq::serialize_element(self, value)
    }
    fn end(self) -> Result<(), CaptureError> {
        SerializeSeq::end(self)
    }
}
impl SerializeStruct for &mut Capture {
    type Ok = ();
    type Error = CaptureError;
    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), CaptureError> {
        self.0.push(Token::Field(key));
        value.serialize(&mut **self)
    }
    fn end(self) -> Result<(), CaptureError> {
        self.0.push(Token::End);
        Ok(())
    }
}
fn capture(value: &impl Serialize) -> Result<Vec<Token>, &'static str> {
    let mut capture = Capture::default();
    value
        .serialize(&mut capture)
        .map_err(|_| "unsupported retained geometry")?;
    Ok(capture.0)
}
pub(super) fn same_surface(a: &super::Surface, b: &super::Surface) -> Result<bool, &'static str> {
    let (a, b) = (capture(a)?, capture(b)?);
    // JSON's old finite-number equality equated signed zeros. Preserve that
    // predicate here; extraction and final-array binding retain every f64 bit.
    Ok(a.len() == b.len()
        && a.iter().zip(&b).all(|(a, b)| match (a, b) {
            (Token::Float(a), Token::Float(b)) => f64::from_bits(*a) == f64::from_bits(*b),
            _ => a == b,
        }))
}
pub(super) fn plane_coefficients(
    plane: &monstertruck_modeling::Plane,
) -> Result<[[f64; 4]; 7], &'static str> {
    let tokens = capture(plane)?;
    let mut cursor = tokens.as_slice();
    fn take(cursor: &mut &[Token], expected: Token) -> Result<(), &'static str> {
        if cursor.first() != Some(&expected) {
            return Err("retained plane schema");
        }
        *cursor = &cursor[1..];
        Ok(())
    }
    fn vector(
        cursor: &mut &[Token],
        name: &'static str,
        count: usize,
    ) -> Result<[f64; 4], &'static str> {
        take(cursor, Token::Struct(name, count))?;
        let mut result = [0.; 4];
        for (i, field) in ["x", "y", "z", "w"].into_iter().take(count).enumerate() {
            take(cursor, Token::Field(field))?;
            let Some(Token::Float(bits)) = cursor.first() else {
                return Err("retained plane coefficient");
            };
            result[i] = f64::from_bits(*bits);
            *cursor = &cursor[1..];
        }
        take(cursor, Token::End)?;
        Ok(result)
    }
    take(&mut cursor, Token::Struct("Plane", 4))?;
    let mut result = [[0.; 4]; 7];
    for (i, field) in ["o", "p", "q"].into_iter().enumerate() {
        take(&mut cursor, Token::Field(field))?;
        result[i] = vector(&mut cursor, "Point3", 3)?;
    }
    take(&mut cursor, Token::Field("transform"))?;
    take(&mut cursor, Token::Struct("Matrix4", 4))?;
    for (i, field) in ["x", "y", "z", "w"].into_iter().enumerate() {
        take(&mut cursor, Token::Field(field))?;
        result[i + 3] = vector(&mut cursor, "Vector4", 4)?;
    }
    take(&mut cursor, Token::End)?;
    take(&mut cursor, Token::End)?;
    if !cursor.is_empty() {
        return Err("retained plane trailing data");
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use monstertruck_modeling::{Matrix4, Plane, Point3, Surface, Transformed, Vector3};

    #[test]
    fn capture_matches_old_json_coefficients_and_surface_binding() {
        let base = Plane::new(
            Point3::new(-0., 0., 1.6),
            Point3::new(1., 0., 1.6),
            Point3::new(0., 1., 1.6),
        );
        for plane in [
            base,
            base.transformed(Matrix4::from_translation(Vector3::new(0.1, -0.2, 1e-20))),
        ] {
            let actual = plane_coefficients(&plane).unwrap();
            let old = serde_json::to_value(plane).unwrap();
            for (i, field) in ["o", "p", "q"].into_iter().enumerate() {
                for (j, coord) in ["x", "y", "z"].into_iter().enumerate() {
                    assert_eq!(
                        actual[i][j].to_bits(),
                        old[field][coord].as_f64().unwrap().to_bits()
                    );
                }
            }
            for (i, column) in ["x", "y", "z", "w"].into_iter().enumerate() {
                for (j, coord) in ["x", "y", "z", "w"].into_iter().enumerate() {
                    assert_eq!(
                        actual[i + 3][j].to_bits(),
                        old["transform"][column][coord].as_f64().unwrap().to_bits()
                    );
                }
            }
            assert!(same_surface(&Surface::Plane(plane), &Surface::Plane(plane)).unwrap());
            assert!(
                !same_surface(
                    &Surface::Plane(plane),
                    &Surface::Plane(
                        plane.transformed(Matrix4::from_translation(Vector3::new(0., 0., 1e-10)))
                    )
                )
                .unwrap()
            );
        }
        let bad = Plane::new(
            Point3::new(f64::NAN, 0., 0.),
            Point3::new(1., 0., 0.),
            Point3::new(0., 1., 0.),
        );
        assert!(plane_coefficients(&bad).is_err());
        assert!(capture(&"unsupported shape").is_err());
        assert!(capture(&vec![f64::INFINITY]).is_err());
        let mut raw = serde_json::to_value(base).unwrap();
        raw["transform"]["x"]["z"] = serde_json::json!(1e-30);
        let tiny: Plane = serde_json::from_value(raw).unwrap();
        assert_eq!(
            plane_coefficients(&tiny).unwrap()[3][2].to_bits(),
            1e-30f64.to_bits()
        );
        assert!(!same_surface(&Surface::Plane(base), &Surface::Plane(tiny)).unwrap());
    }
}
