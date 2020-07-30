// GPLv3 License

use crate::{BezierCurve, Polygon};
use euclid::default::Point2D;
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Serialize, Deserialize)]
pub enum StateDataType {
    Curve,
    Line,
    BufferedLine,
    Polygon,
}

#[derive(Copy, Clone, Serialize, Deserialize)]
pub struct StateDataLoc(StateDataType, usize);

// repr of a line
#[derive(Serialize, Deserialize)]
pub struct StateLine {
    pub points: [Point2D<f32>; 2],
    pub brush: usize,
}

// repr of a buffered line
#[repr(transparent)]
#[derive(Serialize, Deserialize)]
pub struct BufferedLine(pub [Point2D<f32>; 2]);

// repr of a bezier curve
#[derive(Serialize, Deserialize)]
pub struct Curve {
    pub curve: BezierCurve,
    pub brush: usize,
}

// repr of a polygon
#[derive(Serialize, Deserialize)]
pub struct Polyshape {
    pub polygon: Polygon,
    pub brush: usize,
}

pub const HISTORY_LIMIT: usize = 45;

/// A trait unifying every object that can be selected.
pub trait DataObject {
    fn data_type(&self) -> StateDataType;
}

impl DataObject for StateLine {
    #[inline]
    fn data_type(&self) -> StateDataType {
        StateDataType::Line
    }
}

impl DataObject for BufferedLine {
    #[inline]
    fn data_type(&self) -> StateDataType {
        StateDataType::BufferedLine
    }
}

impl DataObject for Curve {
    #[inline]
    fn data_type(&self) -> StateDataType {
        StateDataType::Curve
    }
}

impl DataObject for Polyshape {
    #[inline]
    fn data_type(&self) -> StateDataType {
        StateDataType::Polygon
    }
}

/// A collection of selectable data objects.
pub trait DataObjectCollection {
    fn data_at(&self, index: usize) -> Option<&dyn DataObject>;
}

impl<T: DataObject> DataObjectCollection for Vec<T> {
    #[inline]
    fn data_at(&self, index: usize) -> Option<&dyn DataObject> {
        match self.get(index) {
            Some(r) => Some(r),
            None => None,
        }
    }
}
