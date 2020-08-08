// GPLv3 License

use super::{GraphicalState, StateDataLoc, StateDataType, StateOperation};
use crate::Point;
use euclid::default::Point2D;
use itertools::Itertools;
use ordered_float::NotNan;
use rayon::prelude::*;
use smallvec::SmallVec;
use std::mem;

impl GraphicalState {
    /// Unselect all items
    #[inline]
    pub fn unselect(&mut self) {
        self.selected.clear();
        self.last_history_selected.take();
    }

    /// Delete all selected items.
    #[inline]
    pub fn delete_selected(&mut self) {
        self.history.clear();
        self.last_history_selected.take();

        let mut sel = &mut self.selected;
        let items: SmallVec<[StateDataLoc; 12]> = sel.drain(..).sorted().collect();
        mem::drop(sel);
        items.into_iter().for_each(|StateDataLoc(ty, i)| {
            ty.assoc_collection_mut(self).remove(i); // since we're going backwards, there shouldn't be
                                                     // many adverse side effects
        });
    }

    /// Push an item from the history into the selected item.
    #[inline]
    pub fn select_from_history(&mut self) {
        let mut lhs = &mut self.last_history_selected;
        let new_index = if let Some(mut i) = lhs.take() {
            *lhs = Some(i.saturating_sub(1));
            i
        } else {
            let index = self.history.len() - 1;
            *lhs = Some(index - 1);
            index
        };

        let data_loc = match self.history[new_index] {
            StateOperation::Add(sl) => sl.clone(),
        };

        let mut sel = &mut self.selected;
        if !sel.contains(&data_loc) {
            sel.push(data_loc);
        }
    }

    /// Select the element closest to a click location.
    pub fn select_closest_element<P: Point<f32> + Sync>(&mut self, loc: P) {
        // build a map of all of the lines and their associated data locations
        // TODO: maybe cache this?
        let point_map: Vec<(Point2D<f32>, usize, StateDataType)> = self
            .iter_data_objects()
            .flat_map(|(i, d)| {
                d.points()
                    .into_iter()
                    .map(move |pt| (pt.into_euclid(), i, d.data_type()))
            })
            .collect();

        if point_map.len() > 0 {
            let mut sel = &mut self.selected;

            let (_, index, item) = if let Some(i) = point_map
                .par_iter()
                .map(|(pt, i, d)| (pt.distance_to(&loc), i, d))
                .filter(|(dist, i, d)| !sel.contains(&StateDataLoc(**d, **i)) && !dist.is_nan())
                .min_by(|(dist1, _i1, _d1), (dist2, _i2, _d2)| {
                    NotNan::new(*dist1)
                        .unwrap()
                        .cmp(&NotNan::new(*dist2).unwrap())
                }) {
                i
            } else {
                println!("No minimum identified");
                return;
            };

            println!("Found index {:?} and item of type {:?}", index, item,);

            sel.push(StateDataLoc(*item, *index));
        }
    }
}
