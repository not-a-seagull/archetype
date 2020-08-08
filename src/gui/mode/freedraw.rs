// GPLv3 License

use super::GuiMode;
use crate::Gui;
use pathfinder_geometry::vector::Vector2F;
use std::collections::HashSet;

pub struct FreedrawGuiMode {
    is_drawing: bool,
    point_set: HashSet<Vector2F>,
}

impl FreedrawGuiMode {
    #[inline]
    pub fn new() -> Self {
        Self {
            is_drawing: false,
            point_set: HashSet::new(),
        }
    }
}

impl GuiMode for FreedrawGuiMode {
    #[inline]
    fn switch_in(&mut self, _gui: &Gui) {}

    #[inline]
    fn switch_out(&mut self, _gui: &Gui) {
        self.is_drawing = false;
    }

    #[inline]
    fn key_press(&mut self, _c: char, _gui: &Gui) {}
}
