use carla::sensor::data::{
    RadarDetection as CarlaRadarDetection, RadarMeasurement as RadarMeasurementEvent,
};
use serde::{Deserialize, Serialize};
use std::fmt;

// How many detections to show in non-alternate ({:?}) mode
const PREVIEW_DETECTIONS: usize = 5;

/// Remote schema for the foreign element type
#[derive(Debug, Serialize, Deserialize)]
#[serde(remote = "carla::sensor::data::RadarDetection")]
pub struct RadarDetectionRemote {
    pub velocity: f32,
    pub azimuth: f32,
    pub altitude: f32,
    pub depth: f32,
}

// -------------------- &[RadarDetection] (serialize-only) --------------------
mod slice_radar_detection_remote {
    use super::*;
    use serde::ser::{SerializeSeq, Serializer};

    struct AsRemote<'a>(&'a CarlaRadarDetection);
    impl<'a> Serialize for AsRemote<'a> {
        fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            super::RadarDetectionRemote::serialize(self.0, s)
        }
    }

    pub fn serialize<S: Serializer>(
        slice: &[CarlaRadarDetection],
        s: S,
    ) -> Result<S::Ok, S::Error> {
        let mut seq = s.serialize_seq(Some(slice.len()))?;
        for d in slice {
            seq.serialize_element(&AsRemote(d))?;
        }
        seq.end()
    }
}

/// Borrowed, zero-copy serializer
#[derive(Serialize)]
pub struct RadarMeasurementSerBorrowed<'a> {
    pub detection_amount: usize,
    #[serde(with = "self::slice_radar_detection_remote")]
    pub detections: &'a [CarlaRadarDetection],
    pub len: usize,
    pub is_empty: bool,
}

impl<'a> From<&'a RadarMeasurementEvent> for RadarMeasurementSerBorrowed<'a> {
    fn from(m: &'a RadarMeasurementEvent) -> Self {
        Self {
            detection_amount: m.detection_amount(),
            detections: m.as_slice(),
            len: m.len(),
            is_empty: m.is_empty(),
        }
    }
}

// -------------------- Vec<RadarDetection> (round-trip) --------------------
mod vec_radar_detection_remote {
    use super::*;
    use serde::de::{SeqAccess, Visitor};
    use serde::ser::SerializeSeq;
    use serde::{Deserializer, Serializer};
    use std::fmt;

    struct AsRemote<'a>(&'a CarlaRadarDetection);
    impl<'a> Serialize for AsRemote<'a> {
        fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            super::RadarDetectionRemote::serialize(self.0, s)
        }
    }

    struct FromRemote(CarlaRadarDetection);
    impl<'de> Deserialize<'de> for FromRemote {
        fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
            super::RadarDetectionRemote::deserialize(d).map(FromRemote)
        }
    }

    pub fn serialize<S: Serializer>(v: &Vec<CarlaRadarDetection>, s: S) -> Result<S::Ok, S::Error> {
        let mut seq = s.serialize_seq(Some(v.len()))?;
        for d in v {
            seq.serialize_element(&AsRemote(d))?;
        }
        seq.end()
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        d: D,
    ) -> Result<Vec<CarlaRadarDetection>, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = Vec<CarlaRadarDetection>;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "Vec<RadarDetection>")
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut out = Vec::with_capacity(seq.size_hint().unwrap_or(0));
                while let Some(FromRemote(x)) = seq.next_element::<FromRemote>()? {
                    out.push(x); // <-- x is CarlaRadarDetection; no `.0`
                }
                Ok(out)
            }
        }
        d.deserialize_seq(V)
    }
}

#[derive(Serialize, Deserialize)]
pub struct RadarMeasurementSerDe {
    pub detection_amount: usize,
    #[serde(with = "self::vec_radar_detection_remote")]
    pub detections: Vec<CarlaRadarDetection>,
    pub len: usize,
    pub is_empty: bool,
}

// ======================= Debug helpers (no allocations) =======================

#[inline]
fn write_radar_detection(d: &CarlaRadarDetection, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(
        f,
        "{{ velocity: {}, azimuth: {}, altitude: {}, depth: {} }}",
        d.velocity, d.azimuth, d.altitude, d.depth
    )
}

fn write_radar_seq_full<'a>(
    f: &mut fmt::Formatter<'_>,
    detections: impl IntoIterator<Item = &'a CarlaRadarDetection>,
) -> fmt::Result {
    writeln!(f, "[")?;
    for d in detections {
        write!(f, "  ")?;
        write_radar_detection(d, f)?;
        writeln!(f, ",")?;
    }
    write!(f, "]")
}

fn write_radar_seq_preview<'a>(
    f: &mut fmt::Formatter<'_>,
    detections: impl IntoIterator<Item = &'a CarlaRadarDetection>,
    total: usize,
    max_show: usize,
) -> fmt::Result {
    let max_show = max_show.min(total);
    writeln!(f, "[")?;

    let mut shown = 0usize;
    for d in detections.into_iter().take(max_show) {
        write!(f, "  ")?;
        write_radar_detection(d, f)?;
        writeln!(f, ",")?;
        shown += 1;
    }

    if total > shown {
        writeln!(f, "  … ({} more)", total - shown)?;
    }

    write!(f, "]")
}

// ======================= Custom Debug impls =======================

impl<'a> fmt::Debug for RadarMeasurementSerBorrowed<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut ds = f.debug_struct("RadarMeasurementSerBorrowed");
        ds.field("detection_amount", &self.detection_amount)
            .field("len", &self.len)
            .field("is_empty", &self.is_empty);
        ds.finish_non_exhaustive()?; // header

        write!(f, "\ndetections ")?;
        if f.alternate() {
            write!(f, "(full, {} total) = ", self.len)?;
            write_radar_seq_full(f, self.detections.iter())
        } else {
            write!(
                f,
                "(preview showing {} of {}) = ",
                PREVIEW_DETECTIONS.min(self.len),
                self.len
            )?;
            write_radar_seq_preview(f, self.detections.iter(), self.len, PREVIEW_DETECTIONS)
        }
    }
}

impl fmt::Debug for RadarMeasurementSerDe {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut ds = f.debug_struct("RadarMeasurementSerDe");
        ds.field("detection_amount", &self.detection_amount)
            .field("len", &self.len)
            .field("is_empty", &self.is_empty);
        ds.finish_non_exhaustive()?; // header

        write!(f, "\ndetections ")?;
        if f.alternate() {
            write!(f, "(full, {} total) = ", self.len)?;
            write_radar_seq_full(f, self.detections.iter())
        } else {
            write!(
                f,
                "(preview showing {} of {}) = ",
                PREVIEW_DETECTIONS.min(self.len),
                self.len
            )?;
            write_radar_seq_preview(f, self.detections.iter(), self.len, PREVIEW_DETECTIONS)
        }
    }
}
