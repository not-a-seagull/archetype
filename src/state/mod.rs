// GPL v3.0

mod data;
mod operations;

use data::*;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::{
    collections::HashMap,
    sync::atomic::{AtomicUsize, Ordering},
};

pub use data::*;

/// The current graphical state.
#[derive(Serialize, Deserialize)]
pub struct GraphicalState {
    curves: HashMap<DataID, Curve>,
    buffered_lines: SmallVec<[BufferedLine; 12]>,
    lines: HashMap<DataID, StateLine>,
    polygons: HashMap<DataID, Polyshape>,
    filled_polygons: HashMap<DataID, Polyshape>,
    history: Vec<StateOperation>,

    next_data_id: AtomicUsize,
    selected: Vec<StateDataLoc>,
    last_history_selected: Option<DataID>,
}

impl GraphicalState {
    #[inline]
    pub fn new() -> Self {
        Self {
            curves: HashMap::new(),
            buffered_lines: SmallVec::new(),
            lines: HashMap::new(),
            polygons: HashMap::new(),
            filled_polygons: HashMap::new(),
            history: Vec::new(),
            selected: Vec::new(),
            last_history_selected: None,
            next_data_id: AtomicUsize::new(0),
        }
    }

    #[inline]
    pub fn curves(&self) -> &HashMap<DataID, Curve> {
        &self.curves
    }

    #[inline]
    pub fn curves_mut(&mut self) -> &mut HashMap<DataID, Curve> {
        &mut self.curves
    }

    #[inline]
    pub fn lines(&self) -> &HashMap<DataID, StateLine> {
        &self.lines
    }

    #[inline]
    pub fn lines_mut(&mut self) -> &mut HashMap<DataID, StateLine> {
        &mut self.lines
    }

    #[inline]
    pub fn polygons(&self) -> &HashMap<DataID, Polyshape> {
        &self.polygons
    }

    #[inline]
    pub fn polygons_mut(&mut self) -> &mut HashMap<DataID, Polyshape> {
        &mut self.polygons
    }

    #[inline]
    pub fn history(&self) -> &[StateOperation] {
        &self.history
    }

    /// Update the history to go below the history limit.
    #[inline]
    pub fn update_history_add(&mut self, kind: StateDataType, last_id: usize, item_num: usize) {
        if item_num == 0 {
            return;
        }

        let new_index = kind.assoc_collection(self).length();
        self.history.par_extend(
            (last_id..last_id + item_num)
                .into_par_iter()
                .map(|i| StateOperation::Add(StateDataLoc(kind, i))),
        );

        while self.history.len() > HISTORY_LIMIT {
            self.history.remove(0);
        }
    }

    /// Get an iterator over all data objects (except for buffered lines).
    pub fn iter_data_objects(
        &self,
    ) -> impl Iterator<Item = (usize, &(dyn DataObject + Sync + 'static))> {
        self.polygons
            .iter()
            .map(|(i, d)| (*i, d as _))
            .chain(self.curves.iter().map(|(i, d)| (*i, d as _)))
            .chain(self.lines.iter().map(|(i, d)| (*i, d as _)))
    }

    /// Get the next iteration of the data ID.
    pub fn next_data_id(&self) -> DataID {
        self.next_data_id.fetch_add(1, Ordering::SeqCst)
    }

    /// Generate a set number of data ID's.
    pub fn next_data_ids(&self, count: usize) -> SmallVec<[DataID; 10]> {
        (0..count)
            .into_iter()
            .map(|_i| self.next_data_id())
            .collect()
    }

    /// Get the current iteration of the Data ID.
    pub fn current_data_id(&self) -> DataID {
        self.next_data_id.load(Ordering::SeqCst)
    }
}
