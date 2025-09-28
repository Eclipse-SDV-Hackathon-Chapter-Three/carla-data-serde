use carla::sensor::data::{Color, Image as ImageEvent};
use ndarray::{Array2, ArrayView1, ArrayView2};
use serde::{Deserialize, Serialize};
use std::fmt;

const PREVIEW_W: usize = 3;
const PREVIEW_H: usize = 3;

/// Remote schema for the foreign element type
#[derive(Debug, Serialize, Deserialize)]
#[serde(remote = "carla::sensor::data::Color")]
struct ColorRemote {
    b: u8,
    g: u8,
    r: u8,
    a: u8,
}

// ------------------------ Borrowed serializer ------------------------

mod arrayview2_color_remote {
    use super::*;
    use serde::Serialize;
    use serde::ser::{SerializeSeq, Serializer};

    struct ColorAsRemote<'a>(&'a Color);
    impl<'a> Serialize for ColorAsRemote<'a> {
        fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            super::ColorRemote::serialize(self.0, s)
        }
    }

    struct Row<'a>(ndarray::ArrayView1<'a, Color>);
    impl<'a> Serialize for Row<'a> {
        fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            let mut inner = s.serialize_seq(Some(self.0.len()))?;
            for c in self.0.iter() {
                inner.serialize_element(&ColorAsRemote(c))?;
            }
            inner.end()
        }
    }

    pub fn serialize<S: Serializer>(arr: &ArrayView2<Color>, s: S) -> Result<S::Ok, S::Error> {
        let (h, _) = arr.dim();
        let mut outer = s.serialize_seq(Some(h))?;
        for row in arr.rows() {
            outer.serialize_element(&Row(row))?;
        }
        outer.end()
    }
}

/// Borrowed, zero-copy serializer for Image
#[derive(Serialize)]
pub struct ImageEventSerBorrowed<'a> {
    pub height: usize,
    pub width: usize,
    pub len: usize,
    pub is_empty: bool,
    pub fov_angle: f32,
    #[serde(with = "self::arrayview2_color_remote")]
    pub array: ArrayView2<'a, Color>,
}

impl<'a> From<&'a ImageEvent> for ImageEventSerBorrowed<'a> {
    fn from(value: &'a ImageEvent) -> Self {
        Self {
            height: value.height(),
            width: value.width(),
            len: value.len(),
            is_empty: value.is_empty(),
            fov_angle: value.fov_angle(),
            array: value.as_array(), // borrow, zero-copy
        }
    }
}

// ------------------------ Owned, round-trip ------------------------

mod array2_color_remote {
    use super::*;
    use serde::de::{self, SeqAccess, Visitor};
    use serde::ser::SerializeSeq;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::fmt;

    struct ColorAsRemote<'a>(&'a Color);
    impl<'a> Serialize for ColorAsRemote<'a> {
        fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            super::ColorRemote::serialize(self.0, s)
        }
    }

    struct ColorFromRemote(Color);
    impl<'de> Deserialize<'de> for ColorFromRemote {
        fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
            super::ColorRemote::deserialize(d).map(ColorFromRemote)
        }
    }

    struct Row<'a>(ndarray::ArrayView1<'a, Color>);
    impl<'a> Serialize for Row<'a> {
        fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            let mut inner = s.serialize_seq(Some(self.0.len()))?;
            for c in self.0.iter() {
                inner.serialize_element(&ColorAsRemote(c))?;
            }
            inner.end()
        }
    }

    pub fn serialize<S: Serializer>(arr: &Array2<Color>, s: S) -> Result<S::Ok, S::Error> {
        let (h, _) = arr.dim();
        let mut outer = s.serialize_seq(Some(h))?;
        for row in arr.rows() {
            outer.serialize_element(&Row(row))?;
        }
        outer.end()
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Array2<Color>, D::Error> {
        struct Outer;
        impl<'de> Visitor<'de> for Outer {
            type Value = Array2<Color>;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "Vec<Vec<Color>> with equal-length rows")
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut outer: A) -> Result<Self::Value, A::Error> {
                let mut rows: Vec<Vec<Color>> = Vec::new();
                while let Some(inner) = outer.next_element::<Vec<ColorFromRemote>>()? {
                    rows.push(inner.into_iter().map(|x| x.0).collect());
                }
                let h = rows.len();
                let w = rows.get(0).map_or(0, |r| r.len());
                if w == 0 && h == 0 {
                    return Ok(Array2::from_shape_vec((0, 0), vec![]).unwrap());
                }
                for r in &rows {
                    if r.len() != w {
                        return Err(de::Error::custom("ragged 2D array"));
                    }
                }
                let flat: Vec<Color> = rows.into_iter().flatten().collect();
                ndarray::Array2::from_shape_vec((h, w), flat).map_err(de::Error::custom)
            }
        }
        d.deserialize_seq(Outer)
    }
}

/// Owned, round-trip serializer for Image
#[derive(Serialize, Deserialize)]
pub struct ImageEventSerDe {
    pub height: usize,
    pub width: usize,
    pub len: usize,
    pub is_empty: bool,
    pub fov_angle: f32,
    #[serde(with = "self::array2_color_remote")]
    pub array: Array2<Color>,
}

impl From<ImageEvent> for ImageEventSerDe {
    fn from(value: ImageEvent) -> Self {
        let view = value.as_array();
        let array: Array2<Color> = view.map(|c| Color {
            b: c.b,
            g: c.g,
            r: c.r,
            a: c.a,
        });

        Self {
            height: value.height(),
            width: value.width(),
            len: value.len(),
            is_empty: value.is_empty(),
            fov_angle: value.fov_angle(),
            array,
        }
    }
}

// ---------------------------------------------------------------------
// helpers: write full / preview matrices to the formatter (no allocs)
// ---------------------------------------------------------------------

fn write_full_matrix<'a, A: 'a>(
    f: &mut fmt::Formatter<'_>,
    rows: impl IntoIterator<Item = ArrayView1<'a, A>>,
    mut write_px: impl FnMut(&A, &mut fmt::Formatter<'_>) -> fmt::Result,
) -> fmt::Result {
    writeln!(f, "[")?;
    for row in rows.into_iter() {
        write!(f, "  [")?;
        for (i, px) in row.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write_px(px, f)?;
        }
        writeln!(f, "],")?;
    }
    write!(f, "]")
}

fn write_preview_matrix<'a, A: 'a>(
    f: &mut fmt::Formatter<'_>,
    rows: impl IntoIterator<Item = ArrayView1<'a, A>>,
    total_rows: usize,
    max_h: usize,
    max_w: usize,
    mut write_px: impl FnMut(&A, &mut fmt::Formatter<'_>) -> fmt::Result,
    mut row_len: impl FnMut(&ArrayView1<'a, A>) -> usize,
) -> fmt::Result {
    writeln!(f, "[")?;
    let mut rcount = 0usize;
    for row in rows.into_iter() {
        if rcount >= max_h {
            break;
        }
        rcount += 1;

        write!(f, "  [")?;
        let mut i = 0usize;
        for px in row.iter() {
            if i >= max_w {
                break;
            }
            if i > 0 {
                write!(f, ", ")?;
            }
            write_px(px, f)?;
            i += 1;
        }
        if row_len(&row) > max_w {
            write!(f, ", …")?;
        }
        writeln!(f, "],")?;
    }
    if total_rows > rcount {
        write!(f, "  …\n")?;
    }
    write!(f, "]")
}

// pretty RGBA printer (RGB order first for humans)
#[inline]
fn write_rgba(px: &Color, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "({}, {}, {}, {})", px.r, px.g, px.b, px.a)
}

// ------------------------ Custom Debug impls ------------------------

impl<'a> fmt::Debug for ImageEventSerBorrowed<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (h, w) = self.array.dim();

        let mut ds = f.debug_struct("ImageEventSerBorrowed");
        ds.field("height", &self.height)
            .field("width", &self.width)
            .field("len", &self.len)
            .field("is_empty", &self.is_empty)
            .field("fov_angle", &self.fov_angle);
        ds.finish_non_exhaustive()?;

        write!(f, "\narray ")?;
        if f.alternate() {
            write!(f, "(full {}x{}) = ", h, w)?;
            write_full_matrix(f, self.array.rows(), |c, fmtr| write_rgba(c, fmtr))
        } else {
            write!(
                f,
                "(preview {}x{}, showing {}x{}) = ",
                h,
                w,
                PREVIEW_H.min(h),
                PREVIEW_W.min(w)
            )?;
            write_preview_matrix(
                f,
                self.array.rows(),
                h,
                PREVIEW_H,
                PREVIEW_W,
                |c, fmtr| write_rgba(c, fmtr),
                |row: &ArrayView1<'_, Color>| row.len(),
            )
        }
    }
}

impl fmt::Debug for ImageEventSerDe {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (h, w) = self.array.dim();

        let mut ds = f.debug_struct("ImageEventSerDe");
        ds.field("height", &self.height)
            .field("width", &self.width)
            .field("len", &self.len)
            .field("is_empty", &self.is_empty)
            .field("fov_angle", &self.fov_angle);
        ds.finish_non_exhaustive()?;

        write!(f, "\narray ")?;
        if f.alternate() {
            write!(f, "(full {}x{}) = ", h, w)?;
            write_full_matrix(f, self.array.rows(), |c, fmtr| write_rgba(c, fmtr))
        } else {
            write!(
                f,
                "(preview {}x{}, showing {}x{}) = ",
                h,
                w,
                PREVIEW_H.min(h),
                PREVIEW_W.min(w)
            )?;
            write_preview_matrix(
                f,
                self.array.rows(),
                h,
                PREVIEW_H.min(h),
                PREVIEW_W.min(w),
                |c, fmtr| write_rgba(c, fmtr),
                |row: &ArrayView1<'_, Color>| row.len(),
            )
        }
    }
}
