use serde::{Deserialize, Serialize};
use crate::Vector3DSerDe;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct ImuMeasurementSerDe {
    pub accelerometer: Vector3DSerDe,
    pub gyroscope: Vector3DSerDe,
    pub compass: f32,
}

impl From<carla::sensor::data::ImuMeasurement> for ImuMeasurementSerDe {
    fn from(m: carla::sensor::data::ImuMeasurement) -> Self {
        Self {
            accelerometer: m.accelerometer().into(),
            gyroscope: m.gyroscope().into(),
            compass: m.compass(),
        }
    }
}

// (optional but handy) allow converting from a reference too
impl From<&carla::sensor::data::ImuMeasurement> for ImuMeasurementSerDe {
    fn from(m: &carla::sensor::data::ImuMeasurement) -> Self {
        Self {
            accelerometer: m.accelerometer().into(),
            gyroscope: m.gyroscope().into(),
            compass: m.compass(),
        }
    }
}
