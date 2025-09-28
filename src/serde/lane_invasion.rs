use carla::sensor::data::LaneInvasionEvent;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Serialize, Deserialize)]
#[serde(remote = "carla::road::element::LaneMarking_Type")]
pub enum LaneMarkingTypeSerDe {
    Other = 0,
    Broken = 1,
    Solid = 2,
    SolidSolid = 3,
    SolidBroken = 4,
    BrokenSolid = 5,
    BrokenBroken = 6,
    BottsDots = 7,
    Grass = 8,
    Curb = 9,
    None = 10,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(remote = "carla::road::element::LaneMarking_Color")]
pub enum LaneMarkingColorSerDe {
    Standard = 0,
    Blue = 1,
    Green = 2,
    Red = 3,
    Yellow = 4,
    Other = 5,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(remote = "carla::road::element::LaneMarking_LaneChange")]
pub enum LaneMarkingLaneChangeSerDe {
    None = 0,
    Right = 1,
    Left = 2,
    Both = 3,
}

#[derive(Serialize, Deserialize)]
pub struct LaneMarkingSerDe {
    #[serde(with = "LaneMarkingTypeSerDe")]
    pub marking_type: carla::road::element::LaneMarking_Type,

    #[serde(with = "LaneMarkingColorSerDe")]
    pub marking_color: carla::road::element::LaneMarking_Color,

    #[serde(with = "LaneMarkingLaneChangeSerDe")]
    pub lane_change: carla::road::element::LaneMarking_LaneChange,

    pub width: f64,
}

#[derive(Serialize, Deserialize)]
pub struct LaneInvasionEventSerDe {
    pub crossed_lane_markings: Vec<LaneMarkingSerDe>,
}

impl From<LaneInvasionEvent> for LaneInvasionEventSerDe {
    fn from(value: LaneInvasionEvent) -> Self {
        let mut crossed_lane_markings: Vec<LaneMarkingSerDe> = Vec::new();
        for clm in value.crossed_lane_markings() {
            let lane_marking_serde = LaneMarkingSerDe {
                marking_type: clm.type_(),
                marking_color: clm.color(),
                lane_change: clm.lane_change(),
                width: clm.width(),
            };
            crossed_lane_markings.push(lane_marking_serde);
        }

        LaneInvasionEventSerDe {
            crossed_lane_markings,
        }
    }
}

// ---------- enum conversions ----------
impl From<carla::road::element::LaneMarking_Type> for LaneMarkingTypeSerDe {
    fn from(v: carla::road::element::LaneMarking_Type) -> Self {
        use carla::road::element::LaneMarking_Type as F;
        match v {
            F::Other => Self::Other,
            F::Broken => Self::Broken,
            F::Solid => Self::Solid,
            F::SolidSolid => Self::SolidSolid,
            F::SolidBroken => Self::SolidBroken,
            F::BrokenSolid => Self::BrokenSolid,
            F::BrokenBroken => Self::BrokenBroken,
            F::BottsDots => Self::BottsDots,
            F::Grass => Self::Grass,
            F::Curb => Self::Curb,
            F::None => Self::None,
        }
    }
}

impl From<carla::road::element::LaneMarking_Color> for LaneMarkingColorSerDe {
    fn from(v: carla::road::element::LaneMarking_Color) -> Self {
        use carla::road::element::LaneMarking_Color as F;
        match v {
            F::Standard => Self::Standard,
            F::Blue => Self::Blue,
            F::Green => Self::Green,
            F::Red => Self::Red,
            F::Yellow => Self::Yellow,
            F::Other => Self::Other,
        }
    }
}

impl From<carla::road::element::LaneMarking_LaneChange> for LaneMarkingLaneChangeSerDe {
    fn from(v: carla::road::element::LaneMarking_LaneChange) -> Self {
        use carla::road::element::LaneMarking_LaneChange as F;
        match v {
            F::None => Self::None,
            F::Right => Self::Right,
            F::Left => Self::Left,
            F::Both => Self::Both,
        }
    }
}

// ---------- custom Debug for your types ----------
impl fmt::Debug for LaneMarkingSerDe {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let marking_type = LaneMarkingTypeSerDe::from(self.marking_type.clone());
        let marking_color = LaneMarkingColorSerDe::from(self.marking_color.clone());
        let lane_change = LaneMarkingLaneChangeSerDe::from(self.lane_change.clone());

        f.debug_struct("LaneMarkingSerDe")
            .field("marking_type", &marking_type)
            .field("marking_color", &marking_color)
            .field("lane_change", &lane_change)
            .field("width", &self.width)
            .finish()
    }
}

impl fmt::Debug for LaneInvasionEventSerDe {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LaneInvasionEventSerDe")
            .field("crossed_lane_markings", &self.crossed_lane_markings)
            .finish()
    }
}
