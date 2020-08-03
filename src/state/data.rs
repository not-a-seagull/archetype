// GPLv3 License

use super::GraphicalState as State;
use crate::{BezierCurve, Brush, Polygon};
use euclid::default::Point2D;
use pathfinder_geometry::vector::Vector2F;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::iter;

#[derive(Debug, PartialEq, Eq, Copy, Clone, Serialize, Deserialize, PartialOrd, Ord, Hash)]
pub enum StateDataType {
    Curve,
    Line,
    BufferedLine,
    Polygon,
}

impl StateDataType {
    #[inline]
    pub fn assoc_collection(self, state: &State) -> &dyn DataObjectCollection {
        match self {
            Self::Curve => state.curves(),
            Self::Line => state.lines(),
            Self::BufferedLine => state.buffered_lines(),
            Self::Polygon => state.polygons(),
        }
    }

    #[inline]
    pub fn assoc_collection_mut(self, state: &mut State) -> &mut dyn DataObjectCollection {
        match self {
            Self::Curve => state.curves_mut(),
            Self::Line => state.lines_mut(),
            Self::BufferedLine => state.buffered_lines_mut(),
            Self::Polygon => state.polygons_mut(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Copy, Clone, Serialize, Deserialize, PartialOrd, Ord, Hash)]
pub struct StateDataLoc(pub StateDataType, pub usize);

impl StateDataLoc {
    #[inline]
    pub fn item(self, state: &State) -> &dyn DataObject {
        self.0
            .assoc_collection(state)
            .data_at(self.1)
            .expect("Data location refers to data that does not exist")
    }

    #[inline]
    pub fn item_mut(self, state: &mut State) -> &mut dyn DataObject {
        self.0
            .assoc_collection_mut(state)
            .data_at_mut(self.1)
            .expect("Data location refers to data that does not exist")
    }
}

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
    fn points(&self) -> SmallVec<[Vector2F; 4]>;
}

impl DataObject for StateLine {
    #[inline]
    fn data_type(&self) -> StateDataType {
        StateDataType::Line
    }

    #[inline]
    fn points(&self) -> SmallVec<[Vector2F; 4]> {
        self.points
            .iter()
            .map(|Point2D { x, y, .. }| Vector2F::new(*x, *y))
            .collect()
    }
}

impl DataObject for BufferedLine {
    #[inline]
    fn data_type(&self) -> StateDataType {
        StateDataType::BufferedLine
    }

    #[inline]
    fn points(&self) -> SmallVec<[Vector2F; 4]> {
        unimplemented!()
    }
}

impl DataObject for Curve {
    #[inline]
    fn data_type(&self) -> StateDataType {
        StateDataType::Curve
    }

    #[inline]
    fn points(&self) -> SmallVec<[Vector2F; 4]> {
        self.curve
            .edges()
            .flat_map(|l| iter::once(l.from()).chain(iter::once(l.to())))
            .collect()
    }
}

impl DataObject for Polyshape {
    #[inline]
    fn data_type(&self) -> StateDataType {
        StateDataType::Polygon
    }

    #[inline]
    fn points(&self) -> SmallVec<[Vector2F; 4]> {
        self.polygon
            .as_straight_edges()
            .flat_map(|l| iter::once(l.from()).chain(iter::once(l.to())))
            .collect()
    }
}

/// A collection of selectable data objects.
pub trait DataObjectCollection {
    #[inline]
    fn kind(&self) -> StateDataType {
        self.data_at(0).unwrap().data_type()
    }
    fn data_at(&self, index: usize) -> Option<&dyn DataObject>;
    fn data_at_mut(&mut self, index: usize) -> Option<&mut dyn DataObject>;
    fn length(&self) -> usize;
}

impl<T: DataObject> DataObjectCollection for Vec<T> {
    #[inline]
    fn length(&self) -> usize {
        self.len()
    }

    #[inline]
    fn data_at(&self, index: usize) -> Option<&dyn DataObject> {
        match self.get(index) {
            Some(r) => Some(r),
            None => None,
        }
    }

    #[inline]
    fn data_at_mut(&mut self, index: usize) -> Option<&mut dyn DataObject> {
        match self.get_mut(index) {
            Some(r) => Some(r),
            None => None,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum StateOperation {
    Add(StateDataLoc),
}
