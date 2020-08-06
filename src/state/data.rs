// GPLv3 License

use super::GraphicalState as State;
use crate::{BezierCurve, Brush, Line, Polygon};
use euclid::default::Point2D;
use pathfinder_geometry::vector::Vector2F;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::{
    any::{Any, TypeId},
    boxed::Box,
    iter,
};

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

    #[inline]
    pub fn take_item(self, state: &mut State) -> DataObjectContainer {
        self.0.assoc_collection_mut(state).remove(self.1)
    }
}

// repr of a line
#[derive(Clone, Serialize, Deserialize)]
pub struct StateLine {
    pub points: [Point2D<f32>; 2],
    pub brush: usize,
}

// repr of a buffered line
#[repr(transparent)]
#[derive(Clone, Serialize, Deserialize)]
pub struct BufferedLine(pub [Point2D<f32>; 2]);

// repr of a bezier curve
#[derive(Clone, Serialize, Deserialize)]
pub struct Curve {
    pub curve: BezierCurve,
    pub brush: usize,
}

// repr of a polygon
#[derive(Clone, Serialize, Deserialize)]
pub struct Polyshape {
    pub polygon: Polygon,
    pub brush: usize,
}

#[inline]
fn line_to_points<Ln: Line<f32>>(line: &Ln) -> SmallVec<[Vector2F; 4]> {
    const POINT_SKIP: usize = 8;
    let line_iter = imageproc::drawing::BresenhamLineIter::new(
        (line.from_x(), line.from_y()),
        (line.to_x(), line.to_y()),
    );
    line_iter
        .step_by(POINT_SKIP)
        .map(|(x, y)| Vector2F::new(x as f32, y as f32))
        .collect()
}

pub const HISTORY_LIMIT: usize = 45;

/// A trait unifying every object that can be selected.
pub trait DataObject {
    fn data_type(&self) -> StateDataType;
    fn points(&self) -> SmallVec<[Vector2F; 4]>;
    fn into_container(self) -> DataObjectContainer;
    fn clone_into_container(&self) -> DataObjectContainer;
}

impl DataObject for StateLine {
    #[inline]
    fn data_type(&self) -> StateDataType {
        StateDataType::Line
    }

    #[inline]
    fn points(&self) -> SmallVec<[Vector2F; 4]> {
        line_to_points(&self.points)
    }

    #[inline]
    fn into_container(self) -> DataObjectContainer {
        DataObjectContainer::StateLine(self)
    }

    #[inline]
    fn clone_into_container(&self) -> DataObjectContainer {
        self.clone().into_container()
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

    #[inline]
    fn into_container(self) -> DataObjectContainer {
        DataObjectContainer::BufferedLine(self)
    }

    #[inline]
    fn clone_into_container(&self) -> DataObjectContainer {
        self.clone().into_container()
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
            .flat_map(|l| line_to_points(&l))
            .collect()
    }

    #[inline]
    fn into_container(self) -> DataObjectContainer {
        DataObjectContainer::Curve(self)
    }

    #[inline]
    fn clone_into_container(&self) -> DataObjectContainer {
        self.clone().into_container()
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
            .flat_map(|l| line_to_points(&l))
            .collect()
    }

    #[inline]
    fn into_container(self) -> DataObjectContainer {
        DataObjectContainer::Polyshape(self)
    }

    #[inline]
    fn clone_into_container(&self) -> DataObjectContainer {
        self.clone().into_container()
    }
}

/// Stack-based container for select data objects.
#[derive(Clone, Serialize, Deserialize)]
pub enum DataObjectContainer {
    Curve(Curve),
    StateLine(StateLine),
    BufferedLine(BufferedLine),
    Polyshape(Polyshape),
}

impl DataObjectContainer {
    #[inline]
    pub fn into_boxed_any(self) -> Box<dyn Any + 'static> {
        match self {
            Self::Curve(c) => Box::new(c),
            Self::StateLine(s) => Box::new(s),
            Self::BufferedLine(bl) => Box::new(bl),
            Self::Polyshape(p) => Box::new(p),
        }
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
    fn remove(&mut self, index: usize) -> DataObjectContainer;
    fn insert(&mut self, index: usize, item: DataObjectContainer);
}

impl<T: DataObject + 'static> DataObjectCollection for Vec<T> {
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

    #[inline]
    fn remove(&mut self, index: usize) -> DataObjectContainer {
        self.remove(index).into_container()
    }

    #[inline]
    fn insert(&mut self, index: usize, item: DataObjectContainer) {
        match item.into_boxed_any().downcast::<T>() {
            Ok(b) => self.insert(index, *b),
            _ => panic!("Attempted to insert invalid object into data object collection"),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum StateOperation {
    Add(StateDataLoc),
}
